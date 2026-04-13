<script lang="ts">
  import { getActiveTree, getTagTree, getTagPrereqs, listSkills, lightSkill, unlightSkill, listTags, createTagInline, listSkillTrees, adoptSkillTree, castVote, getFrontierSkills } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { authorName, tagName as resolveTagName } from '../lib/display';
  import type { TagTreeEntry, UserTagPrereq, Tag, SkillTree, FrontierSkill } from '../lib/types';

  // --- Tab state ---
  let activeTab = $state<'my' | 'community'>('my');

  // --- My Skills state ---
  let loading = $state(true);
  let prereqEdges = $state<UserTagPrereq[]>([]);
  let tree = $state<TagTreeEntry[]>([]);
  let tagNamesMap = $state<Record<string, string>>({});
  let tagNamesI18n = $state<Record<string, Record<string, string>>>({});
  let skillMap = $state(new Map<string, 'mastered' | 'learning'>());
  let masteredCount = $derived([...skillMap.values()].filter(s => s === 'mastered').length);
  let learningCount = $derived([...skillMap.values()].filter(s => s === 'learning').length);

  // Frontier skills (next to learn)
  let frontierSkills = $state<FrontierSkill[]>([]);

  // Expansion & selection
  let expandedGroups = $state(new Set<string>());
  let selectedNodeId = $state<string | null>(null);

  // Drag state: user position overrides (nodeId → {x, y} local to group)
  let dragOverrides = $state(new Map<string, { x: number; y: number }>());
  let dragging = $state<{ nodeId: string; rootId: string; startX: number; startY: number; origX: number; origY: number } | null>(null);

  // Tag search
  let allTags = $state<Tag[]>([]);
  let searchQuery = $state('');
  let searchSuggestions = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return allTags.filter(t => t.id.toLowerCase().includes(q) || t.name.toLowerCase().includes(q)).slice(0, 8);
  });

  // --- Hierarchy: parent-child grouping ---
  let childrenOf = $derived.by(() => {
    const map = new Map<string, string[]>();
    const hasParent = new Set<string>();
    for (const e of tree) {
      const arr = map.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      map.set(e.parent_tag, arr);
      hasParent.add(e.child_tag);
    }
    return { map, hasParent };
  });

  let roots = $derived(
    [...new Set(tree.map(e => e.parent_tag))].filter(p => !childrenOf.hasParent.has(p))
  );

  // Expand all roots by default
  $effect(() => {
    if (roots.length > 0 && expandedGroups.size === 0) {
      expandedGroups = new Set(roots);
    }
  });

  function resolveName(id: string): string {
    const i18n = tagNamesI18n[id];
    const name = tagNamesMap[id];
    return i18n ? resolveTagName(i18n, name || id, id) : (name || id);
  }

  function toggleGroup(id: string) {
    const s = new Set(expandedGroups);
    if (s.has(id)) s.delete(id); else s.add(id);
    expandedGroups = s;
  }

  // --- Collect DIRECT children that are leaf nodes (no further children) ---
  function collectDirectLeaves(groupId: string): Set<string> {
    const result = new Set<string>();
    for (const ch of (childrenOf.map.get(groupId) || [])) {
      if (!(childrenOf.map.get(ch) || []).length) {
        result.add(ch); // leaf
      }
    }
    return result;
  }

  // --- Collect ALL descendant leaves (for progress counting) ---
  function collectAllLeaves(groupId: string): Set<string> {
    const result = new Set<string>();
    const stack = [...(childrenOf.map.get(groupId) || [])];
    while (stack.length > 0) {
      const id = stack.pop()!;
      const children = childrenOf.map.get(id) || [];
      if (children.length === 0) {
        result.add(id);
      } else {
        for (const ch of children) stack.push(ch);
      }
    }
    return result;
  }

  // --- Get sub-groups (direct children that have their own children) ---
  function getSubGroups(groupId: string): string[] {
    return (childrenOf.map.get(groupId) || []).filter(ch => (childrenOf.map.get(ch) || []).length > 0);
  }

  // Legacy: collect all leaves for cross-group edge detection
  function collectLeaves(groupId: string): Set<string> {
    return collectAllLeaves(groupId);
  }

  // --- Layout computation per group ---
  const NODE_H = 52;
  const H_GAP = 24;
  const V_GAP = 56;

  // Estimate rendered width for a node label (CJK chars count as 2 units).
  function measureNodeWidth(name: string): number {
    let units = 0;
    for (const ch of name) {
      const cp = ch.codePointAt(0) ?? 0;
      // CJK Unified, CJK Extension A/B, Hiragana, Katakana, Hangul, full-width
      units += (cp >= 0x1100 && cp <= 0xFFEF) || (cp >= 0x20000 && cp <= 0x2FA1F) ? 2 : 1;
    }
    // icon(16) + gap(6) + text(units*7.5) + padding(20) + border(4)
    const estimated = 16 + 6 + units * 7.5 + 20 + 4;
    return Math.max(60, Math.min(180, Math.round(estimated)));
  }

  // NODE_W is now a per-node value; use measureNodeWidth() for layout.
  // For backwards-compat where a single width is needed (conn endpoints), we
  // look it up from the node's own width stored in LayoutNode.


  function getNodeStatus(tagId: string): 'locked' | 'available' | 'learning' | 'mastered' {
    const current = skillMap.get(tagId);
    if (current === 'mastered') return 'mastered';
    if (current === 'learning') return 'learning';
    const required = prereqEdges.filter(e => e.to_tag === tagId && e.prereq_type === 'required');
    if (required.length === 0) return 'available';
    return required.every(e => skillMap.get(e.from_tag) === 'mastered') ? 'available' : 'locked';
  }

  interface LayoutNode {
    id: string; name: string; x: number; y: number; w: number;
    status: 'locked' | 'available' | 'learning' | 'mastered';
    group: string | null; // parent group for visual labeling
  }
  interface LayoutConn {
    from: string; to: string; type: string;
    x1: number; y1: number; x2: number; y2: number;
  }
  interface GroupLayout {
    nodes: LayoutNode[];
    conns: LayoutConn[];
    w: number;
    h: number;
    progress: { total: number; mastered: number; learning: number };
  }

  // Find the immediate sub-group a node belongs to (direct child of root)
  function findSubGroup(nodeId: string, rootId: string): string | null {
    const directChildren = childrenOf.map.get(rootId) || [];
    for (const child of directChildren) {
      if (child === nodeId) return null; // direct child of root, no sub-group
      const descendants = collectLeaves(child);
      if (descendants.has(nodeId)) return child;
    }
    return null;
  }

  function computeLayout(rootId: string): GroupLayout {
    const fieldNodes = collectDirectLeaves(rootId);
    const allLeaves = collectAllLeaves(rootId);
    if (fieldNodes.size === 0 && getSubGroups(rootId).length === 0) return { nodes: [], conns: [], w: 0, h: 0, progress: { total: 0, mastered: 0, learning: 0 } };

    // Precompute per-node widths
    const nodeWidths = new Map<string, number>();
    for (const id of fieldNodes) nodeWidths.set(id, measureNodeWidth(resolveName(id)));

    // Filter prereq edges to within this group
    const localEdges = prereqEdges.filter(e => fieldNodes.has(e.from_tag) && fieldNodes.has(e.to_tag));

    // Topological sort (Kahn's)
    const inDeg = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const id of fieldNodes) { inDeg.set(id, 0); adj.set(id, []); }
    for (const e of localEdges) {
      adj.get(e.from_tag)!.push(e.to_tag);
      inDeg.set(e.to_tag, (inDeg.get(e.to_tag) || 0) + 1);
    }

    const byDepth: string[][] = [];
    let frontier = [...fieldNodes].filter(id => (inDeg.get(id) || 0) === 0);
    const placed = new Set<string>();

    while (frontier.length > 0) {
      // Group by sub-group within same level for visual coherence
      frontier.sort((a, b) => {
        const ga = findSubGroup(a, rootId) || '';
        const gb = findSubGroup(b, rootId) || '';
        if (ga !== gb) return ga.localeCompare(gb);
        return resolveName(a).localeCompare(resolveName(b));
      });
      byDepth.push(frontier);
      for (const id of frontier) placed.add(id);

      const next: string[] = [];
      for (const id of frontier) {
        for (const to of (adj.get(id) || [])) {
          const d = inDeg.get(to)! - 1;
          inDeg.set(to, d);
          if (d === 0 && !placed.has(to)) next.push(to);
        }
      }
      frontier = next;
    }

    const unplaced = [...fieldNodes].filter(id => !placed.has(id));
    if (unplaced.length > 0) byDepth.push(unplaced);

    // Position: each row uses adaptive widths; rows are centered within maxRowW
    const positions = new Map<string, { x: number; y: number }>();
    const rowWidths: number[] = byDepth.map(row => {
      return row.reduce((sum, id) => sum + (nodeWidths.get(id) ?? 136), 0) + Math.max(0, row.length - 1) * H_GAP;
    });
    const maxRowW = Math.max(0, ...rowWidths);

    for (let row = 0; row < byDepth.length; row++) {
      const nodes = byDepth[row];
      const rowW = rowWidths[row];
      const offsetX = (maxRowW - rowW) / 2;
      let curX = offsetX;
      for (const id of nodes) {
        positions.set(id, { x: curX, y: row * (NODE_H + V_GAP) });
        curX += (nodeWidths.get(id) ?? 136) + H_GAP;
      }
    }

    const nodes: LayoutNode[] = [];
    for (const [id, pos] of positions) {
      const w = nodeWidths.get(id) ?? 136;
      nodes.push({ id, name: resolveName(id), x: pos.x, y: pos.y, w, status: getNodeStatus(id), group: findSubGroup(id, rootId) });
    }

    const conns: LayoutConn[] = [];
    for (const e of localEdges) {
      const fp = positions.get(e.from_tag), tp = positions.get(e.to_tag);
      const fw = nodeWidths.get(e.from_tag) ?? 136;
      const tw = nodeWidths.get(e.to_tag) ?? 136;
      if (fp && tp) {
        conns.push({
          from: e.from_tag, to: e.to_tag, type: e.prereq_type,
          x1: fp.x + fw / 2, y1: fp.y + NODE_H,
          x2: tp.x + tw / 2, y2: tp.y,
        });
      }
    }

    const progress = {
      total: allLeaves.size,
      mastered: [...allLeaves].filter(id => skillMap.get(id) === 'mastered').length,
      learning: [...allLeaves].filter(id => skillMap.get(id) === 'learning').length,
    };

    // canvas width needs to accommodate max row + the widest single node (already in rowWidths)
    const canvasW = maxRowW > 0 ? maxRowW : (nodes[0]?.w ?? 136);
    return { nodes, conns, w: canvasW, h: byDepth.length * (NODE_H + V_GAP) || NODE_H, progress };
  }

  // Compute layouts for all expanded groups (roots AND sub-groups)
  let groupLayouts = $derived.by(() => {
    const map = new Map<string, GroupLayout>();
    function computeRecursive(id: string) {
      if (!expandedGroups.has(id)) return;
      map.set(id, computeLayout(id));
      for (const sub of getSubGroups(id)) {
        computeRecursive(sub);
      }
    }
    for (const root of roots) {
      computeRecursive(root);
    }
    return map;
  });

  // --- Cross-group DAG overlay ---
  // DOM refs
  let scrollContainerEl = $state<HTMLElement | null>(null);
  // groupEl[rootId] → the .group-box element
  let groupEls = $state(new Map<string, HTMLElement>());
  // nodeEl[tagId] → the .skill-node button element
  let nodeEls = $state(new Map<string, HTMLElement>());

  interface CrossConn {
    from: string; to: string; type: string;
    x1: number; y1: number; x2: number; y2: number;
  }
  let crossConns = $state<CrossConn[]>([]);

  // Identify cross-group edges: both tags exist in layouts but in different groups
  let crossGroupEdges = $derived.by(() => {
    // Build a tagId → rootId map
    const tagToRoot = new Map<string, string>();
    for (const [rootId, layout] of groupLayouts) {
      for (const node of layout.nodes) tagToRoot.set(node.id, rootId);
    }
    return prereqEdges.filter(e => {
      const fr = tagToRoot.get(e.from_tag);
      const tr = tagToRoot.get(e.to_tag);
      return fr && tr && fr !== tr;
    });
  });

  function recomputeCrossConns() {
    if (!scrollContainerEl || crossGroupEdges.length === 0) {
      crossConns = [];
      return;
    }
    const scrollRect = scrollContainerEl.getBoundingClientRect();
    const scrollTop = scrollContainerEl.scrollTop;
    const scrollLeft = scrollContainerEl.scrollLeft;

    const result: CrossConn[] = [];
    for (const edge of crossGroupEdges) {
      const fromEl = nodeEls.get(edge.from_tag);
      const toEl = nodeEls.get(edge.to_tag);
      if (!fromEl || !toEl) continue;

      const fr = fromEl.getBoundingClientRect();
      const tr = toEl.getBoundingClientRect();

      // Convert from viewport coords to scroll-container local coords
      const x1 = fr.left - scrollRect.left + scrollLeft + fr.width / 2;
      const y1 = fr.top - scrollRect.top + scrollTop + fr.height;
      const x2 = tr.left - scrollRect.left + scrollLeft + tr.width / 2;
      const y2 = tr.top - scrollRect.top + scrollTop;

      result.push({ from: edge.from_tag, to: edge.to_tag, type: edge.prereq_type, x1, y1, x2, y2 });
    }
    crossConns = result;
  }

  // Recompute after each render cycle that might change layout
  $effect(() => {
    // Access reactive dependencies so this re-runs when they change
    const _gl = groupLayouts;
    const _eg = expandedGroups;
    const _cge = crossGroupEdges;

    // Use rAF to ensure DOM has updated before reading positions
    const id = requestAnimationFrame(recomputeCrossConns);
    return () => cancelAnimationFrame(id);
  });

  // Svelte actions for registering DOM elements in our Maps
  function trackGroup(el: HTMLElement, rootId: string) {
    groupEls.set(rootId, el);
    return {
      update(newId: string) {
        groupEls.delete(rootId);
        groupEls.set(newId, el);
      },
      destroy() {
        groupEls.delete(rootId);
      }
    };
  }

  function trackNode(el: HTMLElement, tagId: string) {
    nodeEls.set(tagId, el);
    return {
      update(newId: string) {
        nodeEls.delete(tagId);
        nodeEls.set(newId, el);
      },
      destroy() {
        nodeEls.delete(tagId);
      }
    };
  }

  function crossConnPath(c: CrossConn): string {
    const my = (c.y1 + c.y2) / 2;
    return `M${c.x1},${c.y1} C${c.x1},${my} ${c.x2},${my} ${c.x2},${c.y2}`;
  }

  // --- Node dragging ---
  function onNodePointerDown(e: PointerEvent, nodeId: string, rootId: string, origX: number, origY: number) {
    if (e.button !== 0) return; // left button only
    e.preventDefault();
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    dragging = { nodeId, rootId, startX: e.clientX, startY: e.clientY, origX, origY };
  }

  function onNodePointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - dragging.startX;
    const dy = e.clientY - dragging.startY;
    dragOverrides.set(dragging.nodeId, { x: dragging.origX + dx, y: dragging.origY + dy });
    dragOverrides = new Map(dragOverrides); // trigger reactivity
  }

  let justDragged = false;
  function onNodePointerUp(e: PointerEvent) {
    if (!dragging) return;
    const dx = Math.abs(e.clientX - dragging.startX);
    const dy = Math.abs(e.clientY - dragging.startY);
    justDragged = dx > 3 || dy > 3;
    dragging = null;
    // Recompute cross-group arrows after drag
    requestAnimationFrame(recomputeCrossConns);
    // Clear justDragged after click event has fired
    setTimeout(() => { justDragged = false; }, 0);
  }

  // Apply drag overrides to a layout — returns new layout with overridden positions
  function applyDragOverrides(layout: GroupLayout): GroupLayout {
    const nodes = layout.nodes.map(n => {
      const ov = dragOverrides.get(n.id);
      return ov ? { ...n, x: ov.x, y: ov.y } : n;
    });
    // Recompute connections to follow dragged nodes
    const nodePos = new Map(nodes.map(n => [n.id, n]));
    const conns = layout.conns.map(c => {
      const fn = nodePos.get(c.from), tn = nodePos.get(c.to);
      if (!fn || !tn) return c;
      return { ...c, x1: fn.x + fn.w / 2, y1: fn.y + NODE_H, x2: tn.x + tn.w / 2, y2: tn.y };
    });
    return { ...layout, nodes, conns };
  }

  // Selected node details
  let selectedNode = $derived.by(() => {
    if (!selectedNodeId) return null;
    // Find in any expanded group
    for (const [, layout] of groupLayouts) {
      const node = layout.nodes.find(n => n.id === selectedNodeId);
      if (node) {
        const prereqs = prereqEdges.filter(e => e.to_tag === selectedNodeId).map(e => ({
          id: e.from_tag, name: resolveName(e.from_tag), type: e.prereq_type,
          met: skillMap.get(e.from_tag) === 'mastered',
        }));
        const unlocks = prereqEdges.filter(e => e.from_tag === selectedNodeId).map(e => ({
          id: e.to_tag, name: resolveName(e.to_tag),
        }));
        return { ...node, prereqs, unlocks };
      }
    }
    return null;
  });

  // --- Actions ---
  async function setSkillStatus(tagId: string, status: 'learning' | 'mastered' | 'none') {
    try {
      if (status === 'none') {
        await unlightSkill(tagId);
        skillMap.delete(tagId);
      } else {
        await lightSkill(tagId, status);
        if (status === 'mastered') {
          const s = await listSkills();
          skillMap = new Map(s.map(sk => [sk.tag_id, sk.status]));
          return;
        }
        skillMap.set(tagId, status);
      }
      skillMap = new Map(skillMap);
    } catch (e: any) {
      console.error('setSkillStatus failed:', e);
    }
  }

  async function addTagSkill(tagId: string, tagName?: string) {
    if (!allTags.some(t => t.id === tagId)) {
      const newTag = await createTagInline(tagId, tagName || tagId);
      allTags = [...allTags, newTag];
    }
    await setSkillStatus(tagId, 'learning');
    searchQuery = '';
  }

  // --- Data loading ---
  $effect(() => {
    Promise.all([getActiveTree(), getTagTree(), getTagPrereqs(), listSkills()]).then(([active, tr, pq, sk]) => {
      tree = tr;
      prereqEdges = pq;
      if (active) {
        tagNamesMap = active.tag_names_map;
        tagNamesI18n = active.tag_names_i18n;
      }
      skillMap = new Map(sk.map(s => [s.tag_id, s.status]));
      loading = false;
      // Load frontier skills if logged in
      if (getAuth()) {
        getFrontierSkills().then(fs => frontierSkills = fs).catch(() => {});
      }
    });
  });

  // --- Community state ---
  let treesLoading = $state(true);
  let communityTrees = $state<SkillTree[]>([]);
  let filterField = $state('');
  let isLoggedIn = $derived(!!getAuth());
  let filteredTrees = $derived(filterField ? communityTrees.filter(t => t.tag_id === filterField) : communityTrees);
  let availableFields = $derived.by(() => {
    const m = new Map<string, { id: string; name: string; count: number }>();
    for (const tr of communityTrees) {
      if (!tr.tag_id) continue;
      const ex = m.get(tr.tag_id);
      if (ex) ex.count++;
      else {
        const name = tr.tag_names ? resolveTagName(tr.tag_names, tr.tag_name || tr.tag_id, tr.tag_id) : (tr.tag_name || tr.tag_id);
        m.set(tr.tag_id, { id: tr.tag_id, name, count: 1 });
      }
    }
    return [...m.values()].sort((a, b) => b.count - a.count);
  });

  $effect(() => {
    if (activeTab === 'community' && treesLoading) {
      listSkillTrees().then(t => { communityTrees = t; treesLoading = false; });
    }
  });

  async function adopt(uri: string) {
    if (!isLoggedIn) return;
    await adoptSkillTree(uri);
    alert(t('skills.adopted'));
  }
  async function vote(uri: string, value: number) {
    if (!isLoggedIn) return;
    await castVote(uri, value);
    communityTrees = await listSkillTrees();
  }

  // SVG path helper
  function connPath(c: LayoutConn): string {
    const my = (c.y1 + c.y2) / 2;
    return `M${c.x1},${c.y1} C${c.x1},${my} ${c.x2},${my} ${c.x2},${c.y2}`;
  }
