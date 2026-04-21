<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listTagParents, addTagParent, removeTagParent, listTags, searchTags, createTagInline,
    requestTagDeletion,
  } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, LOCALES, getLocale } from '../lib/i18n/index.svelte';
  import { tagStore } from '../lib/tagStore.svelte';
  import type { Tag } from '../lib/types';

  $effect(() => { tagStore.ensure(); });
  const displayTagId = (id: string, _map?: Map<string, Tag>) => tagStore.localize(id);

  /**
   * For each group pick one canonical tag: the member matching the UI
   * locale, then the en member, then any. Returns Map<tagId, canonical-tagId>.
   * Tag ids outside this mapping (orphan edges referencing a deleted tag)
   * fall through unchanged via callers' `?? id` defaults.
   */
  function buildGroupCanon(tags: Tag[], locale: string): Map<string, string> {
    const byGroup = new Map<string, Tag[]>();
    for (const t of tags) {
      const arr = byGroup.get(t.tag_id) ?? [];
      arr.push(t);
      byGroup.set(t.tag_id, arr);
    }
    const canon = new Map<string, string>();
    for (const members of byGroup.values()) {
      const pick =
        members.find(m => m.lang === locale) ??
        members.find(m => m.lang === 'en') ??
        members[0];
      for (const m of members) canon.set(m.id, pick.id);
    }
    return canon;
  }

  type Edge = { parent_tag: string; child_tag: string };

  let edges = $state<Edge[]>([]);
  let tags = $state<Tag[]>([]);
  let loading = $state(true);
  let error = $state('');
  let isLoggedIn = $derived(!!getAuth());

  // Inline tag editor state (one tag open at a time)
  let editingTag = $state<string | null>(null);
  let editNames = $state<Record<string, string>>({});
  let editSaving = $state(false);
  let editError = $state('');
  let deleteReason = $state('');
  let deleteSubmitting = $state(false);
  let deleteSubmitted = $state(false);

  async function submitDeletionRequest(tagId: string) {
    const reason = deleteReason.trim();
    if (!reason) return;
    deleteSubmitting = true;
    try {
      await requestTagDeletion(tagId, reason);
      deleteSubmitted = true;
      deleteReason = '';
    } catch (err: any) {
      editError = err.message ?? String(err);
    } finally {
      deleteSubmitting = false;
    }
  }

  let tagById = $derived.by(() => {
    const m = new Map<string, Tag>();
    for (const tg of tags) m.set(tg.id, tg);
    return m;
  });

  async function toggleEdit(id: string) {
    if (editingTag === id) { editingTag = null; return; }
    editingTag = id;
    editError = '';
    deleteReason = '';
    deleteSubmitted = false;
    const tg = tagById.get(id);
    const names: Record<string, string> = { ...(tg?.names ?? {}) };
    for (const loc of LOCALES) {
      if (!(loc.code in names)) names[loc.code] = '';
    }
    if (!names.en?.trim()) names.en = id;
    editNames = names;
  }

  async function saveEditNames() {
    if (!editingTag) return;
    editSaving = true;
    editError = '';
    try {
      const cleaned = Object.fromEntries(Object.entries(editNames).filter(([_, v]) => v.trim()));
      const updated = await updateTagNames(editingTag, cleaned);
      // Refresh local tag entry so other UI reflects the change
      const idx = tags.findIndex(x => x.id === updated.id);
      if (idx >= 0) tags[idx] = updated;
    } catch (err: any) {
      editError = err.message ?? String(err);
    } finally {
      editSaving = false;
    }
  }

  // Add-edge form state
  let newParent = $state('');
  let newChild = $state('');
  let parentSuggest = $state<Tag[]>([]);
  let childSuggest = $state<Tag[]>([]);
  let submitting = $state(false);

  // New-root-tag form state
  let newTagId = $state('');
  let newTagName = $state('');
  let creatingTag = $state(false);

  let parentTimer: ReturnType<typeof setTimeout>;
  let childTimer: ReturnType<typeof setTimeout>;

  $effect(() => {
    clearTimeout(parentTimer);
    const q = newParent.trim();
    if (!q) { parentSuggest = []; return; }
    parentTimer = setTimeout(async () => {
      parentSuggest = await searchTags(q).catch(() => []);
    }, 150);
  });
  $effect(() => {
    clearTimeout(childTimer);
    const q = newChild.trim();
    if (!q) { childSuggest = []; return; }
    childTimer = setTimeout(async () => {
      childSuggest = await searchTags(q).catch(() => []);
    }, 150);
  });

  async function load() {
    loading = true;
    try {
      const [e, ts] = await Promise.all([listTagParents(), listTags()]);
      edges = e;
      tags = ts;
    } catch (err: any) {
      error = err.message ?? String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    document.title = `${t('hierarchy.title')} — NightBoat`;
    load();
  });

  async function submitAdd() {
    const p = newParent.trim();
    const c = newChild.trim();
    if (!p || !c) { error = t('hierarchy.bothRequired'); return; }
    if (p === c) { error = t('hierarchy.sameTag'); return; }
    submitting = true;
    error = '';
    try {
      await addTagParent(p, c);
      newParent = '';
      newChild = '';
      parentSuggest = [];
      childSuggest = [];
      await load();
    } catch (err: any) {
      error = err.message ?? String(err);
    } finally {
      submitting = false;
    }
  }

  async function submitCreateTag() {
    const id = newTagId.trim();
    const name = newTagName.trim() || id;
    if (!id) return;
    creatingTag = true;
    error = '';
    try {
      await createTagInline(id, name);
      newTagId = '';
      newTagName = '';
      await load();
    } catch (err: any) {
      error = err.message ?? String(err);
    } finally {
      creatingTag = false;
    }
  }

  async function removeEdge(parent: string, child: string) {
    if (!confirm(t('hierarchy.confirmRemove').replace('{p}', parent).replace('{c}', child))) return;
    try {
      await removeTagParent(parent, child);
      await load();
    } catch (err: any) {
      error = err.message ?? String(err);
    }
  }

  // Canonical-per-group resolver: every tag id (including edge endpoints)
  // is mapped to one representative picked for the UI locale. Group
  // siblings collapse to one chip so zh UI doesn't show "数学" twice
  // just because there happens to be both `Mathematics` (en) and `数学`
  // (zh) as group members.
  let groupCanon = $derived.by(() => buildGroupCanon(tags, getLocale()));

  // Group edges by parent (after canonicalizing both ends to the group
  // rep). Duplicate edges that collapse to the same (p, c) are deduped.
  let groupedByParent = $derived.by(() => {
    const m = new Map<string, Set<string>>();
    for (const e of edges) {
      const p = groupCanon.get(e.parent_tag) ?? e.parent_tag;
      const c = groupCanon.get(e.child_tag) ?? e.child_tag;
      if (p === c) continue;
      const set = m.get(p) ?? new Set<string>();
      set.add(c);
      m.set(p, set);
    }
    return Array.from(m.entries())
      .map(([parent, children]) => ({ parent, children: [...children].sort() }))
      .sort((a, b) => a.parent.localeCompare(b.parent));
  });

  // Orphans: one entry per group, representative only. A tag has no
  // parent if no *group member* of it is a child in any edge.
  let orphans = $derived.by(() => {
    const parentedGroups = new Set<string>();
    for (const e of edges) {
      const childTag = tagById.get(e.child_tag);
      if (childTag) parentedGroups.add(childTag.tag_id);
    }
    const seen = new Set<string>();
    const out: string[] = [];
    for (const tg of tags) {
      if (parentedGroups.has(tg.tag_id)) continue;
      if (seen.has(tg.tag_id)) continue;
      const canonId = groupCanon.get(tg.id) ?? tg.id;
      if (canonId === tg.id) {
        seen.add(tg.tag_id);
        out.push(tg.id);
      }
    }
    return out.sort();
  });
