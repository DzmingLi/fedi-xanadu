<script lang="ts">
  import { goto } from '$app/navigation';
  import { createQuery } from '@tanstack/svelte-query';
  import { keys } from '$lib/queries';
  import { getGraph, getTagTree, listSkills, lightSkill, unlightSkill, listTags, createTagInline, listSkillTrees, adoptSkillTree, castVote } from '$lib/api';
  import { getAuth } from '$lib/auth.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { authorName, tagName as resolveTagName } from '$lib/display';
  import type { GraphNode, GraphEdge, TagTreeEntry, Tag, SkillTree } from '$lib/types';

  // --- Tab state ---
  let activeTab = $state<'my' | 'community'>('my');

  // --- My Skills state ---
  let loading = $state(true);
  let graphNodes = $state<GraphNode[]>([]);
  let graphEdges = $state<GraphEdge[]>([]);
  let tree = $state<TagTreeEntry[]>([]);
  let skillMap = $state(new Map<string, 'mastered' | 'learning'>());
  let masteredCount = $derived([...skillMap.values()].filter(s => s === 'mastered').length);
  let learningCount = $derived([...skillMap.values()].filter(s => s === 'learning').length);

  // Field navigation & selection
  let activeField = $state('');
  let selectedNodeId = $state<string | null>(null);

  // Tag search
  let allTags = $state<Tag[]>([]);
  let searchQuery = $state('');
  let searchSuggestions = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return allTags.filter(t => t.id.toLowerCase().includes(q) || t.name.toLowerCase().includes(q)).slice(0, 8);
  });

  // --- Tree structure computation ---
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

  let nodeMap = $derived(new Map(graphNodes.map(n => [n.id, n])));

  function resolveName(id: string): string {
    const gn = nodeMap.get(id);
    return gn ? resolveTagName(gn.names, gn.name, id) : id;
  }

  // Set first field
  $effect(() => {
    if (roots.length > 0 && !activeField) activeField = roots[0];
  });

  // --- Layout computation ---
  const NODE_W = 136;
  const NODE_H = 52;
  const H_GAP = 24;
  const V_GAP = 56;

  function getNodeStatus(tagId: string): 'locked' | 'available' | 'learning' | 'mastered' {
    const current = skillMap.get(tagId);
    if (current === 'mastered') return 'mastered';
    if (current === 'learning') return 'learning';
    const required = graphEdges.filter(e => e.to === tagId && e.type === 'required');
    if (required.length === 0) return 'available';
    return required.every(e => skillMap.get(e.from) === 'mastered') ? 'available' : 'locked';
  }

  interface LayoutNode {
    id: string; name: string; x: number; y: number;
    status: 'locked' | 'available' | 'learning' | 'mastered';
  }
  interface LayoutConn {
    from: string; to: string; type: string;
    x1: number; y1: number; x2: number; y2: number;
  }

  let fieldLayout = $derived.by(() => {
    if (!activeField) return { nodes: [] as LayoutNode[], conns: [] as LayoutConn[], w: 0, h: 0 };

    // BFS to collect nodes by depth
    const byDepth: string[][] = [];
    const visited = new Set<string>();

    function visit(id: string, depth: number) {
      if (visited.has(id)) return;
      visited.add(id);
      while (byDepth.length <= depth) byDepth.push([]);
      byDepth[depth].push(id);
      for (const ch of (childrenOf.map.get(id) || [])) visit(ch, depth + 1);
    }
    for (const ch of (childrenOf.map.get(activeField) || [])) visit(ch, 0);

    // Position nodes centered per row
    const positions = new Map<string, { x: number; y: number }>();
    let maxRowW = 0;
    for (let row = 0; row < byDepth.length; row++) {
      const nodes = byDepth[row];
      const rowW = nodes.length * NODE_W + (nodes.length - 1) * H_GAP;
      maxRowW = Math.max(maxRowW, rowW);
    }
    for (let row = 0; row < byDepth.length; row++) {
      const nodes = byDepth[row];
      const rowW = nodes.length * NODE_W + (nodes.length - 1) * H_GAP;
      const offsetX = (maxRowW - rowW) / 2;
      for (let col = 0; col < nodes.length; col++) {
        positions.set(nodes[col], {
          x: offsetX + col * (NODE_W + H_GAP),
          y: row * (NODE_H + V_GAP),
        });
      }
    }

    const nodes: LayoutNode[] = [];
    for (const [id, pos] of positions) {
      nodes.push({ id, name: resolveName(id), x: pos.x, y: pos.y, status: getNodeStatus(id) });
    }

    // Connections: prerequisite edges within this field
    const conns: LayoutConn[] = [];
    for (const e of graphEdges) {
      const fp = positions.get(e.from), tp = positions.get(e.to);
      if (fp && tp) {
        conns.push({
          from: e.from, to: e.to, type: e.type,
          x1: fp.x + NODE_W / 2, y1: fp.y + NODE_H,
          x2: tp.x + NODE_W / 2, y2: tp.y,
        });
      }
    }
    // Tree hierarchy connections (shown as subtle lines when no prereq edge exists)
    for (const entry of tree) {
      const fp = positions.get(entry.parent_tag), tp = positions.get(entry.child_tag);
      if (fp && tp && !conns.some(c => c.from === entry.parent_tag && c.to === entry.child_tag)) {
        conns.push({
          from: entry.parent_tag, to: entry.child_tag, type: 'hierarchy',
          x1: fp.x + NODE_W / 2, y1: fp.y + NODE_H,
          x2: tp.x + NODE_W / 2, y2: tp.y,
        });
      }
    }

    return { nodes, conns, w: maxRowW + NODE_W, h: byDepth.length * (NODE_H + V_GAP) || NODE_H };
  });

  // Progress
  let fieldProgress = $derived.by(() => {
    const all = fieldLayout.nodes;
    return {
      total: all.length,
      mastered: all.filter(n => n.status === 'mastered').length,
      learning: all.filter(n => n.status === 'learning').length,
    };
  });

  // Selected node details
  let selectedNode = $derived.by(() => {
    if (!selectedNodeId) return null;
    const node = fieldLayout.nodes.find(n => n.id === selectedNodeId);
    if (!node) return null;

    const prereqs = graphEdges.filter(e => e.to === selectedNodeId).map(e => ({
      id: e.from, name: resolveName(e.from), type: e.type,
      met: skillMap.get(e.from) === 'mastered',
    }));
    const unlocks = graphEdges.filter(e => e.from === selectedNodeId).map(e => ({
      id: e.to, name: resolveName(e.to),
    }));
    return { ...node, prereqs, unlocks };
  });

  // --- Actions ---
  async function setSkillStatus(tagId: string, status: 'learning' | 'mastered' | 'none') {
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
    Promise.all([getGraph(), getTagTree(), listSkills()]).then(([data, tr, sk]) => {
      graphNodes = data.nodes;
      graphEdges = data.edges;
      tree = tr;
      skillMap = new Map(sk.map(s => [s.tag_id, s.status]));
      loading = false;
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
      <!-- Field tabs -->
      <div class="field-bar">
        {#each roots as field}
          <button
            class="field-tab"
            class:active={activeField === field}
            onclick={() => { activeField = field; selectedNodeId = null; }}
          >
            {resolveName(field)}
          </button>
        {/each}

        <!-- Search -->
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
      </div>

      <!-- Progress -->
      {#if fieldProgress.total > 0}
        <div class="progress-row">
          <div class="progress-track">
            <div class="progress-fill mastered" style="width:{fieldProgress.total ? (fieldProgress.mastered / fieldProgress.total * 100) : 0}%"></div>
            <div class="progress-fill learning" style="width:{fieldProgress.total ? (fieldProgress.learning / fieldProgress.total * 100) : 0}%"></div>
          </div>
          <span class="progress-text">{fieldProgress.mastered}/{fieldProgress.total}</span>
        </div>
      {/if}

      <!-- Tree canvas -->
      <div class="tree-canvas" onclick={(e: MouseEvent) => { if (e.target === e.currentTarget) selectedNodeId = null; }}>
        <div class="tree-scroll">
          <div class="tree-content" style="min-width:{fieldLayout.w}px; min-height:{fieldLayout.h + 40}px;">
            <!-- SVG connections -->
            <svg class="conn-svg" style="width:{fieldLayout.w}px; height:{fieldLayout.h + 40}px;">
              <defs>
                <marker id="arr-req" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto">
                  <path d="M0,0 L10,5 L0,10z" fill="#ef4444"/>
                </marker>
                <marker id="arr-rec" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto">
                  <path d="M0,0 L10,5 L0,10z" fill="#f59e0b"/>
                </marker>
                <marker id="arr-hier" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="6" markerHeight="6" orient="auto">
                  <path d="M0,0 L10,5 L0,10z" fill="var(--conn-hier, #c4b5a0)"/>
                </marker>
              </defs>
              {#each fieldLayout.conns as c}
                <path
                  d={connPath(c)}
                  class="conn conn-{c.type}"
                  marker-end={c.type === 'required' ? 'url(#arr-req)' : c.type === 'recommended' ? 'url(#arr-rec)' : 'url(#arr-hier)'}
                />
              {/each}
            </svg>

            <!-- Nodes -->
            {#each fieldLayout.nodes as node (node.id)}
              <button
                class="skill-node st-{node.status}"
                class:selected={selectedNodeId === node.id}
                style="left:{node.x}px; top:{node.y}px; width:{NODE_W}px; height:{NODE_H}px;"
                onclick={() => selectedNodeId = selectedNodeId === node.id ? null : node.id}
                ondblclick={() => goto(`/tag?id=${encodeURIComponent(node.id)}`)}
                title={node.name}
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

  /* Toolbar */
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

  /* Field tabs bar */
  .field-bar {
    display: flex; align-items: center; gap: 0;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .field-tab {
    padding: 10px 20px; font-size: 14px;
    font-family: var(--font-serif);
    border: none; border-bottom: 2px solid transparent;
    background: none; color: var(--text-secondary);
    cursor: pointer; transition: all 0.15s;
  }
  .field-tab.active {
    color: var(--accent); border-bottom-color: var(--accent); font-weight: 600;
  }
  .field-tab:hover:not(.active) { color: var(--text-primary); }

  .search-box {
    position: relative; margin-left: auto;
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

  /* Progress */
  .progress-row {
    display: flex; align-items: center; gap: 12px;
    padding: 8px 20px; flex-shrink: 0;
  }
  .progress-track {
    flex: 1; height: 6px; background: var(--border);
    border-radius: 3px; overflow: hidden; display: flex;
  }
  .progress-fill.mastered { background: var(--green, #5f9b65); transition: width 0.3s; }
  .progress-fill.learning { background: var(--amber, #f59e0b); transition: width 0.3s; }
  .progress-text { font-size: 12px; color: var(--text-hint); white-space: nowrap; }

  /* Tree Canvas */
  .tree-canvas {
    flex: 1; overflow: hidden; position: relative;
    background:
      radial-gradient(circle at 1px 1px, var(--grid-dot, rgba(0,0,0,0.04)) 1px, transparent 0);
    background-size: 24px 24px;
  }
  :global([data-theme="dark"]) .tree-canvas {
    --grid-dot: rgba(255,255,255,0.04);
  }
  .tree-scroll {
    width: 100%; height: 100%;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 32px 40px 60px;
  }
  .tree-content {
    position: relative;
    flex-shrink: 0;
  }

  /* SVG Connections */
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
  .conn-suggested { stroke: #86efac; stroke-dasharray: 4 4; }
  .conn-hierarchy { stroke: var(--conn-hier, #c4b5a0); stroke-opacity: 0.5; }
  :global([data-theme="dark"]) .conn-hierarchy { --conn-hier: #6b6152; }

  /* Skill Nodes */
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

  /* Detail Panel */
  .detail-panel {
    position: absolute; right: 0; top: 0;
    width: 280px; height: 100%;
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
  .type-suggested { background: rgba(134,239,172,0.15); color: #16a34a; }

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

  /* Community Section */
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
</style>
