<script lang="ts">
  import { onMount } from 'svelte';
  import { listTagParents, addTagParent, removeTagParent, listTags, searchTags, createTagInline } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { Tag } from '../lib/types';

  type Edge = { parent_tag: string; child_tag: string };

  let edges = $state<Edge[]>([]);
  let tags = $state<Tag[]>([]);
  let loading = $state(true);
  let error = $state('');
  let isLoggedIn = $derived(!!getAuth());

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

  // Group edges by parent for display
  let groupedByParent = $derived.by(() => {
    const m = new Map<string, string[]>();
    for (const e of edges) {
      const arr = m.get(e.parent_tag) ?? [];
      arr.push(e.child_tag);
      m.set(e.parent_tag, arr);
    }
    return Array.from(m.entries())
      .map(([parent, children]) => ({ parent, children: children.sort() }))
      .sort((a, b) => a.parent.localeCompare(b.parent));
  });

  // All tags that are not a child of any edge — includes tags that have
  // children in the hierarchy AND isolated tags that were just created.
  let orphans = $derived.by(() => {
    const hasParent = new Set(edges.map(e => e.child_tag));
    return tags.map(tg => tg.id).filter(id => !hasParent.has(id)).sort();
  });
</script>

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
          <a href="/tag?id={encodeURIComponent(tag)}" class="tag-chip root">{tag}</a>
        {/each}
      </div>
    </section>

    <section class="groups">
      {#each groupedByParent as g}
        <div class="group">
          <h3 class="group-parent">
            <a href="/tag?id={encodeURIComponent(g.parent)}">{g.parent}</a>
            <span class="count">{g.children.length}</span>
          </h3>
          <div class="chip-row">
            {#each g.children as c}
              <span class="tag-chip">
                <a href="/tag?id={encodeURIComponent(c)}">{c}</a>
                {#if isLoggedIn}
                  <button class="x" onclick={() => removeEdge(g.parent, c)} title={t('hierarchy.remove')}>×</button>
                {/if}
              </span>
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
</style>
