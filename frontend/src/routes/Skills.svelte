<script lang="ts">
  import cytoscape from 'cytoscape';
  // @ts-ignore
  import dagre from 'cytoscape-dagre';
  import { getGraph, getTagTree, listSkills, lightSkill, unlightSkill, listTags, createTagInline } from '../lib/api';
  import type { GraphNode, GraphEdge, TagTreeEntry, Tag } from '../lib/types';

  cytoscape.use(dagre);

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
    // Create tag if it doesn't exist
    if (!allTags.some(t => t.id === tagId)) {
      const newTag = await createTagInline(tagId, tagName || tagId);
      allTags = [...allTags, newTag];
    }
    // Light as learning
    if (!skillMap.has(tagId)) {
      await lightSkill(tagId, 'learning');
      skillMap.set(tagId, 'learning');
      skillMap = new Map(skillMap);
    }
    searchQuery = '';
    searchOpen = false;
    // Rebuild graph to include new standalone node
    buildGraph();
  }

  // Track which compound nodes are expanded
  let expandedSet = new Set<string>();

  function buildGraph() {
    if (!containerEl) return;

    const nodeMap = new Map(graphNodes.map(n => [n.id, n]));

    // Build parent->children map and compute depths
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

    // Compute depth for each node
    const depthOf = new Map<string, number>();
    function setDepth(id: string, d: number) {
      if (depthOf.has(id) && depthOf.get(id)! <= d) return;
      depthOf.set(id, d);
      for (const ch of (childrenOf.get(id) || [])) setDepth(ch, d + 1);
    }
    for (const r of roots) setDepth(r, 0);

    // Direct parent in tree
    const directParent = new Map<string, string>();
    for (const e of tree) {
      directParent.set(e.child_tag, e.parent_tag);
    }

    // Collect all descendant leaf count for compound label
    function leafCount(id: string): number {
      const ch = childrenOf.get(id);
      if (!ch || ch.length === 0) return 1;
      let sum = 0;
      for (const c of ch) sum += leafCount(c);
      return sum;
    }

    // Determine visible nodes:
    // - Roots (depth 0) always shown as compound
    // - Depth 1 (course-level) shown as nodes (collapsed compound if they have children)
    // - Depth 2+ hidden unless parent is expanded
    function isVisible(id: string): boolean {
      const d = depthOf.get(id) ?? 99;
      if (d <= 1) return true;
      // Visible if its direct parent is expanded
      const p = directParent.get(id);
      if (p && expandedSet.has(p)) return true;
      return false;
    }

    // Collect all IDs (include standalone skilled tags not in tree)
    const allIds = new Set<string>();
    for (const n of graphNodes) allIds.add(n.id);
    for (const e of tree) { allIds.add(e.parent_tag); allIds.add(e.child_tag); }
    for (const [tagId] of skillMap) allIds.add(tagId);

    const elements: cytoscape.ElementDefinition[] = [];

    for (const id of allIds) {
      if (!isVisible(id)) continue;

      const gn = nodeMap.get(id);
      const name = gn?.name || id;
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
        id,
        label,
        lit,
        learning,
        compound,
        isRoot,
        depth,
        collapsed,
        hasChildren,
      };

      // Set parent for compound nesting (only if parent is visible and expanded)
      if (parent && isCompound.has(parent) && (depthOf.get(parent) === 0 || expandedSet.has(parent))) {
        data.parent = parent;
      }

      elements.push({ data });
    }

    // Prereq edges — only between visible nodes
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
        // --- Leaf / collapsed nodes ---
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
        // Collapsed compound (has children but not expanded) — show with special style
        {
          selector: 'node[?collapsed]',
          style: {
            'border-style': 'double' as any,
            'border-width': 3,
            'border-color': '#9ca3af',
            'font-weight': 'bold' as any,
          } as any,
        },
        // Learning leaf (amber)
        {
          selector: 'node[!compound][?learning]',
          style: {
            'background-color': '#f59e0b',
            'border-color': '#d97706',
            'color': '#ffffff',
          },
        },
        // Mastered leaf (green)
        {
          selector: 'node[!compound][?lit][!learning]',
          style: {
            'background-color': '#5f9b65',
            'border-color': '#4a8a50',
            'color': '#ffffff',
          },
        },
        // --- Compound parent nodes (expanded containers) ---
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
        // Root compound
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
        // Lit compound
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
        // --- Edges ---
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

    // Single click on leaf: toggle skill
    cy.on('tap', 'node[!compound][!hasChildren]', async (evt) => {
      await toggleSkill(evt.target.id());
    });

    // Single click on collapsed compound: expand it
    cy.on('tap', 'node[?collapsed]', (evt) => {
      expandedSet.add(evt.target.id());
      buildGraph();
    });

    // Single click on expanded compound label area: collapse it
    cy.on('tap', 'node[?compound][depth > 0]', (evt) => {
      expandedSet.delete(evt.target.id());
      buildGraph();
    });

    // Double-click: navigate to tag page
    cy.on('dbltap', 'node', (evt) => {
      const nodeId = evt.target.id();
      window.location.hash = `#/tag?id=${encodeURIComponent(nodeId)}`;
    });

    cy.fit(undefined, 50);
  }

  async function toggleSkill(tagId: string) {
    const current = skillMap.get(tagId);
    if (!current) {
      // unlit → learning
      await lightSkill(tagId, 'learning');
      skillMap.set(tagId, 'learning');
    } else if (current === 'learning') {
      // learning → mastered
      await lightSkill(tagId, 'mastered');
      const s = await listSkills();
      skillMap = new Map(s.map(sk => [sk.tag_id, sk.status]));
    } else {
      // mastered → unlit
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
    if (!loading && containerEl) {
      buildGraph();
    }
  });
</script>

<div class="skills-page">
  <div class="skills-toolbar">
    <div class="toolbar-left">
      <h1>Skills</h1>
      <span class="lit-count">已掌握 <strong>{masteredCount}</strong> · 学习中 <strong>{learningCount}</strong></span>
    </div>
    <div class="toolbar-right">
      <div class="legend-inline">
        <span class="legend-dot mastered"></span> 已掌握
        <span class="legend-dot learning"></span> 学习中
        <span class="legend-dot unlit"></span> 未学习
        <span class="legend-box"></span> 可展开
      </div>
      <span class="toolbar-hint">单击切换状态 &middot; 双击进入标签</span>
    </div>
  </div>

  {#if loading}
    <div class="graph-loading">
      <p class="meta">Loading...</p>
    </div>
  {:else}
    <div class="graph-container" bind:this={containerEl}></div>
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
    align-items: baseline;
    gap: 12px;
  }
  .toolbar-left h1 {
    font-size: 1.2rem;
    margin: 0;
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
</style>
