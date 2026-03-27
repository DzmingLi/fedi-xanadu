<script lang="ts">
  import { createSkillTree, listTags, searchTags } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t } from '../lib/i18n';
  import type { Tag, SkillTreeEdge } from '../lib/types';

  let title = $state('');
  let description = $state('');
  let tagId = $state('');
  let fieldQuery = $state('');
  let fieldSuggestions = $state<Tag[]>([]);
  let showFieldSugg = $state(false);
  let edges = $state<SkillTreeEdge[]>([]);
  let allTags = $state<Tag[]>([]);
  let error = $state('');
  let creating = $state(false);

  let newParent = $state('');
  let newChild = $state('');

  let parentSuggestions = $derived(
    newParent ? allTags.filter(t => t.id.includes(newParent) || t.name.toLowerCase().includes(newParent.toLowerCase())).slice(0, 6) : []
  );
  let childSuggestions = $derived(
    newChild ? allTags.filter(t => t.id.includes(newChild) || t.name.toLowerCase().includes(newChild.toLowerCase())).slice(0, 6) : []
  );

  let showParentSugg = $state(false);
  let showChildSugg = $state(false);

  let fieldDebounce: ReturnType<typeof setTimeout>;
  function onFieldInput() {
    clearTimeout(fieldDebounce);
    if (!fieldQuery.trim()) { fieldSuggestions = []; return; }
    fieldDebounce = setTimeout(async () => {
      fieldSuggestions = await searchTags(fieldQuery.trim());
    }, 150);
  }

  function selectField(tag: Tag) {
    tagId = tag.id;
    fieldQuery = tag.name;
    fieldSuggestions = [];
    showFieldSugg = false;
  }

  function clearField() {
    tagId = '';
    fieldQuery = '';
    fieldSuggestions = [];
  }

  $effect(() => { listTags().then(t => allTags = t); });

  function addEdge() {
    const p = newParent.trim();
    const c = newChild.trim();
    if (!p || !c || p === c) return;
    if (edges.some(e => e.parent_tag === p && e.child_tag === c)) return;
    edges = [...edges, { parent_tag: p, child_tag: c }];
    newParent = ''; newChild = '';
  }

  function removeEdge(i: number) {
    edges = edges.filter((_, idx) => idx !== i);
  }

  function tagDisplay(id: string): string {
    const t = allTags.find(t => t.id === id);
    return t ? t.name : id;
  }

  async function submit() {
    if (!getAuth()) { error = t('auth.submit'); return; }
    if (!title.trim()) { error = t('newSkillTree.errTitle'); return; }
    if (edges.length === 0) { error = t('newSkillTree.errRelations'); return; }
    creating = true;
    error = '';
    try {
      const tree = await createSkillTree({ title: title.trim(), description: description.trim() || undefined, tag_id: tagId || undefined, edges });
      window.location.hash = `#/skill-tree?uri=${encodeURIComponent(tree.at_uri)}`;
    } catch (e: any) {
      error = e.message || t('newSkillTree.errCreate');
    }
    creating = false;
  }
</script>

<h1>{t('newSkillTree.title')}</h1>

