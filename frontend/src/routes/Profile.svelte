<script lang="ts">
  import { getProfile, getArticlesByDid, getQuestionsByDid, getAnswersByDid, listSeries, getAllArticleTeaches, getAllSeriesArticles, listFollows, followUser, unfollowUser, markFollowSeen, updateProfileLinks, getFollowing, getFollowers } from '../lib/api';
  import type { FollowEntry } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { tagName, deduplicateByTranslation, deduplicateSeriesByTranslation } from '../lib/display';
  import { t, onLocaleChange, getLocale } from '../lib/i18n';
  import { buildSeriesArticleMaps, buildArticleRowMap } from '../lib/series';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { ProfileData, Article, Series, ContentTeachRow, ProfileLink } from '../lib/types';

  // All series (including sub-series) for building tree
  let allUserSeries = $state<Series[]>([]);

  let { did } = $props<{ did: string }>();

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let profile = $state<ProfileData | null>(null);
  let articles = $state<Article[]>([]);

  let articleTeaches = $state(new Map<string, ContentTeachRow[]>());
  let seriesArticleUris = $state(new Set<string>());
  let seriesArticleMap = $state(new Map<string, string[]>());
  let loading = $state(true);
  let isFollowing = $state(false);
  let followLoading = $state(false);
  let editingLinks = $state(false);
  let editLinks = $state<ProfileLink[]>([]);
  let newLinkLabel = $state('');
  let newLinkUrl = $state('');

  let questions = $state<Article[]>([]);
  let answers = $state<Article[]>([]);
  let profileTab = $state<'works' | 'qa'>('works');

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

  // Build parent→children map and recursive article count
  let childSeriesMap = $derived.by(() => {
    const map = new Map<string, Series[]>();
    for (const s of allUserSeries) {
      if (s.parent_id) {
        const arr = map.get(s.parent_id) || [];
        arr.push(s);
        map.set(s.parent_id, arr);
      }
    }
    // Sort children by order_index
    for (const [, children] of map) {
      children.sort((a, b) => a.order_index - b.order_index);
    }
    return map;
  });

  function countDescendantArticles(seriesId: string): number {
    const direct = (seriesArticleMap.get(seriesId) || []).length;
    const children = childSeriesMap.get(seriesId) || [];
    return direct + children.reduce((sum, c) => sum + countDescendantArticles(c.id), 0);
  }

  // Track which series are expanded
  let expandedSeries = $state(new Set<string>());
  function toggleExpand(e: MouseEvent, seriesId: string) {
    e.preventDefault();
    e.stopPropagation();
    const next = new Set(expandedSeries);
    if (next.has(seriesId)) next.delete(seriesId);
    else next.add(seriesId);
    expandedSeries = next;
  }

  let profileFeed = $derived.by(() => {
    const items: ProfileFeedItem[] = [];
    // Standalone articles (not in any series), deduplicated by translation
    const deduped = deduplicateByTranslation(articles, locale);
    for (const a of deduped) {
      if (!seriesArticleUris.has(a.at_uri)) {
        items.push({ type: 'article', article: a, sortDate: a.created_at });
      }
    }
    // Only top-level series (no parent)
    for (const s of allUserSeries) {
      if (s.parent_id) continue;
      const totalArticles = countDescendantArticles(s.id);
      items.push({ type: 'series', series: s, articleCount: totalArticles, sortDate: s.created_at });
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
      const [prof, arts, qs, ans, allSeries, tags, seriesArts] = await Promise.all([
        getProfile(did),
        getArticlesByDid(did),
        getQuestionsByDid(did),
        getAnswersByDid(did),
        listSeries(),
        getAllArticleTeaches(),
        getAllSeriesArticles(),
      ]);
      profile = prof;
      articles = arts;
      questions = qs;
      answers = ans;
      allUserSeries = deduplicateSeriesByTranslation(allSeries.filter(s => s.created_by === did), getLocale());

      const saMaps = buildSeriesArticleMaps(seriesArts);
      seriesArticleUris = saMaps.seriesArticleUris;
      seriesArticleMap = saMaps.seriesArticleMap;

      articleTeaches = buildArticleRowMap(tags);

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

{#snippet seriesTree(s: Series, depth: number, totalArticles?: number)}
  {@const children = childSeriesMap.get(s.id) || []}
  {@const directArticleUris = seriesArticleMap.get(s.id) || []}
  {@const hasChildren = children.length > 0 || directArticleUris.length > 0}
  {@const isExpanded = expandedSeries.has(s.id)}
  {@const count = totalArticles ?? countDescendantArticles(s.id)}

  <div class="series-tree-node" style="margin-left: {depth * 20}px">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="series-tree-card" onclick={(e) => hasChildren ? toggleExpand(e, s.id) : null}>
      <div class="series-tree-top">
        {#if hasChildren}
          <span class="expand-arrow" class:expanded={isExpanded}>&#9654;</span>
        {:else}
          <span class="expand-arrow-placeholder"></span>
        {/if}
        <a href="#/series?id={encodeURIComponent(s.id)}" class="series-tree-title" onclick={(e) => e.stopPropagation()}>
          {s.title}
        </a>
        <span class="series-badge">{t('profile.seriesBadge')}</span>
      </div>
      {#if s.description}
        <p class="series-tree-desc">{s.description}</p>
      {/if}
      <div class="series-tree-bottom">
        <span class="post-meta">{s.created_at.split(' ')[0]}</span>
        <span class="card-stats">
          <span class="stat">{count} {t('profile.lectureCount')}</span>
        </span>
      </div>
    </div>

    {#if isExpanded}
      {#each children as child}
        {@render seriesTree(child, depth + 1)}
      {/each}
      {#each directArticleUris as uri}
        {@const art = articles.find(a => a.at_uri === uri)}
        {#if art}
          <div style="margin-left: {(depth + 1) * 20}px">
            <PostCard
              article={art}
              articleTeaches={articleTeaches.get(art.at_uri) || []}
              variant="profile"
            />
          </div>
        {/if}
      {/each}
    {/if}
  </div>
{/snippet}

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
        <span>{profile.article_count} {t('profile.articles')}</span>
        <span>{profile.series_count} {t('profile.seriesCount')}</span>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'following' ? null : 'following'; }}>
          <strong>{following.length}</strong> {t('profile.following')}
        </button>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'followers' ? null : 'followers'; }}>
          <strong>{followers.length}</strong> {t('profile.followers')}
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
        {isFollowing ? t('profile.unfollow') : t('profile.follow')}
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
          {t('profile.editLinks')}
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
        <h3>{t('profile.editLinksTitle')}</h3>
        {#each editLinks as link, i}
          <div class="link-row">
            <span class="link-label">{link.label}</span>
            <span class="link-url">{link.url}</span>
            <button class="link-remove" onclick={() => removeLink(i)}>&times;</button>
          </div>
        {/each}
        <div class="link-add-row">
          <input bind:value={newLinkLabel} placeholder={t('profile.linkLabel')} />
          <input bind:value={newLinkUrl} placeholder="https://..." />
          <button class="link-add-btn" onclick={addLink} disabled={!newLinkLabel.trim() || !newLinkUrl.trim()}>+</button>
        </div>
        <div class="link-actions">
          <button class="link-cancel" onclick={() => { editingLinks = false; }}>{t('common.cancel')}</button>
          <button class="link-save" onclick={saveLinks}>{t('common.save')}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showFollowTab}
    {@const list = showFollowTab === 'following' ? following : followers}
    <div class="follow-list">
      <h3 class="section-title">{showFollowTab === 'following' ? t('profile.following') : t('profile.followers')}</h3>
      {#if list.length === 0}
        <p class="empty-text">{t('profile.none')}</p>
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

  <div class="profile-tabs">
    <button class="tab-btn" class:active={profileTab === 'works'} onclick={() => { profileTab = 'works'; }}>{t('profile.works')}</button>
    <button class="tab-btn" class:active={profileTab === 'qa'} onclick={() => { profileTab = 'qa'; }}>
      {t('profile.questions')}
      {#if questions.length + answers.length > 0}
        <span class="tab-count">{questions.length + answers.length}</span>
      {/if}
    </button>
  </div>

  {#if profileTab === 'works'}
    {#each profileFeed as item}
      {#if item.type === 'article' && item.article}
        <PostCard
          article={item.article}
          articleTeaches={articleTeaches.get(item.article.at_uri) || []}
          variant="profile"
        />
      {:else if item.type === 'series' && item.series}
        {@render seriesTree(item.series, 0, item.articleCount)}
      {/if}
    {/each}

    {#if profileFeed.length === 0}
      <p class="empty-text">{t('profile.noWorks')}</p>
    {/if}

    {#if isOwnProfile}
      <div class="create-actions">
        <a href="#/new" class="create-link">{t('profile.writeArticle')}</a>
        <a href="#/new-series" class="create-link">{t('profile.createSeries')}</a>
      </div>
    {/if}
  {:else}
    {#if questions.length > 0}
      <h3 class="section-title">{t('qa.myQuestions')}</h3>
      {#each questions as q}
        <a href="#/question?uri={encodeURIComponent(q.at_uri)}" class="qa-card question">
          <span class="qa-badge question-badge">{t('qa.questionBadge')}</span>
          <span class="qa-title">{q.title}</span>
          <span class="qa-stat">{t('qa.answerCount', q.answer_count)}</span>
        </a>
      {/each}
    {/if}

    {#if answers.length > 0}
      <h3 class="section-title">{t('qa.myAnswers')}</h3>
      {#each answers as a}
        <a href="#/question?uri={encodeURIComponent(a.question_uri || '')}" class="qa-card answer">
          <span class="qa-badge answer-badge">{t('qa.answerBadge')}</span>
          <span class="qa-title">{a.title}</span>
          <span class="qa-stat">&#9650; {a.vote_score}</span>
        </a>
      {/each}
    {/if}

    {#if questions.length === 0 && answers.length === 0}
      <p class="empty-text">{t('qa.noQuestions')}</p>
    {/if}

    {#if isOwnProfile}
      <div class="create-actions">
        <a href="#/new-question" class="create-link">{t('qa.askQuestion')}</a>
      </div>
    {/if}
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

  /* Card styles are now in PostCard.svelte */
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

  .series-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }

  /* Series tree */
  .series-tree-node {
    margin-bottom: 8px;
  }
  .series-tree-card {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 4px;
    padding: 12px 16px;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .series-tree-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
  }
  .series-tree-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .expand-arrow {
    font-size: 10px;
    color: var(--text-hint);
    transition: transform 0.15s;
    flex-shrink: 0;
    width: 14px;
    text-align: center;
  }
  .expand-arrow.expanded {
    transform: rotate(90deg);
  }
  .expand-arrow-placeholder {
    width: 14px;
    flex-shrink: 0;
  }
  .series-tree-title {
    font-family: var(--font-serif);
    font-size: 1.1rem;
    color: var(--text-primary);
    text-decoration: none;
    flex: 1;
    min-width: 0;
  }
  .series-tree-title:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .series-tree-desc {
    margin: 6px 0 0 22px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .series-tree-bottom {
    margin-top: 8px;
    margin-left: 22px;
    display: flex;
    align-items: center;
  }

  /* Profile tabs */
  .profile-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 16px;
  }
  .tab-btn {
    padding: 8px 20px;
    font-size: 14px;
    font-family: var(--font-sans);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tab-btn:hover { color: var(--text-primary); }
  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab-count {
    font-size: 11px;
    background: var(--border);
    color: var(--text-secondary);
    padding: 1px 6px;
    border-radius: 8px;
  }

  /* Q&A cards in profile */
  .qa-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 6px;
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s;
  }
  .qa-card:hover {
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .qa-card.question {
    border-left: 3px solid #d97706;
  }
  .qa-card.answer {
    border-left: 3px solid var(--accent);
  }
  .qa-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .question-badge {
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
  }
  .answer-badge {
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
  }
  .qa-title {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .qa-stat {
    font-size: 12px;
    color: var(--text-hint);
    flex-shrink: 0;
  }
</style>
