<script lang="ts">
  import { getTag, getArticlesByTag, listSkills, lightSkill, unlightSkill, getArticleVotes, updateTagNames, listTagAliases, addTagAlias, removeTagAlias, listTagGroup, addTagGroupMember, removeTagGroupMember, setTagGroupRepresentative, mergeTagGroups, requestTagDeletion } from '../lib/api';
  import { authorName, tagName } from '../lib/display';
  import { t, LOCALES } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import type { Tag, Article, UserSkill, VoteSummary } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let tag = $state<Tag | null>(null);
  let articles = $state<Article[]>([]);
  let skills = $state<UserSkill[]>([]);
  let voteMap = $state(new Map<string, number>());
  let loading = $state(true);

  // A skill is considered lit when any sibling in the same group is lit,
  // since the group represents one concept across languages and aliases.
  let isLit = $derived(
    skills.some(s =>
      s.tag_id === id ||
      (tag && s.group_id != null && s.group_id === tag.group_id)
    )
  );
  let isLoggedIn = $derived(!!getAuth());

  // Edit state
  let showEdit = $state(false);
  let editNames = $state<Record<string, string>>({});
  let aliases = $state<string[]>([]);
  let newAlias = $state('');
  let editSaving = $state(false);
  let editError = $state('');

  // Group siblings — all tags in the same alias/translation group.
  let siblings = $state<Tag[]>([]);
  // Map: lang → member id that is the representative for that language.
  let representatives = $state<Record<string, string>>({});
  let newMemberId = $state('');
  let newMemberName = $state('');
  let newMemberLang = $state('zh');
  let mergeTargetId = $state('');
  let deleteReason = $state('');
  let deleteSubmitting = $state(false);
  let deleteSubmitted = $state(false);

  function openEdit() {
    if (!tag) return;
    editNames = { ...tag.names };
    for (const loc of LOCALES) {
      if (!(loc.code in editNames)) editNames[loc.code] = '';
    }
    if (!editNames.en?.trim()) editNames.en = tag.id;
    newAlias = '';
    newMemberId = '';
    newMemberName = '';
    newMemberLang = 'zh';
    editError = '';
    showEdit = true;
    listTagAliases(id).then(a => aliases = a).catch(() => {});
    listTagGroup(id).then((g: any) => {
      siblings = g.members ?? g;
      representatives = g.representatives ?? {};
    }).catch(() => {});
  }

  async function submitDeletionRequest() {
    const reason = deleteReason.trim();
    if (!reason) return;
    deleteSubmitting = true;
    try {
      await requestTagDeletion(id, reason);
      deleteSubmitted = true;
      deleteReason = '';
    } catch (err: any) {
      editError = err.message ?? String(err);
    } finally {
      deleteSubmitting = false;
    }
  }

  async function submitMergeGroup() {
    const other = mergeTargetId.trim();
    if (!other || other === id) return;
    if (!confirm(t('tags.confirmMerge').replace('{0}', other).replace('{1}', id))) return;
    try {
      await mergeTagGroups(id, other);
      mergeTargetId = '';
      const g: any = await listTagGroup(id);
      siblings = g.members ?? g;
      representatives = g.representatives ?? {};
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function promoteRepresentative(memberId: string, memberLang: string) {
    try {
      await setTagGroupRepresentative(id, memberId);
      representatives = { ...representatives, [memberLang]: memberId };
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function submitAddGroupMember() {
    const mid = newMemberId.trim();
    const mname = newMemberName.trim() || mid;
    if (!mid) return;
    try {
      await addTagGroupMember(id, { id: mid, name: mname, lang: newMemberLang });
      newMemberId = '';
      newMemberName = '';
      siblings = await listTagGroup(id);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function removeGroupMember(mid: string) {
    if (mid === id) {
      editError = 'Cannot remove the tag you are viewing from its group';
      return;
    }
    try {
      await removeTagGroupMember(id, mid);
      siblings = await listTagGroup(id);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function saveNames() {
    editSaving = true;
    editError = '';
    try {
      const cleaned = Object.fromEntries(Object.entries(editNames).filter(([_, v]) => v.trim()));
      const updated = await updateTagNames(id, cleaned);
      tag = updated;
    } catch (err: any) {
      editError = err.message ?? String(err);
    } finally {
      editSaving = false;
    }
  }

  async function submitAddAlias() {
    const a = newAlias.trim();
    if (!a) return;
    try {
      await addTagAlias(id, a);
      newAlias = '';
      aliases = await listTagAliases(id);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  async function removeAlias(alias: string) {
    try {
      await removeTagAlias(id, alias);
      aliases = await listTagAliases(id);
    } catch (err: any) {
      editError = err.message ?? String(err);
    }
  }

  // Top = sorted by vote score descending
  let topArticles = $derived(
    [...articles].sort((a, b) => (voteMap.get(b.at_uri) ?? 0) - (voteMap.get(a.at_uri) ?? 0)).slice(0, 20)
  );
  // Trending = recent articles weighted by recency (just sort by date for now)
  let trendingArticles = $derived(
    [...articles].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 20)
  );

  $effect(() => {
    loading = true;
    Promise.all([getTag(id), getArticlesByTag(id), listSkills()]).then(async ([t, arts, sk]) => {
      tag = t;
      document.title = `${t.name} — NightBoat`;
      articles = arts;
      skills = sk;

      // Fetch votes for all articles
      const votes = await Promise.all(arts.map(a => getArticleVotes(a.at_uri).catch(() => ({ score: 0 }) as VoteSummary)));
      const map = new Map<string, number>();
      arts.forEach((a, i) => map.set(a.at_uri, votes[i].score));
      voteMap = map;

      loading = false;
    });
  });

  async function toggleSkill() {
    if (isLit) {
      await unlightSkill(id);
    } else {
      await lightSkill(id);
    }
    skills = await listSkills();
  }

</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if tag}
  <div class="tag-header">
    <div class="tag-title-row">
      <h1>{tagName(tag.names, tag.name, tag.id)}</h1>
      <button class="skill-btn" class:lit={isLit} onclick={toggleSkill}>
        {isLit ? t('tags.mastered') : t('tags.light')}
      </button>
      {#if isLoggedIn}
        <button class="edit-btn" onclick={openEdit}>{t('tags.edit')}</button>
      {/if}
    </div>
    {#if tag.description}
      <p class="tag-desc">{tag.description}</p>
    {/if}
    <p class="tag-meta">{articles.length} {t('tags.articles')}</p>
  </div>

  {#if articles.length === 0}
    <p class="meta">{t('tags.empty')}</p>
  {:else}
    <div class="columns">
      <div class="column">
        <h2>Top Articles</h2>
        {#each topArticles as a}
          <a href="/article?uri={encodeURIComponent(a.at_uri)}" class="article-item">
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
          <a href="/article?uri={encodeURIComponent(a.at_uri)}" class="article-item">
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
{/if}

{#if showEdit && tag}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => showEdit = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('tags.editTitle')} — {tag.id}</h3>
      {#if editError}<p class="error-msg">{editError}</p>{/if}

      <h4>{t('tags.groupLabel')}</h4>
      <p class="hint">{t('tags.groupHint')}</p>
      <div class="sibling-list">
        {#each siblings as s (s.id)}
          {@const isRepForLang = representatives[s.lang] === s.id}
          <div class="sibling-row">
            <button
              class="rep-star"
              class:is-rep={isRepForLang}
              title={isRepForLang ? t('tags.currentRep') : t('tags.makeRep')}
              onclick={() => { if (!isRepForLang) promoteRepresentative(s.id, s.lang); }}
            >★</button>
            <span class="sibling-lang">{s.lang}</span>
            <a class="sibling-id" href="/tag?id={encodeURIComponent(s.id)}">{s.id}</a>
            {#if s.id !== id && !isRepForLang}
              <button class="sibling-rm" onclick={() => removeGroupMember(s.id)}>×</button>
            {:else if s.id === id}
              <span class="sibling-self">{t('tags.thisTag')}</span>
            {/if}
          </div>
        {/each}
      </div>
      <div class="sibling-add">
        <input class="sm-input" bind:value={newMemberId} placeholder={t('tags.newMemberIdPlaceholder')} />
        <input class="sm-input" bind:value={newMemberName} placeholder={t('tags.newMemberNamePlaceholder')} />
        <select class="sm-input" bind:value={newMemberLang}>
          {#each LOCALES as loc (loc.code)}
            <option value={loc.code}>{loc.code}</option>
          {/each}
        </select>
        <button class="btn" onclick={submitAddGroupMember}>{t('tags.addGroupMember')}</button>
      </div>

      <div class="sibling-add merge-row">
        <input class="sm-input" bind:value={mergeTargetId} placeholder={t('tags.mergePlaceholder')}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitMergeGroup(); } }} />
        <button class="btn" onclick={submitMergeGroup} disabled={!mergeTargetId.trim()}>{t('tags.mergeGroup')}</button>
      </div>
      <p class="hint">{t('tags.mergeHint')}</p>

      <h4 style="margin-top:18px">{t('tags.translationsLabel')}</h4>
      {#each [...LOCALES].sort((a, b) => (a.code === 'en' ? -1 : b.code === 'en' ? 1 : 0)) as loc (loc.code)}
        <label class="inline-label">
          {loc.label}{#if loc.code === 'en'} <span class="primary-marker">· {t('tags.primaryLocale')}</span>{/if}
        </label>
        <input bind:value={editNames[loc.code]} placeholder={loc.code === 'en' ? 'English name' : ''} />
      {/each}
      <button class="btn btn-primary" onclick={saveNames} disabled={editSaving}>
        {editSaving ? t('common.saving') : t('common.save')}
      </button>

      <h4 style="margin-top:18px">{t('tags.aliasesLabel')}</h4>
      <div class="alias-chips">
        {#each aliases as a}
          <span class="alias-chip">{a} <button onclick={() => removeAlias(a)}>×</button></span>
        {/each}
      </div>
      <div class="alias-add">
        <input bind:value={newAlias} placeholder={t('tags.aliasPlaceholder')}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitAddAlias(); } }} />
        <button class="btn" onclick={submitAddAlias}>{t('tags.addAlias')}</button>
      </div>

      <h4 style="margin-top:18px">{t('tags.deleteTitle')}</h4>
      <p class="hint">{t('tags.deleteHint')}</p>
      {#if deleteSubmitted}
        <p class="hint" style="color: var(--accent)">{t('tags.deleteSubmitted')}</p>
      {:else}
        <div class="sibling-add">
          <input class="sm-input" bind:value={deleteReason} placeholder={t('tags.deleteReasonPlaceholder')} style="flex:1" />
          <button class="btn" style="color:#c00;border-color:#c00" onclick={submitDeletionRequest} disabled={!deleteReason.trim() || deleteSubmitting}>
            {deleteSubmitting ? t('common.saving') : t('tags.requestDelete')}
          </button>
        </div>
      {/if}

      <div class="modal-actions">
        <button class="btn" onclick={() => showEdit = false}>{t('common.close')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tag-header {
    margin-bottom: 1.5rem;
  }
  .tag-title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .tag-title-row h1 {
    margin: 0;
  }
  .skill-btn {
    padding: 4px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: none;
    color: var(--accent);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .skill-btn:hover {
    background: var(--accent);
    color: white;
  }
  .skill-btn.lit {
    background: var(--accent);
    color: white;
  }
  .skill-btn.lit:hover {
    opacity: 0.85;
  }
  .tag-desc {
    margin: 0.5rem 0 0;
    color: var(--text-secondary);
    font-size: 15px;
  }
  .tag-meta {
    margin: 0.25rem 0 0;
    font-size: 13px;
    color: var(--text-hint);
  }

  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
  }
  @media (max-width: 700px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }

  .column h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1rem;
    padding-bottom: 0.25em;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.5rem;
    margin-top: 0;
  }

  .article-score {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-hint);
    min-width: 28px;
    text-align: center;
    flex-shrink: 0;
  }
  .article-info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 8px;
    text-decoration: none;
    color: inherit;
    background: var(--bg-white);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .article-item:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .article-title {
    font-family: var(--font-serif);
    font-size: 15px;
    color: var(--text-primary);
    line-height: 1.35;
  }
  .article-item:hover .article-title {
    color: var(--accent);
  }
  .article-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 3px;
    line-height: 1.45;
  }
  .article-meta {
    font-size: 12px;
    color: var(--text-hint);
    margin-top: 4px;
  }

  .edit-btn { padding: 4px 10px; font-size: 12px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); cursor: pointer; color: var(--text-secondary); }
  .edit-btn:hover { color: var(--accent); border-color: var(--accent); }
  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.35); display: flex; align-items: center; justify-content: center; z-index: 999; }
  .modal { background: var(--bg-white); border-radius: 8px; padding: 20px 24px; min-width: 400px; max-width: 560px; max-height: 85vh; overflow-y: auto; box-shadow: 0 8px 32px rgba(0,0,0,0.15); }
  .modal h3 { margin: 0 0 12px; font-family: var(--font-serif); }
  .modal h4 { margin: 14px 0 6px; font-size: 13px; color: var(--text-secondary); font-weight: 500; }
  .modal input { width: 100%; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); color: var(--text-primary); font-size: 14px; box-sizing: border-box; margin-bottom: 6px; }
  .inline-label { font-size: 12px; color: var(--text-hint); display: block; margin-top: 8px; }
  .primary-marker { color: var(--accent); font-weight: 500; text-transform: lowercase; }
  .btn { padding: 6px 14px; border-radius: 4px; border: 1px solid var(--border); background: var(--bg-white); cursor: pointer; font-size: 14px; }
  .btn-primary { background: var(--accent); color: white; border-color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .modal-actions { margin-top: 16px; text-align: right; }
  .alias-chips { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
  .alias-chip { display: inline-flex; align-items: center; gap: 4px; padding: 3px 8px; border-radius: 12px; background: var(--bg-hover, #f5f5f5); border: 1px solid var(--border); font-size: 12px; }
  .alias-chip button { background: none; border: none; cursor: pointer; color: var(--text-hint); padding: 0; line-height: 1; font-size: 14px; }
  .alias-chip button:hover { color: #c00; }
  .hint { font-size: 12px; color: var(--text-hint); margin: 4px 0 8px; }
  .sibling-list { display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px; }
  .sibling-row { display: flex; align-items: center; gap: 8px; padding: 3px 0; font-size: 13px; }
  .sibling-lang { font-family: monospace; font-size: 11px; padding: 1px 6px; border: 1px solid var(--border); border-radius: 3px; color: var(--text-hint); }
  .sibling-id { color: var(--accent); text-decoration: none; }
  .sibling-id:hover { text-decoration: underline; }
  .sibling-self { font-size: 11px; color: var(--text-hint); font-style: italic; }
  .sibling-rm { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 4px; }
  .sibling-rm:hover { color: #c00; }
  .rep-star { background: none; border: none; cursor: pointer; font-size: 16px; line-height: 1; padding: 0 2px; color: var(--border); }
  .rep-star:hover { color: #d97706; }
  .rep-star.is-rep { color: #d97706; cursor: default; }
  .rep-star.is-rep:hover { color: #d97706; }
  .sibling-add { display: flex; gap: 4px; margin-top: 6px; }
  .sibling-add .sm-input { flex: 1; padding: 4px 6px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; }
  .sibling-add select.sm-input { flex: 0 0 64px; }
  .merge-row { margin-top: 10px; padding-top: 10px; border-top: 1px dashed var(--border); }
  .alias-add { display: flex; gap: 6px; }
  .alias-add input { margin-bottom: 0; flex: 1; }
  .error-msg { background: #fee; color: #c00; padding: 6px 10px; border-radius: 4px; font-size: 13px; }
</style>
