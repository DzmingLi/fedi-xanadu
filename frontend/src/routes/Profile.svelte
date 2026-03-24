<script lang="ts">
  import { getProfile, getArticlesByDid, listSeries, getAllArticleTags, getAllSeriesArticles, listFollows, followUser, unfollowUser, markFollowSeen, updateProfileLinks, getFollowing, getFollowers } from '../lib/api';
  import type { FollowEntry } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t, onLocaleChange, getLocale } from '../lib/i18n';
  import type { ProfileData, Article, Series, ArticleTagRow, ProfileLink } from '../lib/types';

  let { did } = $props<{ did: string }>();

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let profile = $state<ProfileData | null>(null);
  let articles = $state<Article[]>([]);
  let userSeries = $state<Series[]>([]);
  let articleTags = $state(new Map<string, ArticleTagRow[]>());
  let seriesArticleUris = $state(new Set<string>());
  let seriesArticleMap = $state(new Map<string, string[]>());
  let loading = $state(true);
  let isFollowing = $state(false);
  let followLoading = $state(false);
  let editingLinks = $state(false);
  let editLinks = $state<ProfileLink[]>([]);
  let newLinkLabel = $state('');
  let newLinkUrl = $state('');

  let isOwnProfile = $derived(getAuth()?.did === did);
  let following = $state<FollowEntry[]>([]);
  let followers = $state<FollowEntry[]>([]);
  let showFollowTab = $state<'following' | 'followers' | null>(null);

  interface ProfileFeedItem {
    type: 'article' | 'series';
    article?: Article;
    series?: Series;
    articleCount?: number;
    sortDate: string;
  }

  let profileFeed = $derived.by(() => {
    const items: ProfileFeedItem[] = [];
    // Standalone articles (not in any series)
    for (const a of articles) {
      if (!seriesArticleUris.has(a.at_uri)) {
        items.push({ type: 'article', article: a, sortDate: a.created_at });
      }
    }
    // Series cards
    for (const s of userSeries) {
      const memberUris = seriesArticleMap.get(s.id) || [];
      items.push({ type: 'series', series: s, articleCount: memberUris.length, sortDate: s.created_at });
    }
    items.sort((a, b) => b.sortDate.localeCompare(a.sortDate));
    return items;
  });

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      const [prof, arts, allSeries, tags, seriesArts] = await Promise.all([
        getProfile(did),
        getArticlesByDid(did),
        listSeries(),
        getAllArticleTags(),
        getAllSeriesArticles(),
      ]);
      profile = prof;
      articles = arts;
      userSeries = allSeries.filter(s => s.created_by === did);

      const uriSet = new Set<string>();
      const saMap = new Map<string, string[]>();
      for (const sa of seriesArts) {
        uriSet.add(sa.article_uri);
        const arr = saMap.get(sa.series_id) || [];
        arr.push(sa.article_uri);
        saMap.set(sa.series_id, arr);
      }
      seriesArticleUris = uriSet;
      seriesArticleMap = saMap;

      const tagMap = new Map<string, ArticleTagRow[]>();
      for (const t of tags) {
        const arr = tagMap.get(t.article_uri) || [];
        arr.push(t);
        tagMap.set(t.article_uri, arr);
      }
      articleTags = tagMap;

      // Load following/followers
      const [fg, fr] = await Promise.all([getFollowing(did), getFollowers(did)]);
      following = fg;
      followers = fr;

      // Check follow status
      if (getAuth() && !isOwnProfile) {
        try {
          const follows = await listFollows();
          isFollowing = follows.some(f => f.follows_did === did);
          // Mark as seen
          if (isFollowing) markFollowSeen(did).catch(() => {});
        } catch { /* */ }
      }
    } catch { /* */ }
    loading = false;
  }

  async function toggleFollow() {
    followLoading = true;
    try {
      if (isFollowing) {
        await unfollowUser(did);
        isFollowing = false;
      } else {
        await followUser(did);
        isFollowing = true;
      }
    } catch { /* */ }
    followLoading = false;
  }

  function startEditLinks() {
    editLinks = [...(profile?.links || [])];
    editingLinks = true;
  }

  function addLink() {
    if (!newLinkLabel.trim() || !newLinkUrl.trim()) return;
    editLinks = [...editLinks, { label: newLinkLabel.trim(), url: newLinkUrl.trim() }];
    newLinkLabel = '';
    newLinkUrl = '';
  }

  function removeLink(idx: number) {
    editLinks = editLinks.filter((_, i) => i !== idx);
  }

  async function saveLinks() {
    try {
      await updateProfileLinks(editLinks);
      if (profile) profile.links = editLinks;
      editingLinks = false;
    } catch { /* */ }
  }

  function linkIcon(url: string): string {
    if (url.includes('github.com') || url.includes('gitlab.com') || url.includes('codeberg.org')) return 'code';
    if (url.includes('bsky.app') || url.includes('bsky.social')) return 'bluesky';
    return 'link';
  }

  function shortDid(d: string) {
    return d.replace('did:plc:', '').slice(0, 12);
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if profile}
  <div class="profile-header">
    <div class="avatar-wrap">
      {#if profile.avatar_url}
        <img src={profile.avatar_url} alt="avatar" class="avatar" />
      {:else}
        <div class="avatar placeholder">{(profile.handle || profile.did).charAt(0).toUpperCase()}</div>
      {/if}
    </div>
    <div class="profile-info">
      <h1 class="display-name">{profile.display_name || profile.handle || shortDid(profile.did)}</h1>
      {#if profile.handle}
        <p class="handle">@{profile.handle}</p>
      {/if}
      <div class="profile-stats">
        <span>{profile.article_count} {locale === 'zh' ? '篇文章' : 'articles'}</span>
        <span>{profile.series_count} {locale === 'zh' ? '个系列' : 'series'}</span>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'following' ? null : 'following'; }}>
          <strong>{following.length}</strong> {locale === 'zh' ? '关注' : 'following'}
        </button>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'followers' ? null : 'followers'; }}>
          <strong>{followers.length}</strong> {locale === 'zh' ? '粉丝' : 'followers'}
        </button>
      </div>
    </div>
    {#if getAuth() && !isOwnProfile}
      <button
        class="follow-btn"
        class:following={isFollowing}
        onclick={toggleFollow}
        disabled={followLoading}
      >
        {isFollowing ? (locale === 'zh' ? '已关注' : 'Following') : (locale === 'zh' ? '关注' : 'Follow')}
      </button>
    {/if}
  </div>

  <!-- Profile links -->
  {#if profile.links.length > 0 || isOwnProfile}
    <div class="profile-links">
      {#each profile.links as link}
        <a href={link.url} target="_blank" rel="noopener" class="profile-link">
          {#if linkIcon(link.url) === 'code'}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
          {:else if linkIcon(link.url) === 'bluesky'}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2z"/></svg>
          {:else}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/></svg>
          {/if}
          {link.label}
        </a>
      {/each}
      {#if isOwnProfile}
        <button class="edit-links-btn" onclick={startEditLinks}>
          {locale === 'zh' ? '编辑链接' : 'Edit links'}
        </button>
      {/if}
    </div>
  {/if}

  <!-- Edit links modal -->
  {#if editingLinks}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="links-overlay" onclick={() => { editingLinks = false; }}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="links-modal" onclick={(e) => e.stopPropagation()}>
        <h3>{locale === 'zh' ? '编辑个人链接' : 'Edit Profile Links'}</h3>
        {#each editLinks as link, i}
          <div class="link-row">
            <span class="link-label">{link.label}</span>
            <span class="link-url">{link.url}</span>
            <button class="link-remove" onclick={() => removeLink(i)}>&times;</button>
          </div>
        {/each}
        <div class="link-add-row">
          <input bind:value={newLinkLabel} placeholder={locale === 'zh' ? '标签 (如: GitHub)' : 'Label (e.g. GitHub)'} />
          <input bind:value={newLinkUrl} placeholder="https://..." />
          <button class="link-add-btn" onclick={addLink} disabled={!newLinkLabel.trim() || !newLinkUrl.trim()}>+</button>
        </div>
        <div class="link-actions">
          <button class="link-cancel" onclick={() => { editingLinks = false; }}>{locale === 'zh' ? '取消' : 'Cancel'}</button>
          <button class="link-save" onclick={saveLinks}>{locale === 'zh' ? '保存' : 'Save'}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showFollowTab}
    {@const list = showFollowTab === 'following' ? following : followers}
    <div class="follow-list">
      <h3 class="section-title">{showFollowTab === 'following' ? (locale === 'zh' ? '关注' : 'Following') : (locale === 'zh' ? '粉丝' : 'Followers')}</h3>
      {#if list.length === 0}
        <p class="empty-text">{locale === 'zh' ? '暂无' : 'None'}</p>
      {:else}
        {#each list as u}
          <a href="#/profile?did={encodeURIComponent(u.did)}" class="follow-item">
            {#if u.avatar_url}
              <img src={u.avatar_url} alt="" class="follow-avatar" />
            {:else}
              <div class="follow-avatar placeholder">{(u.handle || u.did).charAt(0).toUpperCase()}</div>
            {/if}
            <div class="follow-info">
              <span class="follow-name">{u.display_name || u.handle || u.did.slice(0, 20)}</span>
              {#if u.handle}
                <span class="follow-handle">@{u.handle}</span>
              {/if}
            </div>
          </a>
        {/each}
      {/if}
    </div>
  {/if}

  <h2 class="section-title">{locale === 'zh' ? '作品' : 'Works'}</h2>

  {#each profileFeed as item}
    {#if item.type === 'article' && item.article}
      {@const a = item.article}
      <a href="#/article?uri={encodeURIComponent(a.at_uri)}" class="post-card">
        <div class="card-top">
          <span class="post-title">{a.title}</span>
          <div class="card-tags">
            {#if articleTags.has(a.at_uri)}
              {#each articleTags.get(a.at_uri)! as t}
                <a href="#/tag?id={encodeURIComponent(t.tag_id)}" class="tag" onclick={(e) => e.stopPropagation()}>{t.tag_name}</a>
              {/each}
            {/if}
          </div>
        </div>
        {#if a.description}
          <p class="post-desc">{a.description}</p>
        {/if}
        <div class="card-bottom">
          <span class="post-meta">{a.created_at.split(' ')[0]}</span>
        </div>
      </a>
    {:else if item.type === 'series' && item.series}
      {@const s = item.series}
      <a href="#/series?id={encodeURIComponent(s.id)}" class="post-card series-card-inline">
        <div class="card-top">
          <span class="post-title">{s.title}</span>
          <span class="series-badge">{locale === 'zh' ? '系列' : 'Series'}</span>
        </div>
        {#if s.description}
          <p class="post-desc">{s.description}</p>
        {/if}
        <div class="card-bottom">
          <span class="post-meta">{s.created_at.split(' ')[0]}</span>
          <span class="post-meta">{item.articleCount} {locale === 'zh' ? '篇' : 'articles'}</span>
        </div>
      </a>
    {/if}
  {/each}

  {#if profileFeed.length === 0}
    <p class="empty-text">{locale === 'zh' ? '暂无作品' : 'No works yet'}</p>
  {/if}

  {#if isOwnProfile}
    <div class="create-actions">
      <a href="#/new" class="create-link">{locale === 'zh' ? '写文章' : 'Write article'}</a>
      <a href="#/new-series" class="create-link">{locale === 'zh' ? '创建系列' : 'Create series'}</a>
    </div>
  {/if}
{/if}

<style>
  .profile-header {
    display: flex;
    gap: 20px;
    align-items: center;
    margin-bottom: 24px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border);
  }
  .avatar {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    object-fit: cover;
  }
  .avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-size: 28px;
    font-family: var(--font-serif);
  }
  .profile-info {
    flex: 1;
  }
  .follow-btn {
    padding: 6px 20px;
    font-size: 14px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--accent);
    background: none;
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
    align-self: center;
  }
  .follow-btn:hover {
    background: var(--accent);
    color: white;
  }
  .follow-btn.following {
    background: var(--accent);
    color: white;
  }
  .follow-btn.following:hover {
    background: #dc2626;
    border-color: #dc2626;
  }
  .follow-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .display-name {
    font-family: var(--font-serif);
    margin: 0;
    font-size: 1.5rem;
  }
  .handle {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }
  .profile-stats {
    display: flex;
    gap: 16px;
    margin-top: 8px;
    font-size: 13px;
    color: var(--text-hint);
  }

  .section-title {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.1rem;
    margin: 0 0 12px;
    color: var(--text-secondary);
  }

  .post-card {
    display: block;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px 20px;
    margin-bottom: 12px;
    transition: border-color 0.15s;
    text-decoration: none;
    color: inherit;
  }
  .post-card:hover {
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .card-top {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .post-title {
    font-family: var(--font-serif);
    font-size: 1.1rem;
    color: var(--text-primary);
    flex: 1;
    min-width: 0;
  }
  .post-card:hover .post-title { color: var(--accent); }
  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    flex-shrink: 0;
  }
  .post-desc {
    margin: 6px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .card-bottom { margin-top: 8px; }
  .post-meta { font-size: 13px; color: var(--text-hint); }

  .series-card-inline {
    border-left: 3px solid var(--accent);
  }
  .series-badge {
    font-size: 11px;
    background: rgba(95,155,101,0.12);
    color: var(--accent);
    padding: 1px 8px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .empty-text { color: var(--text-hint); font-size: 14px; }
  .create-actions {
    display: flex;
    gap: 16px;
    margin-top: 12px;
  }
  .create-link {
    font-size: 14px;
    color: var(--accent);
  }

  /* Profile links */
  .profile-links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-bottom: 20px;
  }
  .profile-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: var(--text-secondary);
    text-decoration: none;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    transition: all 0.15s;
  }
  .profile-link:hover {
    color: var(--accent);
    border-color: var(--accent);
    text-decoration: none;
  }
  .edit-links-btn {
    font-size: 12px;
    color: var(--text-hint);
    background: none;
    border: 1px dashed var(--border);
    border-radius: 3px;
    padding: 3px 10px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .edit-links-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  /* Edit links modal */
  .links-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    z-index: 400;
    display: flex;
    justify-content: center;
    padding-top: 10vh;
  }
  .links-modal {
    width: 480px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.15);
    align-self: flex-start;
  }
  .links-modal h3 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .link-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }
  .link-label {
    font-weight: 500;
    font-size: 14px;
    min-width: 80px;
  }
  .link-url {
    font-size: 13px;
    color: var(--text-secondary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 18px;
    color: var(--text-hint);
    padding: 0 4px;
  }
  .link-remove:hover { color: #dc2626; }
  .link-add-row {
    display: flex;
    gap: 6px;
    margin-top: 10px;
  }
  .link-add-row input {
    flex: 1;
    padding: 5px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-sans);
  }
  .link-add-btn {
    padding: 5px 12px;
    font-size: 16px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .link-add-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .link-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .link-cancel {
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .link-save {
    padding: 5px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  /* Follow stats buttons */
  .stat-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-hint);
    padding: 0;
    transition: color 0.15s;
  }
  .stat-btn:hover { color: var(--accent); }
  .stat-btn strong { color: var(--text-secondary); }

  /* Follow list */
  .follow-list {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .follow-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    text-decoration: none;
    color: inherit;
    transition: opacity 0.15s;
  }
  .follow-item:hover { opacity: 0.8; text-decoration: none; }
  .follow-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .follow-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-family: var(--font-serif);
  }
  .follow-info {
    display: flex;
    flex-direction: column;
  }
  .follow-name {
    font-size: 14px;
    color: var(--text-primary);
  }
  .follow-handle {
    font-size: 12px;
    color: var(--text-hint);
  }
</style>