{#if error}
  <p class="error">{error}</p>
{/if}

<div class="form">
  <label>
    {t('newSkillTree.titleLabel')}
    <input type="text" bind:value={title} placeholder={t('newSkillTree.titlePlaceholder')} />
  </label>
  <label>
    {t('newArticle.descLabel')}
    <textarea bind:value={description} rows="2" placeholder={t('newSkillTree.descPlaceholder')}></textarea>
  </label>
  <label>
    {t('newSkillTree.tagLabel')}
    <div class="field-input-wrap">
      <input type="text" bind:value={fieldQuery} oninput={onFieldInput}
        onfocus={() => showFieldSugg = true} onblur={() => setTimeout(() => showFieldSugg = false, 200)}
        placeholder={t('newSkillTree.tagPlaceholder')} />
      {#if tagId}
        <button class="field-clear" onclick={clearField} type="button">×</button>
      {/if}
      {#if showFieldSugg && fieldSuggestions.length > 0}
        <div class="suggestions">
          {#each fieldSuggestions as s}
            <button type="button" onmousedown={() => selectField(s)}>{s.name} <span class="sg-id">{s.id}</span></button>
          {/each}
        </div>
      {/if}
    </div>
  </label>

  <h2>{t('newSkillTree.addRelation')}</h2>
  <p class="hint">{t('newSkillTree.relationHint')}</p>

  <div class="edge-form">
    <div class="input-wrap">
      <input type="text" bind:value={newParent} placeholder={t('newSkillTree.parentPlaceholder')}
        onfocus={() => showParentSugg = true} onblur={() => setTimeout(() => showParentSugg = false, 200)} />
      {#if showParentSugg && parentSuggestions.length > 0}
        <div class="suggestions">
          {#each parentSuggestions as s}
            <button onmousedown={() => { newParent = s.id; showParentSugg = false; }}>{s.name} <span class="sg-id">{s.id}</span></button>
          {/each}
        </div>
      {/if}
    </div>
    <span class="arrow">→</span>
    <div class="input-wrap">
      <input type="text" bind:value={newChild} placeholder={t('newSkillTree.childPlaceholder')}
        onfocus={() => showChildSugg = true} onblur={() => setTimeout(() => showChildSugg = false, 200)} />
      {#if showChildSugg && childSuggestions.length > 0}
        <div class="suggestions">
          {#each childSuggestions as s}
            <button onmousedown={() => { newChild = s.id; showChildSugg = false; }}>{s.name} <span class="sg-id">{s.id}</span></button>
          {/each}
        </div>
      {/if}
    </div>
    <button class="add-btn" onclick={addEdge}>{t('common.add')}</button>
  </div>

  {#if edges.length > 0}
    <div class="edge-list">
      {#each edges as e, i}
        <div class="edge-row">
          <span class="tag">{tagDisplay(e.parent_tag)}</span>
          <span class="arrow">→</span>
          <span class="tag">{tagDisplay(e.child_tag)}</span>
          <button class="remove-btn" onclick={() => removeEdge(i)}>×</button>
        </div>
      {/each}
    </div>
  {/if}

  <button class="submit-btn" onclick={submit} disabled={creating}>
    {creating ? t('newSkillTree.creating') : t('newSkillTree.create')}
  </button>
</div>

<style>
  .form { display: flex; flex-direction: column; gap: 14px; max-width: 600px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 14px; color: var(--text-secondary); }
  input, textarea, select {
    font-family: var(--font-sans); font-size: 14px; padding: 8px 10px;
    border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white);
  }
  h2 { font-family: var(--font-serif); font-weight: 400; margin: 8px 0 0; }
  .hint { font-size: 13px; color: var(--text-hint); margin: 0; }
  .edge-form { display: flex; align-items: flex-start; gap: 8px; }
  .input-wrap { position: relative; flex: 1; }
  .input-wrap input { width: 100%; box-sizing: border-box; }
  .suggestions {
    position: absolute; top: 100%; left: 0; right: 0; background: var(--bg-white);
    border: 1px solid var(--border); border-radius: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10; max-height: 180px; overflow-y: auto;
  }
  .suggestions button {
    display: block; width: 100%; padding: 6px 10px; border: none; background: none;
    text-align: left; cursor: pointer; font-size: 13px;
  }
  .suggestions button:hover { background: var(--bg-gray, #f5f5f5); }
  .sg-id { color: var(--text-hint); font-size: 11px; margin-left: 4px; }
  .arrow { color: var(--text-hint); font-size: 16px; line-height: 32px; }
  .add-btn {
    padding: 6px 14px; font-size: 13px; background: var(--accent); color: white;
    border: none; border-radius: 4px; cursor: pointer; white-space: nowrap;
  }
  .edge-list { display: flex; flex-direction: column; gap: 4px; }
  .edge-row { display: flex; align-items: center; gap: 8px; font-size: 14px; }
  .remove-btn {
    background: none; border: none; cursor: pointer; color: var(--text-hint);
    font-size: 16px; margin-left: auto;
  }
  .remove-btn:hover { color: var(--error, #c33); }
  .submit-btn {
    margin-top: 12px; padding: 10px 24px; font-size: 14px; background: var(--accent);
    color: white; border: none; border-radius: 4px; cursor: pointer; align-self: flex-start;
  }
  .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { color: var(--error, #c33); font-size: 14px; }
  .field-input-wrap { position: relative; }
  .field-input-wrap input { width: 100%; box-sizing: border-box; }
  .field-clear {
    position: absolute; right: 8px; top: 50%; transform: translateY(-50%);
    background: none; border: none; cursor: pointer; color: var(--text-hint);
    font-size: 16px; padding: 0 4px;
  }
  .field-clear:hover { color: var(--text-primary); }
</style>