</script>

{#snippet tagEditor(id: string)}
  <div class="tag-editor">
    <div class="te-header">
      <strong>{id}</strong>
      <button class="btn-ghost" onclick={() => (editingTag = null)}>{t('common.close')}</button>
    </div>
    {#if editError}<p class="error-msg">{editError}</p>{/if}
    <div class="te-section">
      <a class="btn" href="/tag?id={encodeURIComponent(id)}">{t('hierarchy.openTagPage')} →</a>
    </div>
    <div class="te-section">
      <div class="te-label">{t('tags.deleteTitle')}</div>
      <p class="hint" style="margin:4px 0 6px">{t('tags.deleteHint')}</p>
      {#if deleteSubmitted}
        <p class="hint" style="color: var(--accent)">{t('tags.deleteSubmitted')}</p>
      {:else}
        <div class="alias-add">
          <input bind:value={deleteReason} placeholder={t('tags.deleteReasonPlaceholder')} />
          <button class="btn" style="color:#c00;border-color:#c00"
            onclick={() => submitDeletionRequest(id)}
            disabled={!deleteReason.trim() || deleteSubmitting}>
            {deleteSubmitting ? t('common.saving') : t('tags.requestDelete')}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/snippet}

<div class="hierarchy-page">
  <header class="page-header">
    <h1>{t('hierarchy.title')}</h1>
    <p class="hint">{t('hierarchy.intro')}</p>
  </header>

  {#if error}<p class="error-msg">{error}</p>{/if}

  {#if isLoggedIn}
    <section class="add-form">
      <h2>{t('hierarchy.addEdge')}</h2>
      <div class="form-row">
        <div class="field">
          <label>{t('hierarchy.parent')}</label>
          <input bind:value={newParent} placeholder={t('hierarchy.parentPlaceholder')} />
          {#if parentSuggest.length > 0}
            <div class="suggest-list">
              {#each parentSuggest.slice(0, 8) as s}
                <button type="button" onclick={() => { newParent = s.id; parentSuggest = []; }}>{s.name}</button>
              {/each}
            </div>
          {/if}
        </div>
        <div class="arrow">→</div>
        <div class="field">
          <label>{t('hierarchy.child')}</label>
          <input bind:value={newChild} placeholder={t('hierarchy.childPlaceholder')} />
          {#if childSuggest.length > 0}
            <div class="suggest-list">
              {#each childSuggest.slice(0, 8) as s}
                <button type="button" onclick={() => { newChild = s.id; childSuggest = []; }}>{s.name}</button>
              {/each}
            </div>
          {/if}
        </div>
        <button class="btn btn-primary" onclick={submitAdd} disabled={submitting}>
          {submitting ? t('hierarchy.adding') : t('hierarchy.add')}
        </button>
      </div>
    </section>
    <section class="add-form">
      <h2>{t('hierarchy.addRoot')}</h2>
      <div class="form-row">
        <div class="field">
          <label>{t('hierarchy.tagId')}</label>
          <input bind:value={newTagId} placeholder={t('hierarchy.tagIdPlaceholder')} />
        </div>
        <div class="field">
          <label>{t('hierarchy.tagName')}</label>
          <input bind:value={newTagName} placeholder={t('hierarchy.tagNamePlaceholder')} />
        </div>
        <button class="btn btn-primary" onclick={submitCreateTag} disabled={creatingTag}>
          {creatingTag ? t('hierarchy.creating') : t('hierarchy.createRoot')}
        </button>
      </div>
    </section>
  {:else}
    <p class="hint">{t('hierarchy.loginToEdit')}</p>
  {/if}

  {#if loading}
    <p class="state">{t('common.loading')}</p>
  {:else}
    <section class="roots">
      <h2>{t('hierarchy.roots')} <span class="count">({orphans.length})</span></h2>
      <div class="chip-row">
        {#each orphans as tag}
          <span class="tag-chip root">
            <a href="/tag?id={encodeURIComponent(tag)}">{displayTagId(tag, tagById)}</a>
            {#if isLoggedIn}
              <a class="gear" href="/tag?id={encodeURIComponent(tag)}&edit=1" title={t('tags.edit')}>✎</a>
            {/if}
          </span>
          {#if editingTag === tag}
            {@render tagEditor(tag)}
          {/if}
        {/each}
      </div>
    </section>

    <section class="groups">
      {#each groupedByParent as g}
        <div class="group">
          <h3 class="group-parent">
            <a href="/tag?id={encodeURIComponent(g.parent)}">{displayTagId(g.parent, tagById)}</a>
            <span class="count">{g.children.length}</span>
            {#if isLoggedIn}
              <a class="gear" href="/tag?id={encodeURIComponent(g.parent)}&edit=1" title={t('tags.edit')}>✎</a>
            {/if}
          </h3>
          {#if editingTag === g.parent}
            {@render tagEditor(g.parent)}
          {/if}
          <div class="chip-row">
            {#each g.children as c}
              <span class="tag-chip">
                <a href="/tag?id={encodeURIComponent(c)}">{displayTagId(c, tagById)}</a>
                {#if isLoggedIn}
                  <a class="gear" href="/tag?id={encodeURIComponent(c)}&edit=1" title={t('tags.edit')}>✎</a>
                  <button class="x" onclick={() => removeEdge(g.parent, c)} title={t('hierarchy.remove')}>×</button>
                {/if}
              </span>
              {#if editingTag === c}
                {@render tagEditor(c)}
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .hierarchy-page { max-width: 900px; margin: 0 auto; padding: 24px 16px; }
  .page-header { margin-bottom: 20px; }
  .page-header h1 { font-family: var(--font-serif); margin: 0 0 4px; }
  .hint { color: var(--text-secondary); font-size: 13px; margin: 0; }
  .error-msg { background: #fee; color: #c00; padding: 8px 12px; border-radius: 4px; font-size: 13px; }
  .state { text-align: center; color: var(--text-secondary); }

  .add-form { background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px; padding: 14px 18px; margin-bottom: 24px; }
  .add-form h2 { font-size: 14px; margin: 0 0 10px; color: var(--text-secondary); font-weight: 500; }
  .form-row { display: flex; align-items: flex-end; gap: 10px; flex-wrap: wrap; }
  .field { flex: 1; min-width: 180px; position: relative; }
  .field label { display: block; font-size: 12px; color: var(--text-hint); margin-bottom: 3px; }
  .field input { width: 100%; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); color: var(--text-primary); font-size: 14px; box-sizing: border-box; }
  .arrow { font-size: 20px; color: var(--text-hint); padding-bottom: 6px; }
  .suggest-list { position: absolute; top: 100%; left: 0; right: 0; z-index: 10; background: var(--bg-white); border: 1px solid var(--border); border-radius: 4px; max-height: 200px; overflow-y: auto; box-shadow: 0 4px 12px rgba(0,0,0,0.08); }
  .suggest-list button { display: block; width: 100%; text-align: left; padding: 5px 10px; font-size: 13px; background: none; border: none; color: var(--text-primary); cursor: pointer; }
  .suggest-list button:hover { background: var(--bg-hover); }
  .btn { padding: 6px 14px; border-radius: 4px; border: 1px solid var(--border); background: var(--bg-white); cursor: pointer; font-size: 14px; }
  .btn-primary { background: var(--accent); color: white; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }

  section { margin-bottom: 24px; }
  section h2 { font-family: var(--font-serif); font-size: 1.1rem; margin: 0 0 10px; }
  .count { color: var(--text-hint); font-size: 12px; font-weight: 400; }
  .chip-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .tag-chip { display: inline-flex; align-items: center; gap: 3px; padding: 3px 8px; border-radius: 12px; background: var(--bg-hover, #f5f5f5); border: 1px solid var(--border); font-size: 12px; }
  .tag-chip a { color: var(--text-primary); text-decoration: none; }
  .tag-chip a:hover { color: var(--accent); }
  .tag-chip.root { background: var(--accent-light, #e8f4fd); border-color: var(--accent); }
  .tag-chip .x { background: none; border: none; cursor: pointer; color: var(--text-hint); font-size: 14px; padding: 0; line-height: 1; }
  .tag-chip .x:hover { color: #c00; }

  .group { margin-bottom: 14px; }
  .group-parent { font-size: 14px; font-family: var(--font-serif); margin: 0 0 6px; display: flex; align-items: center; gap: 8px; }
  .group-parent a { color: var(--text-primary); text-decoration: none; }
  .group-parent a:hover { color: var(--accent); }

  .gear { background: none; border: none; cursor: pointer; color: var(--text-hint); font-size: 12px; padding: 0; line-height: 1; }
  .gear:hover { color: var(--accent); }

  .tag-editor { flex-basis: 100%; width: 100%; background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px; padding: 12px 14px; margin: 6px 0 10px; box-shadow: 0 1px 4px rgba(0,0,0,0.04); }
  .te-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; font-family: var(--font-serif); }
  .te-section { margin-top: 10px; }
  .te-label { font-size: 12px; color: var(--text-secondary); margin-bottom: 4px; font-weight: 500; }
  .tag-editor input { width: 100%; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); color: var(--text-primary); font-size: 14px; box-sizing: border-box; margin-bottom: 6px; }
  .inline-label { font-size: 12px; color: var(--text-hint); display: block; margin-top: 4px; }
  .alias-chips { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 6px; }
  .alias-chip { display: inline-flex; align-items: center; gap: 4px; padding: 3px 8px; border-radius: 12px; background: var(--bg-hover, #f5f5f5); border: 1px solid var(--border); font-size: 12px; }
  .alias-chip button { background: none; border: none; cursor: pointer; color: var(--text-hint); padding: 0; line-height: 1; font-size: 14px; }
  .alias-chip button:hover { color: #c00; }
  .alias-add { display: flex; gap: 6px; }
  .alias-add input { margin-bottom: 0; flex: 1; }
  .btn-ghost { background: none; border: none; cursor: pointer; color: var(--text-hint); font-size: 12px; padding: 2px 6px; }
  .btn-ghost:hover { color: var(--accent); }
</style>
