<script lang="ts">
  import {
    getPublication, listPublicationContent, listPublicationMembers,
    getPublicationViewerState, followPublication, unfollowPublication,
    acceptPublicationInvite, leavePublication, deletePublication,
    addPublicationMember, removePublicationMember, removePublicationContent,
    addPublicationContent, getArticlesByDid, listSeries,
  } from '../lib/api';
  import type { Publication, PublicationMember, PublicationContentItem, PublicationViewerState, Article, Series } from '../lib/types';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import PostCard from '../lib/components/PostCard.svelte';

  let { slug }: { slug: string } = $props();

  let pub = $state<Publication | null>(null);
  let content = $state<PublicationContentItem[]>([]);
  let members = $state<PublicationMember[]>([]);
  let viewer = $state<PublicationViewerState | null>(null);
  let loading = $state(true);
  let filter = $state<'all' | 'articles' | 'series'>('all');
  let addingMember = $state(false);
  let newMemberDid = $state('');
  let newMemberRole = $state<'writer' | 'editor'>('writer');
  let pickingContent = $state(false);
  let myArticles = $state<Article[]>([]);
  let mySeries = $state<Series[]>([]);

  $effect(() => { void load(); });

  async function load() {
    loading = true;
    try {
      const [p, c, m] = await Promise.all([
        getPublication(slug),
        listPublicationContent(slug),
        listPublicationMembers(slug),
      ]);
      pub = p;
      content = c;
      members = m;
      if (getAuth()) viewer = await getPublicationViewerState(slug);
    } catch (e) {
      console.error(e);
    }
    loading = false;
  }

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const cur = getLocale();
    return field[cur] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  let filteredContent = $derived(
    filter === 'all' ? content : content.filter(c => c.kind === filter.slice(0, -1))
  );

  let isMember = $derived(viewer?.role != null);
  let canEditSettings = $derived(viewer?.role === 'owner' || viewer?.role === 'editor');
  let canManageMembers = $derived(viewer?.role === 'owner');
  let needsAccept = $derived(isMember && viewer?.membership_confirmed === false);

  async function toggleFollow() {
    if (!viewer) return;
    if (viewer.is_following) {
      await unfollowPublication(slug);
      viewer = { ...viewer, is_following: false };
    } else {
      await followPublication(slug);
      viewer = { ...viewer, is_following: true };
    }
  }

  async function acceptInvite() {
    await acceptPublicationInvite(slug);
    if (viewer) viewer = { ...viewer, membership_confirmed: true };
  }

  async function leave() {
    if (!confirm(t('publications.leave'))) return;
    await leavePublication(slug);
    window.location.href = '/publications';
  }

  async function doDelete() {
    if (!confirm(t('publications.deleteConfirm'))) return;
    await deletePublication(slug);
    window.location.href = '/publications';
  }

  async function inviteMember() {
    if (!newMemberDid.trim()) return;
    await addPublicationMember(slug, newMemberDid.trim(), newMemberRole);
    newMemberDid = '';
    members = await listPublicationMembers(slug);
    addingMember = false;
  }

  async function kick(did: string) {
    if (!confirm(`Remove ${did}?`)) return;
    await removePublicationMember(slug, did);
    members = await listPublicationMembers(slug);
  }

  async function detachContent(contentUri: string) {
    if (!confirm(t('publications.removeContent'))) return;
    await removePublicationContent(slug, contentUri);
    content = await listPublicationContent(slug);
  }

  async function openPicker() {
    const me = getAuth();
    if (!me) return;
    pickingContent = true;
    const [arts, ser] = await Promise.all([getArticlesByDid(me.did), listSeries()]);
    myArticles = arts;
    // Series API returns all series; filter to the user's own.
    mySeries = ser.filter(s => s.created_by === me.did);
  }

  async function attach(kind: 'article' | 'series', uri: string) {
    try {
      await addPublicationContent(slug, uri, kind);
      content = await listPublicationContent(slug);
      pickingContent = false;
    } catch (e) {
      alert(e instanceof Error ? e.message : String(e));
    }
  }

  let attachedUris = $derived(new Set(content.map(c => c.article?.at_uri || c.series?.id).filter(Boolean)));
