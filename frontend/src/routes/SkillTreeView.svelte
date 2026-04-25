<script lang="ts">
  import { getSkillTree, adoptSkillTree, addSkillTreeEdge, removeSkillTreeEdge, addSkillTreePrereq, removeSkillTreePrereq, castVote, listTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { tagName as resolveTagName } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import type { SkillTreeDetail, SkillTreeEdge, SkillTreePrereq, Tag } from '../lib/types';
  import SkillTreeGraph from '../lib/components/SkillTreeGraph.svelte';

  let { uri } = $props<{ uri: string }>();

  let detail = $state<SkillTreeDetail | null>(null);
  let loading = $state(true);
  let allTags = $state<Tag[]>([]);
  let isLoggedIn = $derived(!!getAuth());
  let isOwner = $derived(isLoggedIn && detail?.tree.did === getAuth()?.did);

  // Edit: hierarchy edges
  let newParent = $state('');
  let newChild = $state('');
  let tagSuggestions = $state<Tag[]>([]);
  let activeInput = $state<'parent' | 'child' | 'pq-from' | 'pq-to' | null>(null);

  // Edit: prereq edges
  let newPqFrom = $state('');
  let newPqTo = $state('');
  let newPqType = $state<'required' | 'recommended'>('required');

  $effect(() => { load(uri); });

  async function load(treeUri: string) {
    loading = true;
    const [d, tags] = await Promise.all([getSkillTree(treeUri), listTags()]);
    detail = d;
    allTags = tags;
    loading = false;
  }

  function searchTagsFor(query: string): Tag[] {
    if (!query) return [];
    const q = query.toLowerCase();
    return allTags.filter(t => t.id.toLowerCase().includes(q) || t.name.toLowerCase().includes(q)).slice(0, 8);
  }

  function onInputParent() { tagSuggestions = searchTagsFor(newParent); activeInput = 'parent'; }
  function onInputChild() { tagSuggestions = searchTagsFor(newChild); activeInput = 'child'; }
  function onInputPqFrom() { tagSuggestions = searchTagsFor(newPqFrom); activeInput = 'pq-from'; }
  function onInputPqTo() { tagSuggestions = searchTagsFor(newPqTo); activeInput = 'pq-to'; }

  function selectSuggestion(tagId: string) {
    if (activeInput === 'parent') newParent = tagId;
    else if (activeInput === 'child') newChild = tagId;
    else if (activeInput === 'pq-from') newPqFrom = tagId;
    else if (activeInput === 'pq-to') newPqTo = tagId;
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

  async function addPrereq() {
    if (!newPqFrom.trim() || !newPqTo.trim() || !detail) return;
    await addSkillTreePrereq(uri, newPqFrom.trim(), newPqTo.trim(), newPqType);
    newPqFrom = ''; newPqTo = '';
    await load(uri);
  }

  async function removePrereq(p: SkillTreePrereq) {
    if (!detail) return;
    await removeSkillTreePrereq(uri, p.from_tag, p.to_tag);
    await load(uri);
  }

  async function doAdopt() {
    await adoptSkillTree(uri);
    alert(t('skills.adopted'));
  }

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
        <button class="btn" onclick={() => castVote(uri, 1)}>👍</button>
      {/if}
    </div>
  </div>
  {#if detail.tree.description}
    <p class="desc">{detail.tree.description}</p>
  {/if}

  <!-- DAG visualization -->
  <div class="tree-visual">
    <SkillTreeGraph
      edges={detail.edges}
      prereqs={detail.prereqs}
      tagNamesMap={detail.tag_names_map}
      tagNamesI18n={detail.tag_names_i18n}
    />
  </div>

  <!-- Hierarchy edges (editable if owner) -->
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
        <a href="/tag?id={encodeURIComponent(e.parent_tag)}" class="tag">{tagName(e.parent_tag)}</a>
        <span class="arrow">→</span>
        <a href="/tag?id={encodeURIComponent(e.child_tag)}" class="tag">{tagName(e.child_tag)}</a>
        {#if isOwner}
          <button class="remove-btn" onclick={() => removeEdge(e)}>×</button>
        {/if}
      </div>
    {/each}
  </div>

  <!-- Prereq edges -->
  {#if isOwner}
    <div class="editor-section">
      <h3>{t('skillTree.editPrereqs')}</h3>
      <div class="edge-form">
        <div class="input-wrap">
          <input type="text" bind:value={newPqFrom} oninput={onInputPqFrom} placeholder={t('skillTree.prereqFrom')} onfocus={onInputPqFrom} />
          {#if activeInput === 'pq-from' && tagSuggestions.length > 0}
            <div class="suggestions">
              {#each tagSuggestions as s}
                <button onclick={() => selectSuggestion(s.id)}>{s.name} <span class="sg-id">({s.id})</span></button>
              {/each}
            </div>
          {/if}
        </div>
        <span class="arrow">→</span>
        <div class="input-wrap">
          <input type="text" bind:value={newPqTo} oninput={onInputPqTo} placeholder={t('skillTree.prereqTo')} onfocus={onInputPqTo} />
          {#if activeInput === 'pq-to' && tagSuggestions.length > 0}
            <div class="suggestions">
              {#each tagSuggestions as s}
                <button onclick={() => selectSuggestion(s.id)}>{s.name} <span class="sg-id">({s.id})</span></button>
              {/each}
            </div>
          {/if}
        </div>
        <select bind:value={newPqType} class="pq-type-select">
          <option value="required">Required</option>
          <option value="recommended">Recommended</option>
        </select>
        <button class="add-btn" onclick={addPrereq}>{t('common.add')}</button>
      </div>
    </div>
  {/if}

  <div class="edge-list">
    <h3>{t('skillTree.allPrereqs', detail.prereqs.length)}</h3>
    {#if detail.prereqs.length === 0}
      <p class="hint">{t('skillTree.noPrereqs')}</p>
    {:else}
      {#each detail.prereqs as p}
        <div class="edge-row">
          <a href="/tag?id={encodeURIComponent(p.from_tag)}" class="tag">{tagName(p.from_tag)}</a>
          <span class="arrow">→</span>
          <a href="/tag?id={encodeURIComponent(p.to_tag)}" class="tag">{tagName(p.to_tag)}</a>
          <span class="prereq-badge badge-{p.prereq_type}">{p.prereq_type}</span>
          {#if isOwner}
            <button class="remove-btn" onclick={() => removePrereq(p)}>×</button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
{/if}


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

  .tree-visual {
    margin-bottom: 24px;
  }

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
  .prereq-badge {
    font-size: 10px; padding: 1px 6px; border-radius: 3px; margin-left: 4px;
  }
  .badge-required { background: rgba(239,68,68,0.1); color: #dc2626; }
  .badge-recommended { background: rgba(245,158,11,0.1); color: #b45309; }
  .pq-type-select {
    padding: 6px 8px; font-size: 13px; border: 1px solid var(--border);
    border-radius: 4px; background: var(--bg-white);
  }
</style>
