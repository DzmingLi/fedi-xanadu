<script lang="ts">
  import {
    getTag, getArticlesByTag, getArticlesRelatedByTag, listSkills, lightSkill, unlightSkill, getArticleVotes,
    listTagGroup, addTagName, removeTagName,
    mergeTagGroups, deleteTag, setMyNamePref, clearMyNamePref,
    listTagHistory, type TagAuditEntry,
    listTagParents, addTagParent, removeTagParent, lookupTag,
  } from '../lib/api';
  import { navigate } from '../lib/router';
  import { authorName } from '../lib/display';
  import { contentHref } from '../lib/utils';
  import { tagStore } from '../lib/tagStore.svelte';
  $effect(() => { tagStore.ensure(); });
  import { t, LOCALES } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import type { Tag, Article, UserSkill, VoteSummary } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let tag = $state<Tag | null>(null);
  let articles = $state<Article[]>([]);
  let relatedArticles = $state<Article[]>([]);
  let skills = $state<UserSkill[]>([]);
  let voteMap = $state(new Map<string, number>());
  let loading = $state(true);

  // Skills are keyed on concept tag_id; the page's `id` may be a name
  // id OR a tag id, so we compare against tag?.tag_id (the resolved
  // concept) for robustness.
  let isLit = $derived(
    skills.some(s =>
      s.tag_id === id ||
      (tag && s.tag_id != null && s.tag_id === tag.tag_id)
    )
  );
  let isLoggedIn = $derived(!!getAuth());

  // Edit panel state
  let showEdit = $state(false);
  let editError = $state('');
  let siblings = $state<Tag[]>([]);
  let newName = $state('');
  let newLang = $state('zh');
  let mergeFromId = $state('');
  let deleting = $state(false);
  let history = $state<TagAuditEntry[]>([]);
  // Parent / child concept ids attached to this tag.
  let parents = $state<string[]>([]);
  let children = $state<string[]>([]);
  let newParent = $state('');
  let newChild = $state('');

  async function loadEdges() {
    if (!tag) return;
    try {
      const all = await listTagParents();
      parents = all.filter(e => e.child_tag === tag!.tag_id).map(e => e.parent_tag);
      children = all.filter(e => e.parent_tag === tag!.tag_id).map(e => e.child_tag);
    } catch { /* */ }
  }

  async function asTagId(input: string): Promise<string | null> {
    const s = input.trim();
    if (!s) return null;
    if (s.startsWith('tg-')) return s;
    try { return (await lookupTag(s)).tag_id; }
    catch { editError = t('books.tagNotFound').replace('{name}', s); return null; }
  }

  async function submitAddParent() {
    if (!tag) return;
    const pid = await asTagId(newParent);
    if (!pid) return;
    if (pid === tag.tag_id) { editError = t('hierarchy.sameTag'); return; }
    try {
      await addTagParent(pid, tag.tag_id);
      newParent = '';
      editError = '';
      await loadEdges();
    } catch (err: any) { editError = err.message ?? String(err); }
  }

  async function submitAddChild() {
    if (!tag) return;
    const cid = await asTagId(newChild);
    if (!cid) return;
    if (cid === tag.tag_id) { editError = t('hierarchy.sameTag'); return; }
    try {
      await addTagParent(tag.tag_id, cid);
      newChild = '';
      editError = '';
      await loadEdges();
    } catch (err: any) { editError = err.message ?? String(err); }
  }

  async function unparent(parentId: string) {
    if (!tag) return;
    if (!confirm(t('tags.confirmRemoveEdge')
        .replace('{parent}', tagStore.localize(parentId))
        .replace('{child}', tagStore.localize(tag.tag_id)))) return;
    try {
      await removeTagParent(parentId, tag.tag_id);
      await loadEdges();
    } catch (err: any) { editError = err.message ?? String(err); }
  }

  async function unchild(childId: string) {
    if (!tag) return;
    if (!confirm(t('tags.confirmRemoveEdge')
        .replace('{parent}', tagStore.localize(tag.tag_id))
        .replace('{child}', tagStore.localize(childId)))) return;
    try {
      await removeTagParent(tag.tag_id, childId);
      await loadEdges();
    } catch (err: any) { editError = err.message ?? String(err); }
  }

  function conceptId(): string | null { return tag?.tag_id ?? null; }

  function openEdit() {
    if (!tag) return;
    newName = '';
    newLang = 'zh';
    editError = '';
    showEdit = true;
    listTagGroup(id).then((g) => { siblings = g.members; }).catch(() => {});
    listTagHistory(id).then((h) => { history = h; }).catch(() => {});
    loadEdges();
  }

  async function refreshHistory() {
    try { history = await listTagHistory(id); } catch { /* */ }
  }

  async function submitAddName() {
    const name = newName.trim();
    if (!name) return;
    try {
      await addTagName(id, name, newLang);
      newName = '';
      const g = await listTagGroup(id);
      siblings = g.members;
      await tagStore.refresh();
      await refreshHistory();
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function removeName(name_id: string) {
    if (siblings.length <= 1) {
      editError = t('tags.cantRemoveLast');
      return;
    }
    try {
      await removeTagName(id, name_id);
      const g = await listTagGroup(id);
      siblings = g.members;
      await tagStore.refresh();
      await refreshHistory();
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function pickAsPreferred(name_id: string) {
    const c = conceptId();
    if (!c) return;
    try {
      await setMyNamePref(id, name_id);
      tagStore.setMyPref(c, name_id);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function clearPreference() {
    const c = conceptId();
    if (!c) return;
    try {
      await clearMyNamePref(id);
      tagStore.setMyPref(c, null);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function submitMergeGroup() {
    const other = mergeFromId.trim();
    if (!other) return;
    if (!confirm(t('tags.confirmMerge').replace('{0}', other).replace('{1}', id))) return;
    try {
      await mergeTagGroups(id, other);
      mergeFromId = '';
      const g = await listTagGroup(id);
      siblings = g.members;
      await tagStore.refresh();
      await refreshHistory();
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function confirmDelete() {
    if (!tag) return;
    if (!confirm(t('tags.confirmDelete').replace('{name}', tagStore.localize(tag.tag_id)))) return;
    deleting = true;
    try {
      await deleteTag(id);
      navigate('/hierarchy');
    } catch (err: any) {
      editError = err.message ?? String(err);
      deleting = false;
    }
  }

  let topArticles = $derived(
    [...articles].sort((a, b) => (voteMap.get(b.at_uri) ?? 0) - (voteMap.get(a.at_uri) ?? 0)).slice(0, 20)
  );
  let trendingArticles = $derived(
    [...articles].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 20)
  );

  $effect(() => {
    loading = true;
    Promise.all([getTag(id), getArticlesByTag(id), getArticlesRelatedByTag(id), listSkills()]).then(async ([resp, arts, rel, sk]) => {
      tag = resp as any;
      document.title = `${resp.name} — NightBoat`;
      articles = arts;
      relatedArticles = rel;
      skills = sk;
      // Populate parent/child edges so the main page can show them
      // without requiring the user to open the edit modal.
      loadEdges();

      const votes = await Promise.all(arts.map(a => getArticleVotes(a.at_uri).catch(() => ({ score: 0 }) as VoteSummary)));
      const map = new Map<string, number>();
      arts.forEach((a, i) => map.set(a.at_uri, votes[i].score));
      voteMap = map;

      loading = false;

      if (typeof window !== 'undefined') {
        const params = new URLSearchParams(window.location.search);
        if (params.get('edit') === '1') openEdit();
      }
    });
  });

  async function toggleSkill() {
    const c = conceptId();
    if (!c) return;
    if (isLit) await unlightSkill(c);
    else await lightSkill(c);
    skills = await listSkills();
  }

  // My current preferred name id for this concept.
  let myPref = $derived(tag ? tagStore.myPref(tag.tag_id) : undefined);
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if tag}
  <div class="tag-header">
    <div class="tag-title-row">
      <h1>{tagStore.localize(tag.tag_id)}</h1>
      <button class="skill-btn" class:lit={isLit} onclick={toggleSkill}>
        {isLit ? t('tags.mastered') : t('tags.light')}
      </button>
      {#if isLoggedIn}
        <button class="edit-btn" onclick={openEdit}>{t('tags.edit')}</button>
      {/if}
    </div>
    <p class="tag-meta">{articles.length} {t('tags.articles')}</p>
  </div>

  {#if parents.length > 0 || children.length > 0}
    <section class="edges-overview">
      {#if parents.length > 0}
        <div class="edges-row">
          <span class="edges-label">{t('tags.parentsLabel')}</span>
          {#each parents as p (p)}
            <a class="edge-chip parent" href="/tag?id={encodeURIComponent(p)}">{tagStore.localize(p)}</a>
          {/each}
        </div>
      {/if}
      {#if children.length > 0}
        <div class="edges-row">
          <span class="edges-label">{t('tags.childrenLabel')}</span>
          {#each children as c (c)}
            <a class="edge-chip child" href="/tag?id={encodeURIComponent(c)}">{tagStore.localize(c)}</a>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  {#if articles.length === 0 && relatedArticles.length === 0}
    <p class="meta">{t('tags.empty')}</p>
  {:else if articles.length === 0}
    <!-- teaches empty but related non-empty: just show related section below -->
  {:else}
    <div class="columns">
      <div class="column">
        <h2>Top Articles</h2>
        {#each topArticles as a}
          <a href={contentHref(a.at_uri, a.kind, a.question_uri)} class="article-item">
            <span class="article-score">{voteMap.get(a.at_uri) ?? 0}</span>
            <div class="article-info">
              <span class="article-title">{a.title}</span>
              {#if a.summary}
                <span class="article-desc">{a.summary}</span>
              {/if}
              <span class="article-meta">{authorName(a)} &middot; {a.created_at.split(' ')[0]}</span>
            </div>
          </a>
        {/each}
      </div>

      <div class="column">
        <h2>Trending</h2>
        {#each trendingArticles as a}
          <a href={contentHref(a.at_uri, a.kind, a.question_uri)} class="article-item">
            <span class="article-score">{voteMap.get(a.at_uri) ?? 0}</span>
            <div class="article-info">
              <span class="article-title">{a.title}</span>
              {#if a.summary}
                <span class="article-desc">{a.summary}</span>
              {/if}
              <span class="article-meta">{authorName(a)} &middot; {a.created_at.split(' ')[0]}</span>
            </div>
          </a>
        {/each}
      </div>
    </div>
  {/if}

  {#if relatedArticles.length > 0}
    <section class="related-section">
      <h2>{t('tags.relatedContent')}</h2>
      <p class="hint">{t('tags.relatedContentHint')}</p>
      {#each relatedArticles as a}
        <a href={contentHref(a.at_uri, a.kind, a.question_uri)} class="article-item">
          <div class="article-info">
            <span class="article-title">{a.title}</span>
            {#if a.summary}
              <span class="article-desc">{a.summary}</span>
            {/if}
            <span class="article-meta">{authorName(a)} &middot; {a.created_at.split(' ')[0]}</span>
          </div>
        </a>
      {/each}
    </section>
  {/if}
{/if}

{#if showEdit && tag}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => showEdit = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('tags.editTitle')}</h3>
      {#if editError}<p class="error-msg">{editError}</p>{/if}

      <h4>{t('tags.namesLabel')}</h4>
      <p class="hint">{t('tags.namesHint')}</p>
      <div class="sibling-list">
        {#each siblings as s (s.id)}
          {@const isMyPref = myPref === s.id}
          <div class="sibling-row">
            <button
              class="pref-star"
              class:is-pref={isMyPref}
              title={isMyPref ? t('tags.currentPref') : t('tags.makePref')}
              onclick={() => { if (isMyPref) clearPreference(); else pickAsPreferred(s.id); }}
            >★</button>
            <span class="sibling-lang">{s.lang}</span>
            <span class="sibling-name">{s.name}</span>
            {#if siblings.length > 1}
              <button class="sibling-rm" title={t('tags.removeName')} onclick={() => removeName(s.id)}>×</button>
            {/if}
          </div>
        {/each}
      </div>
      <div class="sibling-add">
        <input class="sm-input" bind:value={newName} placeholder={t('tags.newNamePlaceholder')} style="flex:1"
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitAddName(); } }} />
        <select class="sm-input" bind:value={newLang}>
          {#each LOCALES as loc (loc.code)}
            <option value={loc.code}>{loc.code}</option>
          {/each}
        </select>
        <button class="btn" onclick={submitAddName}>{t('tags.addName')}</button>
      </div>

      <h4 style="margin-top:18px">{t('tags.parentsLabel')}</h4>
      <p class="hint">{t('tags.parentsHint')}</p>
      <div class="sibling-list">
        {#each parents as p (p)}
          <div class="sibling-row">
            <a class="sibling-name" href="/tag?id={encodeURIComponent(p)}">{tagStore.localize(p)}</a>
            <button class="sibling-rm" onclick={() => unparent(p)} title={t('hierarchy.remove')}>×</button>
          </div>
        {/each}
      </div>
      <div class="sibling-add">
        <input class="sm-input" bind:value={newParent} placeholder={t('tags.parentPlaceholder')} style="flex:1"
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitAddParent(); } }} />
        <button class="btn" onclick={submitAddParent}>{t('hierarchy.add')}</button>
      </div>

      <h4 style="margin-top:18px">{t('tags.childrenLabel')}</h4>
      <p class="hint">{t('tags.childrenHint')}</p>
      <div class="sibling-list">
        {#each children as c (c)}
          <div class="sibling-row">
            <a class="sibling-name" href="/tag?id={encodeURIComponent(c)}">{tagStore.localize(c)}</a>
            <button class="sibling-rm" onclick={() => unchild(c)} title={t('hierarchy.remove')}>×</button>
          </div>
        {/each}
      </div>
      <div class="sibling-add">
        <input class="sm-input" bind:value={newChild} placeholder={t('tags.childPlaceholder')} style="flex:1"
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitAddChild(); } }} />
        <button class="btn" onclick={submitAddChild}>{t('hierarchy.add')}</button>
      </div>

      <div class="sibling-add merge-row">
        <input class="sm-input" bind:value={mergeFromId} placeholder={t('tags.mergePlaceholder')}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitMergeGroup(); } }} />
        <button class="btn" onclick={submitMergeGroup} disabled={!mergeFromId.trim()}>{t('tags.mergeGroup')}</button>
      </div>
      <p class="hint">{t('tags.mergeHint')}</p>

      <h4 style="margin-top:18px">{t('tags.deleteTitle')}</h4>
      <p class="hint">{t('tags.deleteHint')}</p>
      <div class="sibling-add">
        <button class="btn" style="color:#c00;border-color:#c00" onclick={confirmDelete} disabled={deleting}>
          {deleting ? t('common.saving') : t('tags.deleteNow')}
        </button>
      </div>

      <h4 style="margin-top:18px">{t('tags.historyTitle')}</h4>
      {#if history.length === 0}
        <p class="hint">{t('tags.historyEmpty')}</p>
      {:else}
        <ul class="history">
          {#each history as h (h.id)}
            <li class="history-row">
              <span class="history-when">{h.at.split('.')[0].replace('T', ' ')}</span>
              <span class="history-action action-{h.action}">{t(`tags.action.${h.action}`)}</span>
              {#if h.name}<span class="history-name">{h.name}</span>{/if}
              {#if h.lang}<span class="history-lang">{h.lang}</span>{/if}
              {#if h.action === 'merge_tag' && h.merged_into}
                <span class="history-arrow">→</span>
                <span class="history-name">{h.merged_into}</span>
              {/if}
              <span class="history-actor">{h.actor_display_name || h.actor_handle || h.actor_did}</span>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="modal-actions">
        <button class="btn" onclick={() => showEdit = false}>{t('common.close')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tag-header { margin-bottom: 1.5rem; }
  .tag-title-row { display: flex; align-items: center; gap: 1rem; }
  .tag-title-row h1 { margin: 0; }
  .skill-btn {
    padding: 4px 14px; font-size: 13px;
    border: 1px solid var(--accent); border-radius: 3px;
    background: none; color: var(--accent); cursor: pointer;
    transition: all 0.15s; white-space: nowrap;
  }
  .skill-btn:hover { background: var(--accent); color: white; }
  .skill-btn.lit { background: var(--accent); color: white; }
  .skill-btn.lit:hover { opacity: 0.85; }
  .tag-meta { margin: 0.25rem 0 0; font-size: 13px; color: var(--text-hint); }

  .columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
  @media (max-width: 700px) { .columns { grid-template-columns: 1fr; } }
  .edges-overview { margin-bottom: 1.25rem; display: flex; flex-direction: column; gap: 6px; }
  .edges-row { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .edges-label { font-size: 12px; color: var(--text-hint); min-width: 60px; }
  .edge-chip { display: inline-flex; align-items: center; padding: 2px 10px; border-radius: 12px; font-size: 12px; text-decoration: none; border: 1px solid var(--border); background: var(--bg-white); color: var(--text-primary); transition: border-color 0.15s; }
  .edge-chip:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }
  .edge-chip.parent { background: var(--bg-hover, #f5f5f5); }
  .related-section { margin-top: 2rem; padding-top: 1.5rem; border-top: 1px dashed var(--border); }
  .related-section h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1rem; margin: 0 0 0.25rem; }
  .related-section .hint { color: var(--text-secondary); font-size: 13px; margin: 0 0 0.75rem; }
  .column h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1rem; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); margin-bottom: 0.5rem; margin-top: 0; }

  .article-score { font-size: 14px; font-weight: 600; color: var(--text-hint); min-width: 28px; text-align: center; flex-shrink: 0; }
  .article-info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .article-item { display: flex; align-items: flex-start; gap: 10px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 8px; text-decoration: none; color: inherit; background: var(--bg-white); transition: border-color 0.15s, box-shadow 0.15s; }
  .article-item:hover { border-color: var(--border-strong); box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04); text-decoration: none; }
  .article-title { font-family: var(--font-serif); font-size: 15px; color: var(--text-primary); line-height: 1.35; }
  .article-item:hover .article-title { color: var(--accent); }
  .article-desc { font-size: 13px; color: var(--text-secondary); margin-top: 3px; line-height: 1.45; }
  .article-meta { font-size: 12px; color: var(--text-hint); margin-top: 4px; }

  .edit-btn { padding: 4px 10px; font-size: 12px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); cursor: pointer; color: var(--text-secondary); }
  .edit-btn:hover { color: var(--accent); border-color: var(--accent); }
  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.35); display: flex; align-items: center; justify-content: center; z-index: 999; }
  .modal { background: var(--bg-white); border-radius: 8px; padding: 20px 24px; min-width: 400px; max-width: 560px; max-height: 85vh; overflow-y: auto; box-shadow: 0 8px 32px rgba(0,0,0,0.15); }
  .modal h3 { margin: 0 0 12px; font-family: var(--font-serif); }
  .modal h4 { margin: 14px 0 6px; font-size: 13px; color: var(--text-secondary); font-weight: 500; }
  .modal input { width: 100%; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); color: var(--text-primary); font-size: 14px; box-sizing: border-box; margin-bottom: 6px; }
  .btn { padding: 6px 14px; border-radius: 4px; border: 1px solid var(--border); background: var(--bg-white); cursor: pointer; font-size: 14px; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .modal-actions { margin-top: 16px; text-align: right; }
  .hint { font-size: 12px; color: var(--text-hint); margin: 4px 0 8px; }
  .sibling-list { display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px; }
  .sibling-row { display: flex; align-items: center; gap: 8px; padding: 3px 0; font-size: 13px; }
  .sibling-lang { font-family: monospace; font-size: 11px; padding: 1px 6px; border: 1px solid var(--border); border-radius: 3px; color: var(--text-hint); }
  .sibling-name { color: var(--text-primary); flex: 1; }
  .sibling-rm { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 4px; }
  .sibling-rm:hover { color: #c00; }
  .pref-star { background: none; border: none; cursor: pointer; font-size: 16px; line-height: 1; padding: 0 2px; color: var(--border); }
  .pref-star:hover { color: #d97706; }
  .pref-star.is-pref { color: #d97706; }
  .sibling-add { display: flex; gap: 4px; margin-top: 6px; }
  .sibling-add .sm-input { flex: 1; padding: 4px 6px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; }
  .sibling-add select.sm-input { flex: 0 0 64px; }
  .merge-row { margin-top: 10px; padding-top: 10px; border-top: 1px dashed var(--border); }
  .error-msg { background: #fee; color: #c00; padding: 6px 10px; border-radius: 4px; font-size: 13px; }
  .history { list-style: none; padding: 0; margin: 0; font-size: 12px; }
  .history-row { display: flex; align-items: center; gap: 6px; padding: 3px 0; color: var(--text-secondary); }
  .history-when { font-family: monospace; color: var(--text-hint); white-space: nowrap; }
  .history-action { font-size: 11px; padding: 1px 6px; border-radius: 3px; background: var(--bg-hover, #f5f5f5); }
  .action-create_tag { background: #d1fae5; color: #065f46; }
  .action-add_name { background: #dbeafe; color: #1e40af; }
  .action-remove_name { background: #fee2e2; color: #991b1b; }
  .action-merge_tag { background: #fef3c7; color: #92400e; }
  .history-name { color: var(--text-primary); }
  .history-lang { font-family: monospace; color: var(--text-hint); }
  .history-arrow { color: var(--text-hint); }
  .history-actor { margin-left: auto; color: var(--text-hint); font-size: 11px; }
</style>
