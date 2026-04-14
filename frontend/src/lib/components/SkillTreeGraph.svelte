<script lang="ts">
  /**
   * Reusable DAG visualization for skill trees.
   * Renders hierarchy as sub-group boxes with nodes inside,
   * and prereq edges as curved arrows (within and across groups).
   */
  import { tagName as resolveTagName } from '../display';

  interface TreeEdge { parent_tag: string; child_tag: string }
  interface PrereqEdge { from_tag: string; to_tag: string; prereq_type: string }

  let {
    edges,
    prereqs,
    tagNamesMap = {},
    tagNamesI18n = {},
  }: {
    edges: TreeEdge[];
    prereqs: PrereqEdge[];
    tagNamesMap: Record<string, string>;
    tagNamesI18n: Record<string, Record<string, string>>;
  } = $props();

  // --- Name resolution ---
  function resolveName(id: string): string {
    const i18n = tagNamesI18n[id];
    const name = tagNamesMap[id];
    return i18n ? resolveTagName(i18n, name || id, id) : (name || id);
  }

  // --- Hierarchy ---
  let childrenOf = $derived.by(() => {
    const map = new Map<string, string[]>();
    const hasParent = new Set<string>();
    for (const e of edges) {
      const arr = map.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      map.set(e.parent_tag, arr);
      hasParent.add(e.child_tag);
    }
    return { map, hasParent };
  });

  let roots = $derived(
    [...new Set(edges.map(e => e.parent_tag))].filter(p => !childrenOf.hasParent.has(p))
  );

  function collectAllLeaves(groupId: string): Set<string> {
    const result = new Set<string>();
    const stack = [...(childrenOf.map.get(groupId) || [])];
    while (stack.length > 0) {
      const id = stack.pop()!;
      const ch = childrenOf.map.get(id) || [];
      if (ch.length === 0) result.add(id);
      else for (const c of ch) stack.push(c);
    }
    return result;
  }

  function collectDirectLeaves(groupId: string): Set<string> {
    const result = new Set<string>();
    for (const ch of (childrenOf.map.get(groupId) || [])) {
      if (!(childrenOf.map.get(ch) || []).length) result.add(ch);
    }
    return result;
  }

  function getSubGroups(groupId: string): string[] {
    return (childrenOf.map.get(groupId) || []).filter(ch => (childrenOf.map.get(ch) || []).length > 0);
  }

  function findSubGroup(nodeId: string, rootId: string): string | null {
    for (const child of (childrenOf.map.get(rootId) || [])) {
      if (child === nodeId) return null;
      if (collectAllLeaves(child).has(nodeId)) return child;
    }
    return null;
  }

  // --- Layout ---
  const NODE_H = 44;
  const H_GAP = 20;
  const V_GAP = 48;

  function measureNodeWidth(name: string): number {
    let units = 0;
    for (const ch of name) {
      const cp = ch.codePointAt(0) ?? 0;
      units += (cp >= 0x1100 && cp <= 0xFFEF) || (cp >= 0x20000 && cp <= 0x2FA1F) ? 2 : 1;
    }
    return Math.max(60, Math.min(180, Math.round(16 + units * 7.5 + 20)));
  }

  interface LayoutNode { id: string; name: string; x: number; y: number; w: number }
  interface LayoutConn { from: string; to: string; type: string; x1: number; y1: number; x2: number; y2: number }
  interface GroupLayout { nodes: LayoutNode[]; conns: LayoutConn[]; w: number; h: number; count: number }

  function computeLayout(rootId: string, deep: boolean = false): GroupLayout {
    const fieldNodes = deep ? collectAllLeaves(rootId) : collectDirectLeaves(rootId);
    if (fieldNodes.size === 0) return { nodes: [], conns: [], w: 0, h: 0, count: 0 };

    const nodeWidths = new Map<string, number>();
    for (const id of fieldNodes) nodeWidths.set(id, measureNodeWidth(resolveName(id)));

    const localEdges = prereqs.filter(e => fieldNodes.has(e.from_tag) && fieldNodes.has(e.to_tag));

    // Topological sort
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
      frontier.sort((a, b) => {
        const ga = findSubGroup(a, rootId) || '';
        const gb = findSubGroup(b, rootId) || '';
        return ga !== gb ? ga.localeCompare(gb) : resolveName(a).localeCompare(resolveName(b));
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

    const positions = new Map<string, { x: number; y: number }>();
    const rowWidths = byDepth.map(row =>
      row.reduce((s, id) => s + (nodeWidths.get(id) ?? 100), 0) + Math.max(0, row.length - 1) * H_GAP
    );
    const maxRowW = Math.max(0, ...rowWidths);

    for (let row = 0; row < byDepth.length; row++) {
      const rowW = rowWidths[row];
      let curX = (maxRowW - rowW) / 2;
      for (const id of byDepth[row]) {
        positions.set(id, { x: curX, y: row * (NODE_H + V_GAP) });
        curX += (nodeWidths.get(id) ?? 100) + H_GAP;
      }
    }

    const nodes: LayoutNode[] = [...positions.entries()].map(([id, pos]) => ({
      id, name: resolveName(id), x: pos.x, y: pos.y, w: nodeWidths.get(id) ?? 100,
    }));

    const conns: LayoutConn[] = localEdges
      .filter(e => positions.has(e.from_tag) && positions.has(e.to_tag))
      .map(e => {
        const fp = positions.get(e.from_tag)!;
        const tp = positions.get(e.to_tag)!;
        const fw = nodeWidths.get(e.from_tag) ?? 100;
        const tw = nodeWidths.get(e.to_tag) ?? 100;
        return { from: e.from_tag, to: e.to_tag, type: e.prereq_type, x1: fp.x + fw / 2, y1: fp.y + NODE_H, x2: tp.x + tw / 2, y2: tp.y };
      });

    return { nodes, conns, w: maxRowW || 100, h: byDepth.length * (NODE_H + V_GAP) || NODE_H, count: fieldNodes.size };
  }

  function connPath(c: LayoutConn): string {
    const my = (c.y1 + c.y2) / 2;
    return `M${c.x1},${c.y1} C${c.x1},${my} ${c.x2},${my} ${c.x2},${c.y2}`;
  }

  // --- Compute all group layouts ---
  let activeField = $state('');
  $effect(() => { if (roots.length > 0 && !activeField) activeField = roots[0]; });

  let activeSubGroups = $derived(activeField ? getSubGroups(activeField) : []);

  let groupLayouts = $derived.by(() => {
    const map = new Map<string, GroupLayout>();
    if (!activeField) return map;
    const direct = computeLayout(activeField);
    if (direct.nodes.length > 0) map.set(activeField, direct);
    for (const sub of activeSubGroups) {
      const layout = computeLayout(sub, true);
      if (layout.nodes.length > 0) map.set(sub, layout);
    }
    return map;
  });

  // --- Cross-group arrows ---
  let scrollEl = $state<HTMLElement | null>(null);
  let groupEls = $state(new Map<string, HTMLElement>());
  let nodeEls = $state(new Map<string, HTMLElement>());

  interface CrossConn { from: string; to: string; type: string; x1: number; y1: number; x2: number; y2: number }
  let crossConns = $state<CrossConn[]>([]);

  // Identify which root group a tag belongs to
  let tagToRoot = $derived.by(() => {
    const m = new Map<string, string>();
    if (!activeField) return m;
    const directLeaves = collectDirectLeaves(activeField);
    for (const l of directLeaves) m.set(l, activeField);
    for (const sub of activeSubGroups) {
      for (const l of collectAllLeaves(sub)) m.set(l, sub);
    }
    return m;
  });

  let crossGroupEdges = $derived(
    prereqs.filter(e => {
      const rA = tagToRoot.get(e.from_tag), rB = tagToRoot.get(e.to_tag);
      return rA && rB && rA !== rB;
    })
  );

  function recomputeCrossConns() {
    if (!scrollEl || crossGroupEdges.length === 0) { crossConns = []; return; }
    const containerRect = scrollEl.getBoundingClientRect();
    const result: CrossConn[] = [];
    for (const e of crossGroupEdges) {
      const fromEl = nodeEls.get(e.from_tag), toEl = nodeEls.get(e.to_tag);
      if (!fromEl || !toEl) continue;
      const fr = fromEl.getBoundingClientRect(), tr = toEl.getBoundingClientRect();
      result.push({
        from: e.from_tag, to: e.to_tag, type: e.prereq_type,
        x1: fr.left + fr.width / 2 - containerRect.left + scrollEl.scrollLeft,
        y1: fr.top + fr.height - containerRect.top + scrollEl.scrollTop,
        x2: tr.left + tr.width / 2 - containerRect.left + scrollEl.scrollLeft,
        y2: tr.top - containerRect.top + scrollEl.scrollTop,
      });
    }
    crossConns = result;
  }

  $effect(() => {
    const _gl = groupLayouts;
    const _af = activeField;
    const _cge = crossGroupEdges;
    const id = requestAnimationFrame(recomputeCrossConns);
    return () => cancelAnimationFrame(id);
  });

  function trackGroup(el: HTMLElement, rootId: string) {
    groupEls.set(rootId, el);
    return { destroy() { groupEls.delete(rootId); } };
  }
  function trackNode(el: HTMLElement, nodeId: string) {
    nodeEls.set(nodeId, el);
    return { destroy() { nodeEls.delete(nodeId); } };
  }

  function crossConnPath(c: CrossConn): string {
    const dx = Math.abs(c.x2 - c.x1);
    const dy = Math.abs(c.y2 - c.y1);
    const bend = Math.max(40, Math.min(dy * 0.4, dx * 0.5));
    return `M${c.x1},${c.y1} C${c.x1},${c.y1 + bend} ${c.x2},${c.y2 - bend} ${c.x2},${c.y2}`;
  }
</script>

{#if edges.length === 0}
  <div class="empty">No skill tree data</div>
{:else}
  <!-- Field tabs -->
  <div class="field-tabs">
    {#each roots as root (root)}
      {@const leaves = collectAllLeaves(root)}
      <button class="field-tab" class:active={activeField === root} onclick={() => activeField = root}>
        {resolveName(root)}
        <span class="tab-count">{leaves.size}</span>
      </button>
    {/each}
  </div>

  <div class="graph-content" bind:this={scrollEl} onscroll={recomputeCrossConns}>
    <div class="subgroup-row">
      {#each [...groupLayouts.entries()] as [boxId, layout] (boxId)}
        <div class="subgroup-box" use:trackGroup={boxId}>
          <div class="subgroup-header">
            <span class="subgroup-name">{resolveName(boxId)}</span>
            <span class="subgroup-count">{layout.count}</span>
          </div>
          {#if layout.nodes.length > 0}
            <div class="subgroup-canvas">
              <div class="subgroup-content" style="min-width:{layout.w}px; min-height:{layout.h + 16}px;">
                <svg class="conn-svg" style="width:{layout.w}px; height:{layout.h + 16}px;">
                  <defs>
                    <marker id="garr-req-{boxId}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto">
                      <path d="M0,0 L10,5 L0,10z" fill="#ef4444"/>
                    </marker>
                    <marker id="garr-rec-{boxId}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto">
                      <path d="M0,0 L10,5 L0,10z" fill="#f59e0b"/>
                    </marker>
                  </defs>
                  {#each layout.conns as c}
                    <path d={connPath(c)} class="conn conn-{c.type}" marker-end={c.type === 'required' ? `url(#garr-req-${boxId})` : `url(#garr-rec-${boxId})`} />
                  {/each}
                </svg>

                {#each layout.nodes as node (node.id)}
                  <a
                    href="/tag?id={encodeURIComponent(node.id)}"
                    class="skill-node"
                    style="left:{node.x}px; top:{node.y}px; width:{node.w}px; height:{NODE_H}px;"
                    use:trackNode={node.id}
                  >
                    <span class="node-name">{node.name}</span>
                  </a>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Cross-box arrows -->
    {#if crossConns.length > 0}
      <svg class="cross-conn-svg" aria-hidden="true">
        <defs>
          <marker id="gxarr-req" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto">
            <path d="M0,0 L10,5 L0,10z" fill="#ef4444"/>
          </marker>
          <marker id="gxarr-rec" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto">
            <path d="M0,0 L10,5 L0,10z" fill="#f59e0b"/>
          </marker>
        </defs>
        {#each crossConns as c}
          <path d={crossConnPath(c)} class="conn conn-{c.type} cross-conn" marker-end={c.type === 'required' ? 'url(#gxarr-req)' : 'url(#gxarr-rec)'} />
        {/each}
      </svg>
    {/if}
  </div>
{/if}

<style>
  .empty { padding: 2rem; text-align: center; color: var(--text-hint); font-size: 14px; }

  .field-tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 1rem; flex-wrap: wrap; }
  .field-tab { padding: 8px 14px; font-size: 13px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-secondary); cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .field-tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .tab-count { font-size: 11px; background: var(--border); color: var(--text-hint); padding: 1px 5px; border-radius: 8px; }

  .graph-content { position: relative; overflow: auto; padding: 8px; }

  .subgroup-row { display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start; }
  .subgroup-box { border: 1px solid var(--border); border-radius: 6px; background: var(--bg-white); min-width: 160px; }
  .subgroup-header { padding: 8px 12px; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 8px; }
  .subgroup-name { font-size: 14px; font-weight: 500; color: var(--text-primary); }
  .subgroup-count { font-size: 11px; color: var(--text-hint); background: var(--bg-page); padding: 1px 5px; border-radius: 8px; }

  .subgroup-canvas { overflow: auto; padding: 12px; }
  .subgroup-content { position: relative; }

  .conn-svg { position: absolute; top: 0; left: 0; pointer-events: none; overflow: visible; }
  .conn { fill: none; stroke-width: 2; }
  .conn-required { stroke: #ef4444; stroke-dasharray: 8 4; }
  .conn-recommended { stroke: #f59e0b; stroke-dasharray: 6 4; }

  .cross-conn-svg { position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; overflow: visible; z-index: 8; }
  .cross-conn { opacity: 0.5; stroke-width: 1.5; }

  .skill-node {
    position: absolute;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1.5px solid var(--accent);
    border-radius: 6px;
    background: var(--bg-white);
    cursor: pointer;
    text-decoration: none;
    color: var(--text-primary);
    transition: border-color 0.15s, background 0.15s;
    font-size: 13px;
    padding: 0 8px;
  }
  .skill-node:hover { border-color: var(--accent); background: rgba(95, 155, 101, 0.06); text-decoration: none; }
  .node-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
