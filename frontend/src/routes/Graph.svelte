<script lang="ts">
  import { getGraph, getTagTree } from '../lib/api';
  import type { GraphData, GraphNode, GraphEdge, TagTreeEntry } from '../lib/types';

  let loading = $state(true);
  let nodes = $state<GraphNode[]>([]);
  let edges = $state<GraphEdge[]>([]);
  let tree = $state<TagTreeEntry[]>([]);

  // Build structured category data
  let categories = $derived.by(() => {
    if (loading) return [];

    const nodeMap = new Map(nodes.map(n => [n.id, n]));

    // Build parent->children map from tag tree
    const childrenOf = new Map<string, string[]>();
    const hasParent = new Set<string>();
    for (const e of tree) {
      const arr = childrenOf.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      childrenOf.set(e.parent_tag, arr);
      hasParent.add(e.child_tag);
    }

    // Find root categories (nodes that are parents but have no parent themselves)
    const roots: string[] = [];
    for (const [parent] of childrenOf) {
      if (!hasParent.has(parent)) {
        roots.push(parent);
      }
    }

    // Build recursive tree structure
    type TreeNode = { id: string; name: string; lit: boolean; children: TreeNode[] };
    function buildTree(id: string): TreeNode | null {
      const node = nodeMap.get(id);
      if (!node) return null;
      const childIds = childrenOf.get(id) || [];
      const children = childIds.map(c => buildTree(c)).filter((c): c is TreeNode => c !== null);
      return { id: node.id, name: node.name, lit: node.lit, children };
    }

    const cats = roots.map(r => buildTree(r)).filter((c): c is TreeNode => c !== null);

    // Orphan nodes (not in any tree)
    const inTree = new Set<string>();
    function collectIds(t: TreeNode) { inTree.add(t.id); t.children.forEach(collectIds); }
    cats.forEach(collectIds);

    const orphans = nodes
      .filter(n => !inTree.has(n.id))
      .map(n => ({ id: n.id, name: n.name, lit: n.lit, children: [] as TreeNode[] }));

    if (orphans.length > 0) {
      cats.push({ id: '__other', name: 'Other', lit: false, children: orphans });
    }

    return cats;
  });

  // Prerequisite edges for display
  let prereqInfo = $derived.by(() => {
    const nodeMap = new Map(nodes.map(n => [n.id, n]));
    return edges.map(e => ({
      from: nodeMap.get(e.from)?.name || e.from,
      to: nodeMap.get(e.to)?.name || e.to,
      type: e.type,
    }));
  });

  $effect(() => {
    Promise.all([getGraph(), getTagTree()]).then(([data, t]) => {
      nodes = data.nodes;
      edges = data.edges;
      tree = t;
      loading = false;
    });
  });
</script>

<div class="graph-page">
  <h1>Knowledge Map</h1>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else}
    <div class="categories">
      {#each categories as cat}
        <div class="category">
          <div class="category-header">
            <span class="category-name">{cat.name}</span>
          </div>
          <div class="category-body">
            {#if cat.children.length === 0}
              <a href="#/tag?id={encodeURIComponent(cat.id)}" class="tag-node" class:lit={cat.lit}>
                {cat.name}
              </a>
            {:else}
              {#each cat.children as sub}
                {#if sub.children.length > 0}
                  <div class="subcategory">
                    <a href="#/tag?id={encodeURIComponent(sub.id)}" class="sub-header" class:lit={sub.lit}>
                      {sub.name}
                    </a>
                    <div class="sub-tags">
                      {#each sub.children as leaf}
                        <a href="#/tag?id={encodeURIComponent(leaf.id)}" class="tag-node" class:lit={leaf.lit}>
                          {leaf.name}
                        </a>
                      {/each}
                    </div>
                  </div>
                {:else}
                  <a href="#/tag?id={encodeURIComponent(sub.id)}" class="tag-node" class:lit={sub.lit}>
                    {sub.name}
                  </a>
                {/if}
              {/each}
            {/if}
          </div>
        </div>
      {/each}
    </div>

    {#if prereqInfo.length > 0}
      <div class="prereq-section">
        <h2>Prerequisite Relationships</h2>
        <div class="prereq-list">
          {#each prereqInfo as p}
            <div class="prereq-edge">
              <span class="prereq-from">{p.from}</span>
              <span class="prereq-arrow {p.type}">&rarr;</span>
              <span class="prereq-to">{p.to}</span>
              <span class="prereq-type {p.type}">{p.type}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="legend">
      <span class="tag-node lit" style="font-size:12px;padding:2px 8px">mastered</span>
      <span class="tag-node" style="font-size:12px;padding:2px 8px">unlearned</span>
      <span class="prereq-type required">required</span>
      <span class="prereq-type recommended">recommended</span>
      <span class="prereq-type suggested">suggested</span>
    </div>
  {/if}
</div>

<style>
  .graph-page {
    max-width: 960px;
    margin: 0 auto;
    padding: 0 1rem 3rem;
  }

  .categories {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1.25rem;
    margin-top: 1rem;
  }

  .category {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .category-header {
    padding: 10px 16px;
    background: rgba(0,0,0,0.02);
    border-bottom: 1px solid var(--border);
  }
  .category-name {
    font-family: var(--font-serif);
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .category-body {
    padding: 12px 16px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .subcategory {
    width: 100%;
    margin-bottom: 4px;
  }
  .sub-header {
    display: inline-block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
    text-decoration: none;
    padding: 2px 8px;
    border-radius: 3px;
    transition: background 0.1s, color 0.1s;
  }
  .sub-header:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    text-decoration: none;
  }
  .sub-header.lit {
    color: var(--accent);
  }
  .sub-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    padding-left: 8px;
  }

  .tag-node {
    display: inline-block;
    padding: 3px 10px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 12px;
    color: var(--text-secondary);
    background: var(--bg-white);
    text-decoration: none;
    transition: all 0.15s;
    cursor: pointer;
  }
  .tag-node:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .tag-node.lit {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .tag-node.lit:hover {
    opacity: 0.85;
    color: white;
  }

  .prereq-section {
    margin-top: 2.5rem;
  }
  .prereq-section h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.1rem;
    padding-bottom: 0.25em;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.75rem;
  }
  .prereq-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .prereq-edge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .prereq-arrow {
    font-size: 14px;
  }
  .prereq-arrow.required { color: #dc2626; }
  .prereq-arrow.recommended { color: #d97706; }
  .prereq-arrow.suggested { color: #16a34a; }
  .prereq-type {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 8px;
    margin-left: auto;
  }
  .prereq-type.required { color: #dc2626; background: #fef2f2; }
  .prereq-type.recommended { color: #d97706; background: #fffbeb; }
  .prereq-type.suggested { color: #16a34a; background: #f0fdf4; }

  .legend {
    margin-top: 2rem;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-hint);
  }
</style>