</script>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if !pub}
  <p class="meta">{t('common.noResults')}</p>
{:else}
  <div class="pub-detail">
    {#if pub.cover_url}
      <div class="pub-banner" style="background-image: url({pub.cover_url})"></div>
    {:else}
      <div class="pub-banner placeholder"></div>
    {/if}

    <header class="pub-head">
      <div class="titles">
        <h1>{loc(pub.title_i18n) || pub.id}</h1>
        {#if loc(pub.description_i18n)}
          <p class="desc">{loc(pub.description_i18n)}</p>
        {/if}
        <div class="head-meta">
          <span>{t('publications.memberCount', members.length)}</span>
          <span>{t('publications.contentCount', content.length)}</span>
        </div>
      </div>
      <div class="head-actions">
        {#if needsAccept}
          <button class="primary" onclick={acceptInvite}>{t('publications.acceptInvite')}</button>
        {:else if viewer}
          <button class:primary={!viewer.is_following} class="follow-btn" onclick={toggleFollow}>
            {viewer.is_following ? t('publications.unfollow') : t('publications.follow')}
          </button>
        {/if}
        {#if isMember && viewer?.role !== 'owner'}
          <button class="ghost" onclick={leave}>{t('publications.leave')}</button>
        {/if}
        {#if viewer?.role === 'owner'}
          <button class="ghost danger" onclick={doDelete}>{t('publications.delete')}</button>
        {/if}
      </div>
    </header>

    <div class="layout">
      <main>
        <div class="filter-bar">
          <button class:active={filter === 'all'} onclick={() => { filter = 'all'; }}>
            {t('publications.filter.all')}
          </button>
          <button class:active={filter === 'articles'} onclick={() => { filter = 'articles'; }}>
            {t('publications.filter.articles')}
          </button>
          <button class:active={filter === 'series'} onclick={() => { filter = 'series'; }}>
            {t('publications.filter.series')}
          </button>
          {#if isMember && viewer?.membership_confirmed}
            <button class="add-content-btn" onclick={openPicker}>
              + {t('publications.addContent')}
            </button>
          {/if}
        </div>

        {#if filteredContent.length === 0}
          <p class="meta">{t('publications.emptyContent')}</p>
        {:else}
          <div class="content-list">
            {#each filteredContent as item}
              {#if item.kind === 'article' && item.article}
                <div class="content-wrap">
                  <PostCard article={item.article} />
                  {#if canEditSettings}
                    <button class="detach" onclick={() => detachContent(item.article!.at_uri)} title={t('publications.removeContent')}>×</button>
                  {/if}
                </div>
              {:else if item.kind === 'series' && item.series}
                {@const s = item.series}
                <a class="series-card" href="/series?id={encodeURIComponent(s.id)}">
                  <span class="series-badge">{t('home.series')}</span>
                  <h3>{s.title}</h3>
                  {#if s.summary}<p class="desc">{s.summary}</p>{/if}
                </a>
              {/if}
            {/each}
          </div>
        {/if}
      </main>

      <aside>
        <div class="card">
          <h3>{t('publications.members')}</h3>
          <ul class="member-list">
            {#each members as m}
              <li>
                <a href="/profile?did={encodeURIComponent(m.did)}">
                  {#if m.avatar_url}
                    <img src={m.avatar_url} alt="" />
                  {:else}
                    <div class="avatar-ph">{(m.handle || m.did).charAt(0).toUpperCase()}</div>
                  {/if}
                  <span class="name">{m.display_name || m.handle || m.did.slice(0, 16)}</span>
                </a>
                <span class="role">
                  {t(`publications.role.${m.role}`)}
                  {#if !m.membership_at_uri && m.role !== 'owner'}
                    <span class="pending">({t('publications.role.unconfirmed')})</span>
                  {/if}
                </span>
                {#if canManageMembers && m.role !== 'owner'}
                  <button class="kick" onclick={() => kick(m.did)}>×</button>
                {/if}
              </li>
            {/each}
          </ul>
          {#if canManageMembers}
            {#if addingMember}
              <div class="add-member">
                <input bind:value={newMemberDid} placeholder={t('publications.memberHandle')} />
                <select bind:value={newMemberRole}>
                  <option value="writer">{t('publications.role.writer')}</option>
                  <option value="editor">{t('publications.role.editor')}</option>
                </select>
                <button onclick={() => { addingMember = false; }}>{t('common.cancel')}</button>
                <button class="primary" onclick={inviteMember}>{t('common.confirm')}</button>
              </div>
            {:else}
              <button class="ghost block" onclick={() => { addingMember = true; }}>{t('publications.addMember')}</button>
            {/if}
          {/if}
        </div>
      </aside>
    </div>
  </div>
{/if}

{#if pickingContent}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => { pickingContent = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('publications.addContent')}</h3>
      <section>
        <h4>{t('publications.filter.articles')}</h4>
        {#if myArticles.length === 0}
          <p class="meta">{t('home.noArticles')}</p>
        {:else}
          <ul class="pick-list">
            {#each myArticles as a}
              <li>
                <span class="pick-title">{a.title}</span>
                {#if attachedUris.has(a.at_uri)}
                  <span class="pick-done">✓</span>
                {:else}
                  <button onclick={() => attach('article', a.at_uri)}>+</button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
      <section>
        <h4>{t('publications.filter.series')}</h4>
        {#if mySeries.length === 0}
          <p class="meta">—</p>
        {:else}
          <ul class="pick-list">
            {#each mySeries as s}
              <li>
                <span class="pick-title">{s.title}</span>
                {#if attachedUris.has(s.id)}
                  <span class="pick-done">✓</span>
                {:else}
                  <button onclick={() => attach('series', s.id)}>+</button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
      <div class="actions">
        <button onclick={() => { pickingContent = false; }}>{t('common.close') || t('common.cancel')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .pub-detail { max-width: 1080px; margin: 0 auto; }
  .pub-banner {
    width: 100%; aspect-ratio: 4 / 1; max-height: 240px;
    background-size: cover; background-position: center;
    border-radius: 0 0 8px 8px;
  }
  .pub-banner.placeholder { background: linear-gradient(135deg, #f3f3ee, #e8e8e0); }
  .pub-head {
    padding: 1rem; display: flex; gap: 24px; align-items: flex-start; justify-content: space-between;
    flex-wrap: wrap;
  }
  .titles h1 { margin: 0; font-family: var(--font-serif); font-weight: 400; font-size: 1.6rem; }
  .desc { margin: 6px 0 8px; color: var(--text-secondary); font-size: 0.9rem; }
  .head-meta { display: flex; gap: 12px; font-size: 12px; color: var(--text-hint); }
  .head-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .head-actions button {
    padding: 6px 14px; font-size: 13px; border: 1px solid var(--border);
    border-radius: 4px; background: none; cursor: pointer;
  }
  .head-actions button.primary { background: var(--accent); color: white; border-color: var(--accent); }
  .head-actions button.follow-btn { border-color: var(--accent); color: var(--accent); }
  .head-actions button.danger { color: #dc2626; border-color: #dc2626; }
  .head-actions button.ghost:hover { background: var(--bg-hover, #f5f5f5); }

  .layout {
    display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 24px;
    padding: 0 1rem 2rem;
  }
  @media (max-width: 768px) { .layout { grid-template-columns: 1fr; } }

  .filter-bar { display: flex; gap: 4px; margin-bottom: 12px; border-bottom: 1px solid var(--border); }
  .filter-bar button {
    padding: 8px 14px; background: none; border: none; cursor: pointer;
    font-size: 13px; color: var(--text-secondary); border-bottom: 2px solid transparent;
  }
  .filter-bar button.active { color: var(--accent); border-color: var(--accent); }
  .add-content-btn {
    margin-left: auto; color: var(--accent) !important; border: 1px dashed var(--accent) !important;
    border-radius: 3px; padding: 4px 10px !important; font-size: 12px !important;
    align-self: center; margin-bottom: 4px;
  }
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; justify-content: center; align-items: center; z-index: 500;
  }
  .modal {
    background: var(--bg-white); border-radius: 6px; padding: 20px;
    width: 560px; max-width: 92vw; max-height: 85vh; overflow-y: auto;
  }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); font-weight: 400; }
  .modal h4 { font-size: 12px; font-weight: 600; text-transform: uppercase; color: var(--text-hint); margin: 12px 0 6px; letter-spacing: 0.04em; }
  .pick-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
  .pick-list li {
    display: flex; align-items: center; gap: 8px; padding: 6px 8px;
    border: 1px solid var(--border); border-radius: 3px; font-size: 13px;
  }
  .pick-title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pick-list button {
    width: 22px; height: 22px; border: none; background: var(--accent); color: white;
    border-radius: 50%; cursor: pointer; font-size: 14px;
  }
  .pick-done { color: var(--accent); font-size: 14px; }
  .modal .actions { display: flex; justify-content: flex-end; margin-top: 12px; }
  .modal .actions button {
    padding: 6px 14px; font-size: 13px; border: 1px solid var(--border);
    border-radius: 3px; background: none; cursor: pointer;
  }

  .content-list { display: flex; flex-direction: column; gap: 12px; }
  .content-wrap { position: relative; }
  .detach {
    position: absolute; top: 4px; right: 4px; z-index: 2;
    width: 20px; height: 20px; border-radius: 50%;
    background: rgba(0,0,0,0.5); color: white; border: none;
    cursor: pointer; font-size: 14px; line-height: 1;
  }
  .detach:hover { background: #dc2626; }

  .series-card {
    display: block; padding: 12px 14px; border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg-white); color: inherit; text-decoration: none;
  }
  .series-card:hover { border-color: var(--accent); }
  .series-badge {
    display: inline-block; font-size: 11px; background: var(--accent); color: white;
    padding: 1px 6px; border-radius: 2px; margin-bottom: 6px;
  }
  .series-card h3 { margin: 0 0 4px; font-size: 1rem; font-weight: 500; }
  .series-card .desc { margin: 0; font-size: 13px; color: var(--text-secondary); }

  aside .card {
    border: 1px solid var(--border); border-radius: 6px; padding: 14px;
    background: var(--bg-white);
  }
  aside h3 { margin: 0 0 10px; font-size: 13px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-hint); }
  .member-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .member-list li { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .member-list a { display: inline-flex; align-items: center; gap: 6px; flex: 1; color: inherit; text-decoration: none; min-width: 0; }
  .member-list img, .avatar-ph {
    width: 24px; height: 24px; border-radius: 50%; background: var(--bg-hover, #eee);
    display: inline-flex; align-items: center; justify-content: center; font-size: 12px;
    flex-shrink: 0; object-fit: cover;
  }
  .member-list .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .member-list .role { font-size: 11px; color: var(--text-hint); }
  .member-list .pending { color: #f59e0b; }
  .member-list .kick {
    width: 18px; height: 18px; border-radius: 50%; border: none; cursor: pointer;
    background: none; color: var(--text-hint); font-size: 14px; line-height: 1;
  }
  .member-list .kick:hover { color: #dc2626; }
  .add-member { margin-top: 10px; display: flex; flex-direction: column; gap: 6px; }
  .add-member input, .add-member select {
    width: 100%; padding: 5px 8px; font-size: 13px;
    border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white);
  }
  .add-member button {
    padding: 4px 10px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px;
    background: none; cursor: pointer;
  }
  .add-member button.primary { background: var(--accent); color: white; border-color: var(--accent); }
  .ghost.block { width: 100%; margin-top: 10px; padding: 5px; border: 1px dashed var(--border); border-radius: 3px; background: none; color: var(--text-hint); cursor: pointer; font-size: 12px; }
  .ghost.block:hover { border-color: var(--accent); color: var(--accent); }
  .meta { text-align: center; color: var(--text-hint); padding: 40px 0; }
</style>
