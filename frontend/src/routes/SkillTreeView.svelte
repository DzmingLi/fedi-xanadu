<script lang="ts">
  import { getSkillTree, forkSkillTree, adoptSkillTree, addSkillTreeEdge, removeSkillTreeEdge, castVote, listTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { tagName as resolveTagName } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { untrack } from 'svelte';
  import type { SkillTreeDetail, SkillTreeEdge, Tag } from '../lib/types';

  let { uri } = $props<{ uri: string }>();

  let detail = $state<SkillTreeDetail | null>(null);
  let loading = $state(true);
  let allTags = $state<Tag[]>([]);
  let isLoggedIn = $derived(!!getAuth());
  let isOwner = $derived(isLoggedIn && detail?.tree.did === getAuth()?.did);

  // Collapse state
  let collapsed = $state(new Set<string>());

  function toggleCollapse(id: string) {
    if (collapsed.has(id)) {
      collapsed.delete(id);
    } else {
      collapsed.add(id);
    }
    collapsed = new Set(collapsed);
  }

  // Edit mode
  let newParent = $state('');
  let newChild = $state('');
  let tagSuggestions = $state<Tag[]>([]);
  let activeInput = $state<'parent' | 'child' | null>(null);

  $effect(() => {
    // Explicitly track uri (re-load when navigating to a different tree)
    // but untrack the API calls so auth state changes don't cause re-runs
    const currentUri = uri;
    untrack(() => load(currentUri));
  });

  async function load(treeUri: string) {
    loading = true;
    const [d, tags] = await Promise.all([getSkillTree(treeUri), listTags()]);
    detail = d;
    allTags = tags;
    loading = false;
  }

  function searchTags(query: string): Tag[] {
    if (!query) return [];
    const q = query.toLowerCase();
    return allTags.filter(t => t.id.toLowerCase().includes(q) || t.name.toLowerCase().includes(q)).slice(0, 8);
  }

  function onInputParent() { tagSuggestions = searchTags(newParent); activeInput = 'parent'; }
  function onInputChild() { tagSuggestions = searchTags(newChild); activeInput = 'child'; }
  function selectSuggestion(tagId: string) {
    if (activeInput === 'parent') newParent = tagId;
    else if (activeInput === 'child') newChild = tagId;
    tagSuggestions = [];
    activeInput = null;
  }

  async function addEdge() {
    if (!newParent.trim() || !newChild.trim() || !detail) return;
    await addSkillTreeEdge(uri, newParent.trim(), newChild.trim());
    newParent = ''; newChild = '';
    await load(uri);
  }

  async function removeEdge(e: SkillTreeEdge) {
    if (!detail) return;
    await removeSkillTreeEdge(uri, e.parent_tag, e.child_tag);
    await load(uri);
  }

  async function doFork() {
    const result = await forkSkillTree(uri);
    window.location.hash = `#/skill-tree?uri=${encodeURIComponent(result.at_uri)}`;
  }

  async function doAdopt() {
    await adoptSkillTree(uri);
    alert(t('skills.adopted'));
  }

  // Build tree structure for visualization
  let treeStructure = $derived.by(() => {
    if (!detail) return { roots: [] as string[], children: new Map<string, string[]>() };
    const children = new Map<string, string[]>();
    const hasParent = new Set<string>();
    const allNodes = new Set<string>();
    for (const e of detail.edges) {
      const arr = children.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      children.set(e.parent_tag, arr);
      hasParent.add(e.child_tag);
      allNodes.add(e.parent_tag);
      allNodes.add(e.child_tag);
    }
    const roots = [...allNodes].filter(n => !hasParent.has(n)).sort();
    return { roots, children };
  });

  function tagName(id: string): string {
    const i18nNames = detail?.tag_names_i18n?.[id];
    const fallbackName = detail?.tag_names_map[id] || id;
    return resolveTagName(i18nNames, fallbackName, id);
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if detail}
  <div class="tree-header">
    <div class="tree-title-row">
      <h1>{detail.tree.title}</h1>
      {#if detail.tree.tag_id}
        <span class="field-badge">{tagName(detail.tree.tag_id)}</span>
      {/if}
    </div>
    <div class="tree-actions">
      {#if isLoggedIn}
        <button class="btn" onclick={doAdopt}>{t('skillTree.adopt')}</button>
        <button class="btn" onclick={doFork}>Fork</button>
        <button class="btn" onclick={() => castVote(uri, 1)}>👍</button>
      {/if}
    </div>
  </div>
  {#if detail.tree.description}
    <p class="desc">{detail.tree.description}</p>
  {/if}
  {#if detail.tree.forked_from}
    <p class="forked-info">Forked from <a href="#/skill-tree?uri={encodeURIComponent(detail.tree.forked_from)}">{detail.tree.forked_from.slice(0, 40)}...</a></p>
  {/if}

  <!-- Tree visualization -->
  <div class="tree-visual">
    {#each treeStructure.roots as root}
      {@render treeNode(root, 0)}
    {/each}
  </div>

  <!-- Edge list (editable if owner) -->
  {#if isOwner}
    <div class="editor-section">
      <h3>{t('skillTree.editRelations')}</h3>
      <div class="edge-form">
        <div class="input-wrap">
          <input type="text" bind:value={newParent} oninput={onInputParent} placeholder={t('skillTree.parentTag')} onfocus={onInputParent} />
          {#if activeInput === 'parent' && tagSuggestions.length > 0}
            <div class="suggestions">
              {#each tagSuggestions as s}
                <button onclick={() => selectSuggestion(s.id)}>{s.name} <span class="sg-id">({s.id})</span></button>
              {/each}
            </div>
          {/if}
        </div>
        <span class="arrow">→</span>
        <div class="input-wrap">
          <input type="text" bind:value={newChild} oninput={onInputChild} placeholder={t('skillTree.childTag')} onfocus={onInputChild} />
          {#if activeInput === 'child' && tagSuggestions.length > 0}
            <div class="suggestions">
              {#each tagSuggestions as s}
                <button onclick={() => selectSuggestion(s.id)}>{s.name} <span class="sg-id">({s.id})</span></button>
              {/each}
            </div>
          {/if}
        </div>
        <button class="add-btn" onclick={addEdge}>{t('common.add')}</button>
      </div>
      <p class="hint">{t('skillTree.autoCreateHint')}</p>
    </div>
  {/if}

  <div class="edge-list">
    <h3>{t('skillTree.allRelations', detail.edges.length)}</h3>
    {#each detail.edges as e}
      <div class="edge-row">
        <a href="#/tag?id={encodeURIComponent(e.parent_tag)}" class="tag">{tagName(e.parent_tag)}</a>
        <span class="arrow">→</span>
        <a href="#/tag?id={encodeURIComponent(e.child_tag)}" class="tag">{tagName(e.child_tag)}</a>
        {#if isOwner}
          <button class="remove-btn" onclick={() => removeEdge(e)}>×</button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#snippet treeNode(id: string, depth: number)}
  <div class="tree-item" style="padding-left: {depth * 24}px">
    {#if treeStructure.children.has(id)}
      <button class="collapse-btn" title={t('skillTree.collapse')} class:collapsed={collapsed.has(id)} onclick={() => toggleCollapse(id)}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
      </button>
    {:else}
      <span class="collapse-spacer"></span>
    {/if}
    <a href="#/tag?id={encodeURIComponent(id)}" class="node-link">{tagName(id)}</a>
    {#if treeStructure.children.has(id)}
      <span class="child-count">{treeStructure.children.get(id)!.length}</span>
    {/if}
  </div>
  {#if treeStructure.children.has(id) && !collapsed.has(id)}
    {#each treeStructure.children.get(id)! as child}
      {@render treeNode(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<style>
  .tree-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .tree-header h1 { margin: 0; }
  .tree-title-row { display: flex; align-items: center; gap: 10px; }
  .field-badge {
    font-size: 12px;
    background: rgba(95,155,101,0.12);
    color: var(--accent);
    padding: 2px 10px;
    border-radius: 3px;
  }
  .tree-actions { display: flex; gap: 8px; }
  .btn {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn:hover { border-color: var(--accent); color: var(--accent); }
  .desc { font-size: 14px; color: var(--text-secondary); margin: 0 0 12px; }
  .forked-info { font-size: 13px; color: var(--text-hint); margin: 0 0 16px; }
  .forked-info a { color: var(--accent); }

  /* Tree visualization */
  .tree-visual {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px;
    margin-bottom: 24px;
    background: var(--bg-white);
  }
  .tree-item {
    padding: 4px 0;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .collapse-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    color: var(--text-hint);
    transition: transform 0.15s, color 0.15s;
    width: 16px;
    flex-shrink: 0;
    transform: rotate(90deg);
  }
  .collapse-btn.collapsed {
    transform: rotate(0deg);
  }
  .collapse-btn:hover { color: var(--accent); }
  .collapse-spacer {
    width: 16px;
    flex-shrink: 0;
  }
  .child-count {
    font-size: 11px;
    color: var(--text-hint);
    margin-left: 4px;
  }
  .node-link {
    font-family: var(--font-serif);
    font-size: 14px;
    color: var(--text-primary);
    text-decoration: none;
    padding: 2px 8px;
    border-radius: 3px;
    transition: all 0.15s;
  }
  .node-link:hover { color: var(--accent); background: rgba(95,155,101,0.06); }

  /* Editor */
  .editor-section {
    margin: 24px 0 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  .editor-section h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 0 0 12px;
  }
  .edge-form {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .input-wrap {
    position: relative;
    flex: 1;
  }
  .input-wrap input {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 10px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .suggestions {
    position: absolute;
    top: 100%;
    left: 0; right: 0;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10;
    max-height: 180px;
    overflow-y: auto;
  }
  .suggestions button {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }
  .suggestions button:hover { background: var(--bg-gray, #f5f5f5); }
  .sg-id { color: var(--text-hint); font-size: 11px; }
  .arrow { color: var(--text-hint); font-size: 16px; line-height: 32px; }
  .add-btn {
    padding: 6px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }
  .hint { font-size: 12px; color: var(--text-hint); margin: 6px 0 0; }

  /* Edge list */
  .edge-list h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 16px 0 8px;
  }
  .edge-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 14px;
  }
  .remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 16px;
    margin-left: auto;
  }
  .remove-btn:hover { color: var(--error, #c33); }
</style>
