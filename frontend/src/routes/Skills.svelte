<script lang="ts">
  import cytoscape from 'cytoscape';
  // @ts-ignore
  import dagre from 'cytoscape-dagre';
  import { getGraph, getTagTree, listSkills, lightSkill, unlightSkill, listTags, createTagInline, listSkillTrees, adoptSkillTree, castVote } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t } from '../lib/i18n';
  import { authorName, tagName as resolveTagName } from '../lib/display';
  import type { GraphNode, GraphEdge, TagTreeEntry, Tag, SkillTree, SkillTreeEdge } from '../lib/types';

  cytoscape.use(dagre);

  // --- Tab state ---
  let activeTab = $state<'my' | 'community'>('my');

  // --- My Skills state ---
  let loading = $state(true);
  let graphNodes = $state<GraphNode[]>([]);
  let graphEdges = $state<GraphEdge[]>([]);
  let tree = $state<TagTreeEntry[]>([]);
  let skillMap = $state(new Map<string, 'mastered' | 'learning'>());
  let litSet = $derived(new Set(skillMap.keys()));
  let masteredCount = $derived([...skillMap.values()].filter(s => s === 'mastered').length);
  let learningCount = $derived([...skillMap.values()].filter(s => s === 'learning').length);

  let containerEl: HTMLDivElement | undefined = $state();
  let cy: cytoscape.Core | null = null;

  // Tag search
  let allTags = $state<Tag[]>([]);
  let searchQuery = $state('');
  let searchOpen = $state(false);
  let searchSuggestions = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return allTags.filter(t => t.id.toLowerCase().includes(q) || t.name.toLowerCase().includes(q)).slice(0, 8);
  });

  async function addTagSkill(tagId: string, tagName?: string) {
    if (!allTags.some(t => t.id === tagId)) {
      const newTag = await createTagInline(tagId, tagName || tagId);
      allTags = [...allTags, newTag];
    }
    if (!skillMap.has(tagId)) {
      await lightSkill(tagId, 'learning');
      skillMap.set(tagId, 'learning');
      skillMap = new Map(skillMap);
    }
    searchQuery = '';
    searchOpen = false;
    buildGraph();
  }

  // Track which compound nodes are expanded
  let expandedSet = new Set<string>();

  function buildGraph() {
    if (!containerEl) return;

    const nodeMap = new Map(graphNodes.map(n => [n.id, n]));

    const childrenOf = new Map<string, string[]>();
    const hasParent = new Set<string>();
    for (const e of tree) {
      const arr = childrenOf.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      childrenOf.set(e.parent_tag, arr);
      hasParent.add(e.child_tag);
    }

    const isCompound = new Set<string>();
    for (const [parent] of childrenOf) {
      isCompound.add(parent);
    }

    const roots = [...new Set(tree.map(e => e.parent_tag))].filter(p => !hasParent.has(p));

    const depthOf = new Map<string, number>();
    function setDepth(id: string, d: number) {
      if (depthOf.has(id) && depthOf.get(id)! <= d) return;
      depthOf.set(id, d);
      for (const ch of (childrenOf.get(id) || [])) setDepth(ch, d + 1);
    }
    for (const r of roots) setDepth(r, 0);

    const directParent = new Map<string, string>();
    for (const e of tree) {
      directParent.set(e.child_tag, e.parent_tag);
    }

    function leafCount(id: string): number {
      const ch = childrenOf.get(id);
      if (!ch || ch.length === 0) return 1;
      let sum = 0;
      for (const c of ch) sum += leafCount(c);
      return sum;
    }

    function isVisible(id: string): boolean {
      const d = depthOf.get(id) ?? 99;
      if (d <= 1) return true;
      const p = directParent.get(id);
      if (p && expandedSet.has(p)) return true;
      return false;
    }

    const allIds = new Set<string>();
    for (const n of graphNodes) allIds.add(n.id);
    for (const e of tree) { allIds.add(e.parent_tag); allIds.add(e.child_tag); }
    for (const [tagId] of skillMap) allIds.add(tagId);

    const elements: cytoscape.ElementDefinition[] = [];

    for (const id of allIds) {
      if (!isVisible(id)) continue;

      const gn = nodeMap.get(id);
      const name = gn ? resolveTagName(gn.names, gn.name, id) : id;
      const lit = skillMap.has(id);
      const learning = skillMap.get(id) === 'learning';
      const depth = depthOf.get(id) ?? 99;
      const isRoot = roots.includes(id);
      const parent = directParent.get(id);
      const compound = isCompound.has(id) && (depth === 0 || expandedSet.has(id));
      const hasChildren = isCompound.has(id);
      const collapsed = hasChildren && !expandedSet.has(id) && depth >= 1;
      const lc = hasChildren ? leafCount(id) : 0;

      const label = collapsed ? `${name} (${lc})` : name;

      const data: any = {
        id, label, lit, learning, compound, isRoot, depth, collapsed, hasChildren,
      };

      if (parent && isCompound.has(parent) && (depthOf.get(parent) === 0 || expandedSet.has(parent))) {
        data.parent = parent;
      }

      elements.push({ data });
    }

    const visibleIds = new Set(elements.map(e => e.data.id));
    for (const e of graphEdges) {
      if (visibleIds.has(e.from) && visibleIds.has(e.to)) {
        elements.push({
          data: {
            id: `prereq_${e.from}_${e.to}`,
            source: e.from,
            target: e.to,
            edgeType: e.type,
          },
        });
      }
    }

    if (cy) cy.destroy();

    cy = cytoscape({
      container: containerEl,
      elements,
      layout: {
        name: 'dagre',
        rankDir: 'TB',
        spacingFactor: 1.5,
        nodeSep: 40,
        rankSep: 50,
        animate: false,
      } as any,
      style: [
        {
          selector: 'node[!compound]',
          style: {
            'label': 'data(label)',
            'text-valign': 'center',
            'text-halign': 'center',
            'font-size': 14,
            'font-family': '"Liberation Sans", "Calibri", sans-serif',
            'width': 'label',
            'height': 40,
            'padding-left': '16px' as any,
            'padding-right': '16px' as any,
            'shape': 'round-rectangle',
            'background-color': '#f3f4f6',
            'border-width': 1.5,
            'border-color': '#d1d5db',
            'color': '#374151',
            'text-wrap': 'wrap',
            'text-max-width': '140px',
            'transition-property': 'background-color, border-color, color',
            'transition-duration': 200,
          } as any,
        },
        {
          selector: 'node[?collapsed]',
          style: {
            'border-style': 'double' as any,
            'border-width': 3,
            'border-color': '#9ca3af',
            'font-weight': 'bold' as any,
          } as any,
        },
        {
          selector: 'node[!compound][?learning]',
          style: {
            'background-color': '#f59e0b',
            'border-color': '#d97706',
            'color': '#ffffff',
          },
        },
        {
          selector: 'node[!compound][?lit][!learning]',
          style: {
            'background-color': '#5f9b65',
            'border-color': '#4a8a50',
            'color': '#ffffff',
          },
        },
        {
          selector: 'node[?compound]',
          style: {
            'label': 'data(label)',
            'text-valign': 'top',
            'text-halign': 'center',
            'font-size': 16,
            'font-weight': 'bold' as any,
            'font-family': '"Liberation Serif", "Palatino", Georgia, serif',
            'color': '#1f2937',
            'background-color': 'rgba(255,255,255,0.5)',
            'background-opacity': 0.5,
            'border-width': 2,
            'border-color': '#d1d5db',
            'border-style': 'solid' as any,
            'shape': 'round-rectangle',
            'padding': '28px' as any,
            'text-margin-y': -8,
            'compound-sizing-wrt-labels': 'include',
          } as any,
        },
        {
          selector: 'node[?isRoot][?compound]',
          style: {
            'font-size': 20,
            'border-color': '#9ca3af',
            'border-width': 2.5,
            'background-color': 'rgba(248,244,238,0.7)',
            'padding': '34px' as any,
          } as any,
        },
        {
          selector: 'node[?compound][?lit]',
          style: {
            'border-color': '#5f9b65',
            'background-color': 'rgba(95,155,101,0.08)',
          },
        },
        {
          selector: 'node:active',
          style: { 'overlay-opacity': 0.08 },
        },
        {
          selector: 'edge[edgeType="required"]',
          style: {
            'width': 2,
            'line-color': '#f87171',
            'line-style': 'dashed',
            'target-arrow-color': '#f87171',
            'target-arrow-shape': 'triangle',
            'arrow-scale': 0.8,
            'curve-style': 'bezier',
          },
        },
        {
          selector: 'edge[edgeType="recommended"]',
          style: {
            'width': 1.5,
            'line-color': '#fbbf24',
            'line-style': 'dashed',
            'target-arrow-color': '#fbbf24',
            'target-arrow-shape': 'triangle',
            'arrow-scale': 0.7,
            'curve-style': 'bezier',
          },
        },
        {
          selector: 'edge[edgeType="suggested"]',
          style: {
            'width': 1,
            'line-color': '#86efac',
            'line-style': 'dotted',
            'target-arrow-color': '#86efac',
            'target-arrow-shape': 'triangle',
            'arrow-scale': 0.6,
            'curve-style': 'bezier',
          },
        },
      ],
      minZoom: 0.15,
      maxZoom: 4,
      wheelSensitivity: 0.3,
      boxSelectionEnabled: false,
    });

    cy.on('tap', 'node[!compound][!hasChildren]', async (evt) => {
      await toggleSkill(evt.target.id());
    });

    cy.on('tap', 'node[?collapsed]', (evt) => {
      expandedSet.add(evt.target.id());
      buildGraph();
    });

    cy.on('tap', 'node[?compound][depth > 0]', (evt) => {
      expandedSet.delete(evt.target.id());
      buildGraph();
    });

    cy.on('dbltap', 'node', (evt) => {
      const nodeId = evt.target.id();
      window.location.hash = `#/tag?id=${encodeURIComponent(nodeId)}`;
    });

    cy.fit(undefined, 50);
  }

  async function toggleSkill(tagId: string) {
    const current = skillMap.get(tagId);
    if (!current) {
      await lightSkill(tagId, 'learning');
      skillMap.set(tagId, 'learning');
    } else if (current === 'learning') {
      await lightSkill(tagId, 'mastered');
      const s = await listSkills();
      skillMap = new Map(s.map(sk => [sk.tag_id, sk.status]));
    } else {
      await unlightSkill(tagId);
      skillMap.delete(tagId);
    }
    skillMap = new Map(skillMap);
    if (cy) {
      cy.nodes().forEach(n => {
        n.data('lit', skillMap.has(n.id()));
        n.data('learning', skillMap.get(n.id()) === 'learning');
      });
    }
  }

  $effect(() => {
    Promise.all([getGraph(), getTagTree(), listSkills()]).then(([data, t, sk]) => {
      graphNodes = data.nodes;
      graphEdges = data.edges;
      tree = t;
      skillMap = new Map(sk.map(s => [s.tag_id, s.status]));
      loading = false;
    });
  });

  $effect(() => {
    if (!loading && containerEl && activeTab === 'my') {
      buildGraph();
    }
  });

  // --- Community skill trees state ---

  let treesLoading = $state(true);
  let communityTrees = $state<SkillTree[]>([]);
  let filterField = $state('');
  let isLoggedIn = $derived(!!getAuth());
  let filteredTrees = $derived(
    filterField ? communityTrees.filter(t => t.tag_id === filterField) : communityTrees
  );

  // Dynamic field list from trees
  let availableFields = $derived.by(() => {
    const fieldMap = new Map<string, { id: string; name: string; count: number }>();
    for (const tr of communityTrees) {
      if (!tr.tag_id) continue;
      const existing = fieldMap.get(tr.tag_id);
      if (existing) {
        existing.count++;
      } else {
        const name = tr.tag_names ? resolveTagName(tr.tag_names, tr.tag_name || tr.tag_id, tr.tag_id) : (tr.tag_name || tr.tag_id);
        fieldMap.set(tr.tag_id, { id: tr.tag_id, name, count: 1 });
      }
    }
    return [...fieldMap.values()].sort((a, b) => b.count - a.count);
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
</script>

<div class="skills-page">
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
      {#if activeTab === 'my'}
        <span class="lit-count">{t('skills.mastered')} <strong>{masteredCount}</strong> · {t('skills.learning')} <strong>{learningCount}</strong></span>
      {/if}
    </div>
    <div class="toolbar-right">
      {#if activeTab === 'my'}
        <div class="legend-inline">
          <span class="legend-dot mastered"></span> {t('skills.mastered')}
          <span class="legend-dot learning"></span> {t('skills.learning')}
          <span class="legend-dot unlit"></span> {t('skills.unlearned')}
          <span class="legend-box"></span> {t('skills.expandable')}
        </div>
        <span class="toolbar-hint">{t('skills.statusHint')}</span>
      {:else if isLoggedIn}
        <a href="#/skill-tree/new" class="create-btn">{t('skills.createTree')}</a>
      {/if}
    </div>
  </div>

  {#if activeTab === 'my'}
    {#if loading}
      <div class="graph-loading">
        <p class="meta">Loading...</p>
      </div>
    {:else}
      <div class="graph-container" bind:this={containerEl}></div>
    {/if}
  {:else}
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
          {#if isLoggedIn}
            <a href="#/skill-tree/new">{t('skills.createFirst')}</a>
          {/if}
        </div>
      {:else}
        <div class="tree-list">
          {#each filteredTrees as tree (tree.at_uri)}
            <div class="tree-card">
              <div class="tree-main">
                <a href="#/skill-tree?uri={encodeURIComponent(tree.at_uri)}" class="tree-title">{tree.title}</a>
                {#if tree.tag_id}
                  <span class="field-badge">{tree.tag_names ? resolveTagName(tree.tag_names, tree.tag_name || tree.tag_id, tree.tag_id) : (tree.tag_name || tree.tag_id)}</span>
                {/if}
                {#if tree.forked_from}
                  <span class="forked-badge">Fork</span>
                {/if}
                {#if tree.description}
                  <p class="tree-desc">{tree.description}</p>
                {/if}
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

  .skills-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .lit-count {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .toolbar-hint {
    font-size: 12px;
    color: var(--text-hint);
  }

  /* Tabs */
  .tab-bar {
    display: flex;
    gap: 0;
  }
  .tab {
    padding: 6px 16px;
    font-size: 14px;
    border: 1px solid var(--border);
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .tab:first-child {
    border-radius: 4px 0 0 4px;
  }
  .tab:last-child {
    border-radius: 0 4px 4px 0;
    border-left: none;
  }
  .tab.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .tab:hover:not(.active) {
    border-color: var(--accent);
    color: var(--accent);
  }

  .legend-inline {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
  }
  .legend-dot.mastered { background: #5f9b65; }
  .legend-dot.learning { background: #f59e0b; }
  .legend-dot.unlit { background: #e5e7eb; border: 1px solid #d1d5db; }
  .legend-box {
    width: 14px;
    height: 10px;
    display: inline-block;
    border: 2px double #9ca3af;
    border-radius: 2px;
    margin-left: 4px;
  }

  .graph-loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .graph-container {
    flex: 1;
    background: var(--bg-page);
  }

  /* Community section */
  .community-section {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }
  .subtitle {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 16px;
  }
  .create-btn {
    padding: 6px 16px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border-radius: 4px;
    text-decoration: none;
    transition: opacity 0.15s;
  }
  .create-btn:hover { opacity: 0.85; text-decoration: none; }
  .field-filter {
    display: flex;
    gap: 6px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .filter-btn {
    padding: 4px 12px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .filter-btn:hover { border-color: var(--accent); color: var(--accent); }
  .filter-btn.active { background: var(--accent); color: white; border-color: var(--accent); }
  .tree-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tree-card {
    display: flex;
    gap: 16px;
    padding: 16px 20px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    transition: border-color 0.15s;
  }
  .tree-card:hover { border-color: var(--border-strong); }
  .tree-main { flex: 1; min-width: 0; }
  .tree-title {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    color: var(--text-primary);
    text-decoration: none;
  }
  .tree-title:hover { color: var(--accent); }
  .forked-badge {
    font-size: 11px;
    background: var(--bg-gray, #f0f0f0);
    color: var(--text-hint);
    padding: 1px 6px;
    border-radius: 3px;
    margin-left: 6px;
  }
  .field-badge {
    font-size: 11px;
    background: rgba(95,155,101,0.12);
    color: var(--accent);
    padding: 1px 8px;
    border-radius: 3px;
    margin-left: 6px;
  }
  .tree-desc {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 4px 0 0;
    line-height: 1.5;
  }
  .tree-meta {
    display: flex;
    gap: 12px;
    margin-top: 8px;
    font-size: 13px;
    color: var(--text-hint);
  }
  .tree-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .vote-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .vote-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 12px;
    padding: 2px 4px;
    transition: color 0.15s;
  }
  .vote-btn:hover:not(:disabled) { color: var(--accent); }
  .vote-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .score {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .adopt-btn {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    background: none;
    color: var(--accent);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .adopt-btn:hover:not(:disabled) { background: var(--accent); color: white; }
  .adopt-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .empty { color: var(--text-hint); }
</style>