</script>

<div class="skills-page">
  <!-- Top toolbar -->
  <div class="skills-toolbar">
    <div class="toolbar-left">
      <div class="tab-bar">
        <button class="tab" class:active={activeTab === 'my'} onclick={() => activeTab = 'my'}>
          {t('skills.mySkills')}
        </button>
        <button class="tab" class:active={activeTab === 'community'} onclick={() => activeTab = 'community'}>
          {t('skills.communityTrees')}
        </button>
      </div>
      {#if activeTab === 'my' && !loading}
        <span class="stat-count">{t('skills.mastered')} <strong>{masteredCount}</strong> · {t('skills.learning')} <strong>{learningCount}</strong></span>
      {/if}
    </div>
    <div class="toolbar-right">
      {#if activeTab === 'my'}
        <div class="search-box">
          <input
            type="text"
            bind:value={searchQuery}
            placeholder={t('skills.searchTag')}
            onfocus={() => { if (!allTags.length) listTags().then(t => allTags = t); }}
          />
          {#if searchSuggestions.length > 0}
            <div class="search-dropdown">
              {#each searchSuggestions as tag}
                <button class="search-item" onclick={() => addTagSkill(tag.id, tag.name)}>
                  {resolveTagName(tag.names || {}, tag.name, tag.id)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <div class="legend">
          <span class="legend-item"><span class="dot mastered"></span>{t('skills.mastered')}</span>
          <span class="legend-item"><span class="dot learning"></span>{t('skills.learning')}</span>
          <span class="legend-item"><span class="dot available"></span>{t('skills.available')}</span>
          <span class="legend-item"><span class="dot locked"></span>{t('skills.locked')}</span>
        </div>
      {:else if isLoggedIn}
        <a href="/skill-tree/new" class="create-btn">{t('skills.createTree')}</a>
      {/if}
    </div>
  </div>

  {#if activeTab === 'my'}
    {#if loading}
      <div class="center-msg"><p>Loading...</p></div>
    {:else}
      <div class="groups-scroll"
        bind:this={scrollContainerEl}
        onscroll={recomputeCrossConns}
        onclick={(e: MouseEvent) => { if ((e.target as HTMLElement).classList.contains('groups-scroll')) selectedNodeId = null; }}
      >
        <!-- Frontier skills: next to learn -->
        {#if frontierSkills.length > 0}
          <div class="frontier-section">
            <h3 class="frontier-title">{t('skills.nextToLearn')}</h3>
            <div class="frontier-chips">
              {#each frontierSkills as fs}
                <a href="/tag?id={encodeURIComponent(fs.tag_id)}" class="frontier-chip">
                  <span class="frontier-name">{resolveTagName(fs.tag_names, fs.tag_name, fs.tag_id)}</span>
                  {#if fs.article_count > 0}
                    <span class="frontier-count">{fs.article_count}</span>
                  {/if}
                </a>
              {/each}
            </div>
          </div>
        {/if}
        {#snippet renderGroup(groupId: string, depth: number)}
          {@const rawLayout = groupLayouts.get(groupId)}
          {@const layout = rawLayout ? applyDragOverrides(rawLayout) : rawLayout}
          {@const expanded = expandedGroups.has(groupId)}
          {@const subGroups = getSubGroups(groupId)}
          <div class="group-box" class:expanded class:sub-group={depth > 0} use:trackGroup={groupId}>
            <button class="group-header" class:sub-header={depth > 0} onclick={() => toggleGroup(groupId)}>
              <svg class="chevron" class:open={expanded} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
              <span class="group-name">{resolveName(groupId)}</span>
              {#if layout}
                <span class="group-progress">{layout.progress.mastered}/{layout.progress.total}</span>
                {#if layout.progress.total > 0}
                  <div class="group-bar">
                    <div class="bar-fill mastered" style="width:{layout.progress.mastered / layout.progress.total * 100}%"></div>
                    <div class="bar-fill learning" style="width:{layout.progress.learning / layout.progress.total * 100}%"></div>
                  </div>
                {/if}
              {/if}
            </button>

            {#if expanded}
              <!-- Leaf nodes for this group -->
              {#if layout && layout.nodes.length > 0}
                <div class="group-canvas">
                  <div class="group-content" style="min-width:{layout.w}px; min-height:{layout.h + 20}px;">
                    <svg class="conn-svg" style="width:{layout.w}px; height:{layout.h + 20}px;">
                      <defs>
                        <marker id="arr-req-{groupId}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto">
                          <path d="M0,0 L10,5 L0,10z" fill="#ef4444"/>
                        </marker>
                        <marker id="arr-rec-{groupId}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto">
                          <path d="M0,0 L10,5 L0,10z" fill="#f59e0b"/>
                        </marker>
                      </defs>
                      {#each layout.conns as c}
                        <path
                          d={connPath(c)}
                          class="conn conn-{c.type}"
                          marker-end={c.type === 'required' ? `url(#arr-req-${groupId})` : `url(#arr-rec-${groupId})`}
                        />
                      {/each}
                    </svg>

                    {#each layout.nodes as node (node.id)}
                      <button
                        class="skill-node st-{node.status}"
                        class:selected={selectedNodeId === node.id}
                        class:dragging-node={dragging?.nodeId === node.id}
                        style="left:{node.x}px; top:{node.y}px; width:{node.w}px; height:{NODE_H}px;"
                        onclick={() => { if (!justDragged) selectedNodeId = selectedNodeId === node.id ? null : node.id; }}
                        ondblclick={() => window.location.href = `/tag?id=${encodeURIComponent(node.id)}`}
                        onpointerdown={(e) => onNodePointerDown(e, node.id, groupId, node.x, node.y)}
                        onpointermove={onNodePointerMove}
                        onpointerup={onNodePointerUp}
                        title={node.name}
                        use:trackNode={node.id}
                      >
                        <span class="node-icon">
                          {#if node.status === 'mastered'}
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 13l4 4L19 7"/></svg>
                          {:else if node.status === 'learning'}
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="6"/></svg>
                          {:else if node.status === 'available'}
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2l3 7h7l-5.5 4.5L18.5 21 12 16.5 5.5 21l2-7.5L2 9h7z"/></svg>
                          {:else}
                            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="11" width="14" height="11" rx="2"/><path d="M8 11V7a4 4 0 118 0v4"/></svg>
                          {/if}
                        </span>
                        <span class="node-name">{node.name}</span>
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Nested sub-groups -->
              {#each subGroups as sub (sub)}
                {@render renderGroup(sub, depth + 1)}
              {/each}

              {#if (!layout || layout.nodes.length === 0) && subGroups.length === 0}
                <div class="group-empty">{t('skills.noSkillsInGroup')}</div>
              {/if}
            {/if}
          </div>
        {/snippet}

        {#each roots as root (root)}
          {@render renderGroup(root, 0)}
        {/each}

        <!-- Cross-group DAG overlay -->
        {#if crossConns.length > 0}
          <svg class="cross-conn-svg" aria-hidden="true">
            <defs>
              <marker id="xarr-req" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto">
                <path d="M0,0 L10,5 L0,10z" fill="#ef4444"/>
              </marker>
              <marker id="xarr-rec" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto">
                <path d="M0,0 L10,5 L0,10z" fill="#f59e0b"/>
              </marker>
            </defs>
            {#each crossConns as c}
              <path
                d={crossConnPath(c)}
                class="conn conn-{c.type} cross-conn"
                marker-end={c.type === 'required' ? 'url(#xarr-req)' : 'url(#xarr-rec)'}
              />
            {/each}
          </svg>
        {/if}
      </div>

      <!-- Detail panel -->
      {#if selectedNode}
        <div class="detail-panel" onclick={(e: MouseEvent) => e.stopPropagation()}>
          <button class="panel-close" onclick={() => selectedNodeId = null}>×</button>
          <h3 class="panel-title">{selectedNode.name}</h3>

          <div class="panel-status st-{selectedNode.status}">
            {#if selectedNode.status === 'mastered'}{t('skills.mastered')}
            {:else if selectedNode.status === 'learning'}{t('skills.learning')}
            {:else if selectedNode.status === 'available'}{t('skills.available')}
            {:else}{t('skills.locked')}{/if}
          </div>

          {#if selectedNode.prereqs.length > 0}
            <div class="panel-section">
              <h4>{t('skills.prerequisites')}</h4>
              {#each selectedNode.prereqs as p}
                <div class="prereq-row">
                  <span class="prereq-check" class:met={p.met}>{p.met ? '✓' : '○'}</span>
                  <a href="/tag?id={encodeURIComponent(p.id)}" class="prereq-name">{p.name}</a>
                  <span class="prereq-type type-{p.type}">{p.type}</span>
                </div>
              {/each}
            </div>
          {/if}

          {#if selectedNode.unlocks.length > 0}
            <div class="panel-section">
              <h4>{t('skills.unlocks')}</h4>
              {#each selectedNode.unlocks as u}
                <a href="/tag?id={encodeURIComponent(u.id)}" class="unlock-link">{u.name}</a>
              {/each}
            </div>
          {/if}

          <a href="/tag?id={encodeURIComponent(selectedNode.id)}" class="panel-link">
            {t('skills.viewTag')} →
          </a>

          <div class="panel-actions">
            {#if selectedNode.status === 'locked'}
              <p class="hint">{t('skills.completePrereqs')}</p>
            {:else if selectedNode.status === 'available'}
              <button class="action-btn btn-learning" onclick={() => setSkillStatus(selectedNode!.id, 'learning')}>
                {t('skills.startLearning')}
              </button>
              <button class="action-btn btn-mastered" onclick={() => setSkillStatus(selectedNode!.id, 'mastered')}>
                {t('skills.markMastered')}
              </button>
            {:else if selectedNode.status === 'learning'}
              <button class="action-btn btn-mastered" onclick={() => setSkillStatus(selectedNode!.id, 'mastered')}>
                {t('skills.markMastered')}
              </button>
              <button class="action-btn btn-reset" onclick={() => setSkillStatus(selectedNode!.id, 'none')}>
                {t('skills.resetSkill')}
              </button>
            {:else}
              <button class="action-btn btn-reset" onclick={() => setSkillStatus(selectedNode!.id, 'none')}>
                {t('skills.resetSkill')}
              </button>
            {/if}
          </div>
        </div>
      {/if}
    {/if}

  {:else}
    <!-- Community tab -->
    <div class="community-section">
      <p class="subtitle">{t('skills.browseHint')}</p>
      <div class="field-filter">
        <button class="filter-btn" class:active={!filterField} onclick={() => filterField = ''}>{t('home.all')}</button>
        {#each availableFields as f}
          <button class="filter-btn" class:active={filterField === f.id} onclick={() => filterField = f.id}>{f.name} ({f.count})</button>
        {/each}
      </div>
      {#if treesLoading}
        <p class="meta">Loading...</p>
      {:else if communityTrees.length === 0}
        <div class="empty">
          <p>{t('skills.noTrees')}</p>
          {#if isLoggedIn}<a href="/skill-tree/new">{t('skills.createFirst')}</a>{/if}
        </div>
      {:else}
        <div class="tree-list">
          {#each filteredTrees as tree (tree.at_uri)}
            <div class="tree-card">
              <div class="tree-main">
                <a href="/skill-tree?uri={encodeURIComponent(tree.at_uri)}" class="tree-title">{tree.title}</a>
                {#if tree.tag_id}
                  <span class="field-badge">{tree.tag_names ? resolveTagName(tree.tag_names, tree.tag_name || tree.tag_id, tree.tag_id) : (tree.tag_name || tree.tag_id)}</span>
                {/if}
                {#if tree.forked_from}<span class="forked-badge">Fork</span>{/if}
                {#if tree.description}<p class="tree-desc">{tree.description}</p>{/if}
                <div class="tree-meta">
                  <span>{tree.author_handle ? `@${tree.author_handle}` : tree.did.slice(0, 20)}</span>
                  <span>{tree.edge_count} {t('skills.edgeCount')}</span>
                  <span>{tree.adopt_count} {t('skills.adoptCount')}</span>
                </div>
              </div>
              <div class="tree-actions">
                <div class="vote-col">
                  <button class="vote-btn" onclick={() => vote(tree.at_uri, 1)} disabled={!isLoggedIn}>▲</button>
                  <span class="score">{tree.score ?? 0}</span>
                  <button class="vote-btn" onclick={() => vote(tree.at_uri, -1)} disabled={!isLoggedIn}>▼</button>
                </div>
                <button class="adopt-btn" onclick={() => adopt(tree.at_uri)} disabled={!isLoggedIn}>{t('skills.adopt')}</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .skills-page {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 3.5rem);
  }

  /* ─── Toolbar ─── */
  .skills-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .toolbar-left { display: flex; align-items: center; gap: 16px; }
  .toolbar-right { display: flex; align-items: center; gap: 16px; }
  .stat-count { font-size: 13px; color: var(--text-secondary); }

  .tab-bar { display: flex; }
  .tab {
    padding: 6px 16px; font-size: 14px;
    border: 1px solid var(--border); background: none;
    color: var(--text-secondary); cursor: pointer; transition: all 0.15s;
  }
  .tab:first-child { border-radius: 4px 0 0 4px; }
  .tab:last-child { border-radius: 0 4px 4px 0; border-left: none; }
  .tab.active { background: var(--accent); color: white; border-color: var(--accent); }
  .tab:hover:not(.active) { border-color: var(--accent); color: var(--accent); }

  .legend { display: flex; align-items: center; gap: 12px; font-size: 12px; color: var(--text-secondary); }
  .legend-item { display: flex; align-items: center; gap: 4px; }
  .dot {
    width: 10px; height: 10px; border-radius: 50%; display: inline-block;
  }
  .dot.mastered { background: var(--green, #5f9b65); }
  .dot.learning { background: var(--amber, #f59e0b); }
  .dot.available { background: transparent; border: 2px solid var(--green, #5f9b65); box-sizing: border-box; }
  .dot.locked { background: #d1d5db; }

  .search-box {
    position: relative;
  }
  .search-box input {
    width: 180px; padding: 5px 10px; font-size: 13px;
    border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg-page); font-family: var(--font-sans);
  }
  .search-dropdown {
    position: absolute; top: 100%; right: 0; width: 240px;
    background: var(--bg-white); border: 1px solid var(--border);
    border-radius: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 20; max-height: 200px; overflow-y: auto;
  }
  .search-item {
    display: block; width: 100%; padding: 8px 12px; border: none;
    background: none; text-align: left; cursor: pointer; font-size: 13px;
  }
  .search-item:hover { background: var(--bg-gray, #f5f5f5); }

  /* ─── Scrollable groups area ─── */
  .groups-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    position: relative;
  }

  /* ─── Group box ─── */
  .group-box {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    overflow: hidden;
    transition: border-color 0.2s;
  }
  .group-box.expanded {
    border-color: var(--border-strong, var(--border));
  }
  .group-box.sub-group {
    margin: 4px 12px 8px;
    border-radius: 6px;
    border-color: var(--border);
    background: var(--bg-page, #f9f9f7);
  }
  .sub-header {
    font-size: 13px !important;
    padding: 6px 12px !important;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 12px 16px;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 15px;
    text-align: left;
    transition: background 0.15s;
  }
  .group-header:hover {
    background: var(--bg-gray, rgba(0,0,0,0.02));
  }
  .chevron {
    flex-shrink: 0;
    transition: transform 0.2s;
    color: var(--text-hint);
    transform: rotate(0deg);
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .group-name {
    font-family: var(--font-serif);
    font-weight: 600;
    color: var(--text-primary);
  }
  .group-progress {
    font-size: 12px;
    color: var(--text-hint);
    margin-left: auto;
  }
  .group-bar {
    width: 80px;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
    display: flex;
    flex-shrink: 0;
  }
  .bar-fill.mastered { background: var(--green, #5f9b65); transition: width 0.3s; }
  .bar-fill.learning { background: var(--amber, #f59e0b); transition: width 0.3s; }

  .group-canvas {
    padding: 16px 20px 24px;
    overflow-x: auto;
    border-top: 1px solid var(--border);
    background:
      radial-gradient(circle at 1px 1px, var(--grid-dot, rgba(0,0,0,0.03)) 1px, transparent 0);
    background-size: 20px 20px;
  }
  :global([data-theme="dark"]) .group-canvas {
    --grid-dot: rgba(255,255,255,0.03);
  }
  .group-content {
    position: relative;
    margin: 0 auto;
  }
  .group-empty {
    padding: 16px 20px;
    font-size: 13px;
    color: var(--text-hint);
    border-top: 1px solid var(--border);
  }

  /* ─── SVG Connections ─── */
  .conn-svg {
    position: absolute; top: 0; left: 0;
    pointer-events: none; overflow: visible;
  }
  .conn {
    fill: none; stroke-width: 2;
    transition: stroke-opacity 0.2s;
  }
  .conn-required { stroke: #ef4444; stroke-dasharray: 8 4; }
  .conn-recommended { stroke: #f59e0b; stroke-dasharray: 6 4; }

  /* ─── Cross-group overlay SVG ─── */
  .cross-conn-svg {
    position: absolute;
    top: 0; left: 0;
    width: 100%; height: 100%;
    pointer-events: none;
    overflow: visible;
    z-index: 8;
  }
  .cross-conn {
    opacity: 0.5;
    stroke-width: 1.5;
  }

  /* ─── Skill Nodes ─── */
  .skill-node {
    position: absolute;
    display: flex; align-items: center; gap: 6px;
    padding: 0 10px;
    border-radius: 8px;
    border: 2px solid;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s ease;
    box-sizing: border-box;
    text-align: left;
    background: var(--bg-white);
    overflow: hidden;
  }
  .skill-node:hover {
    transform: translateY(-2px);
    z-index: 5;
  }
  .skill-node.selected {
    z-index: 10;
    transform: scale(1.08);
  }
  .skill-node.dragging-node {
    z-index: 20;
    opacity: 0.85;
    cursor: grabbing;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }

  .node-icon {
    flex-shrink: 0; width: 16px; height: 16px;
    display: flex; align-items: center; justify-content: center;
  }
  .node-name {
    flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-family: var(--font-sans);
    line-height: 1.2;
  }

  /* Locked */
  .st-locked {
    border-color: #d1d5db; background: #f3f4f6; color: #9ca3af;
    opacity: 0.55;
  }
  :global([data-theme="dark"]) .st-locked {
    background: #1f2937; border-color: #374151; color: #6b7280;
  }

  /* Available */
  .st-available {
    border-color: var(--green, #5f9b65); color: var(--text-primary);
    box-shadow: 0 0 0 1px rgba(95,155,101,0.15), 0 2px 8px rgba(95,155,101,0.1);
  }
  .st-available:hover {
    box-shadow: 0 0 0 2px rgba(95,155,101,0.25), 0 4px 16px rgba(95,155,101,0.15);
  }

  /* Learning */
  .st-learning {
    border-color: #d97706;
    background: linear-gradient(135deg, #fbbf24 0%, #f59e0b 100%);
    color: #fff;
    box-shadow: 0 0 12px rgba(245,158,11,0.25);
  }
  .st-learning:hover {
    box-shadow: 0 0 20px rgba(245,158,11,0.35);
  }

  /* Mastered */
  .st-mastered {
    border-color: #4a8a50;
    background: linear-gradient(135deg, #6dbf73 0%, #5f9b65 100%);
    color: #fff;
    box-shadow: 0 0 12px rgba(95,155,101,0.25);
  }
  .st-mastered:hover {
    box-shadow: 0 0 20px rgba(95,155,101,0.35);
  }

  .skill-node.selected.st-mastered { box-shadow: 0 0 0 3px rgba(95,155,101,0.4); }
  .skill-node.selected.st-learning { box-shadow: 0 0 0 3px rgba(245,158,11,0.4); }
  .skill-node.selected.st-available { box-shadow: 0 0 0 3px rgba(95,155,101,0.3); }

  /* ─── Detail Panel ─── */
  .detail-panel {
    position: fixed; right: 0; top: 3.5rem;
    width: 280px; height: calc(100vh - 3.5rem);
    background: var(--bg-white);
    border-left: 1px solid var(--border);
    padding: 20px;
    box-shadow: -4px 0 20px rgba(0,0,0,0.08);
    overflow-y: auto;
    z-index: 50;
  }
  .panel-close {
    position: absolute; top: 12px; right: 12px;
    background: none; border: none; font-size: 18px;
    color: var(--text-hint); cursor: pointer; padding: 4px 8px;
  }
  .panel-close:hover { color: var(--text-primary); }
  .panel-title {
    font-family: var(--font-serif);
    font-size: 1.25rem; font-weight: 500;
    margin: 0 0 8px; padding-right: 24px;
  }
  .panel-status {
    display: inline-block; padding: 3px 10px;
    border-radius: 12px; font-size: 12px; font-weight: 600;
    margin-bottom: 16px;
  }
  .panel-status.st-mastered { background: rgba(95,155,101,0.15); color: #4a8a50; }
  .panel-status.st-learning { background: rgba(245,158,11,0.15); color: #b45309; }
  .panel-status.st-available { background: rgba(95,155,101,0.1); color: #5f9b65; }
  .panel-status.st-locked { background: rgba(156,163,175,0.15); color: #6b7280; }

  .panel-section { margin-bottom: 14px; }
  .panel-section h4 {
    font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--text-hint); margin: 0 0 6px;
  }
  .prereq-row {
    display: flex; align-items: center; gap: 6px;
    font-size: 13px; padding: 3px 0;
  }
  .prereq-check { width: 16px; text-align: center; }
  .prereq-check.met { color: var(--green, #5f9b65); }
  .prereq-check:not(.met) { color: var(--text-hint); }
  .prereq-name { color: var(--text-primary); text-decoration: none; }
  .prereq-name:hover { color: var(--accent); }
  .prereq-type {
    font-size: 10px; padding: 1px 5px; border-radius: 3px;
    margin-left: auto;
  }
  .type-required { background: rgba(239,68,68,0.1); color: #dc2626; }
  .type-recommended { background: rgba(245,158,11,0.1); color: #b45309; }

  .unlock-link {
    display: block; font-size: 13px; padding: 2px 0;
    color: var(--text-secondary); text-decoration: none;
  }
  .unlock-link:hover { color: var(--accent); }

  .panel-link {
    display: block; font-size: 13px; color: var(--accent);
    text-decoration: none; margin: 12px 0;
  }
  .panel-link:hover { text-decoration: underline; }

  .panel-actions {
    display: flex; flex-direction: column; gap: 8px; margin-top: 16px;
    padding-top: 16px; border-top: 1px solid var(--border);
  }
  .action-btn {
    padding: 8px 16px; font-size: 13px; border: none;
    border-radius: 6px; cursor: pointer; transition: all 0.15s;
    font-family: var(--font-sans);
  }
  .btn-learning { background: #fbbf24; color: #78350f; }
  .btn-learning:hover { background: #f59e0b; }
  .btn-mastered { background: var(--green, #5f9b65); color: white; }
  .btn-mastered:hover { background: #4a8a50; }
  .btn-reset { background: none; border: 1px solid var(--border) !important; color: var(--text-secondary); }
  .btn-reset:hover { border-color: var(--text-hint) !important; }
  .hint { font-size: 13px; color: var(--text-hint); margin: 0; }

  .center-msg {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--text-hint);
  }

  /* ─── Community Section ─── */
  .community-section { flex: 1; overflow-y: auto; padding: 20px; }
  .subtitle { font-size: 14px; color: var(--text-secondary); margin: 0 0 16px; }
  .create-btn {
    padding: 6px 16px; font-size: 13px; background: var(--accent);
    color: white; border-radius: 4px; text-decoration: none;
  }
  .create-btn:hover { opacity: 0.85; text-decoration: none; }
  .field-filter { display: flex; gap: 6px; margin-bottom: 16px; flex-wrap: wrap; }
  .filter-btn {
    padding: 4px 12px; font-size: 13px; border: 1px solid var(--border);
    border-radius: 14px; background: var(--bg-white); color: var(--text-secondary);
    cursor: pointer; transition: all 0.15s;
  }
  .filter-btn:hover { border-color: var(--accent); color: var(--accent); }
  .filter-btn.active { background: var(--accent); color: white; border-color: var(--accent); }
  .tree-list { display: flex; flex-direction: column; gap: 12px; }
  .tree-card {
    display: flex; gap: 16px; padding: 16px 20px;
    border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg-white); transition: border-color 0.15s;
  }
  .tree-card:hover { border-color: var(--border-strong); }
  .tree-main { flex: 1; min-width: 0; }
  .tree-title {
    font-family: var(--font-serif); font-size: 1.15rem;
    color: var(--text-primary); text-decoration: none;
  }
  .tree-title:hover { color: var(--accent); }
  .forked-badge {
    font-size: 11px; background: var(--bg-gray, #f0f0f0);
    color: var(--text-hint); padding: 1px 6px; border-radius: 3px; margin-left: 6px;
  }
  .field-badge {
    font-size: 11px; background: rgba(95,155,101,0.12);
    color: var(--accent); padding: 1px 8px; border-radius: 3px; margin-left: 6px;
  }
  .tree-desc { font-size: 14px; color: var(--text-secondary); margin: 4px 0 0; line-height: 1.5; }
  .tree-meta { display: flex; gap: 12px; margin-top: 8px; font-size: 13px; color: var(--text-hint); }
  .tree-actions { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }
  .vote-col { display: flex; flex-direction: column; align-items: center; gap: 2px; }
  .vote-btn {
    background: none; border: none; cursor: pointer;
    color: var(--text-hint); font-size: 12px; padding: 2px 4px;
  }
  .vote-btn:hover:not(:disabled) { color: var(--accent); }
  .vote-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .score { font-size: 15px; font-weight: 600; color: var(--text-primary); }
  .adopt-btn {
    padding: 6px 14px; font-size: 13px; border: 1px solid var(--accent);
    background: none; color: var(--accent); border-radius: 4px; cursor: pointer;
  }
  .adopt-btn:hover:not(:disabled) { background: var(--accent); color: white; }
  .adopt-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .empty { color: var(--text-hint); }

  /* ─── Frontier Skills ─── */
  .frontier-section {
    padding: 16px 20px;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 8px;
    border-left: 3px solid var(--accent);
  }
  .frontier-title {
    font-family: var(--font-serif);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 10px;
  }
  .frontier-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .frontier-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid var(--accent);
    border-radius: 16px;
    background: rgba(95,155,101,0.06);
    color: var(--accent);
    font-size: 13px;
    text-decoration: none;
    transition: all 0.15s;
  }
  .frontier-chip:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }
  .frontier-name { font-family: var(--font-sans); }
  .frontier-count {
    font-size: 11px;
    background: rgba(95,155,101,0.15);
    color: var(--accent);
    padding: 1px 6px;
    border-radius: 8px;
    font-weight: 600;
  }
  .frontier-chip:hover .frontier-count {
    background: rgba(255,255,255,0.25);
    color: white;
  }
</style>
