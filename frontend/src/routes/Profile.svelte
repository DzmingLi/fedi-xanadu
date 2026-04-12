<script lang="ts">
  import { getProfile, getArticlesByDid, getQuestionsByDid, getAnswersByDid, listSeries, getAllArticleTeaches, getAllSeriesArticles, listFollows, followUser, unfollowUser, markFollowSeen, updateProfileLinks, getFollowing, getFollowers, getSettings, setSettings, blockUser as apiBlockUser, unblockUser as apiUnblockUser, createReport, listPublicBookmarks } from '../lib/api';
  import type { FollowEntry } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { isBlocked, addBlocked, removeBlocked } from '../lib/blocklist.svelte';
  import { tagName, deduplicateByTranslation, deduplicateSeriesByTranslation } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { buildSeriesArticleMaps, buildArticleRowMap } from '../lib/series';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { ProfileData, Article, Series, ContentTeachRow, ProfileLink, BookmarkWithTitle, EducationEntry } from '../lib/types';

  // All series (including sub-series) for building tree
  let allUserSeries = $state<Series[]>([]);

  let { did } = $props<{ did: string }>();

  let locale = $derived(getLocale());

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
  let editingEmail = $state(false);
  let editEmail = $state('');
  let userBlocked = $state(false);
  let reportOpen = $state(false);
  let reportReason = $state('');

  let questions = $state<Article[]>([]);
  let answers = $state<Article[]>([]);
  let publicBookmarks = $state<BookmarkWithTitle[]>([]);
  let profileTab = $state<string>('general');

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

  function buildFeed(categoryFilter: string): ProfileFeedItem[] {
    const items: ProfileFeedItem[] = [];
    const deduped = deduplicateByTranslation(articles, locale);
    for (const a of deduped) {
      if (!seriesArticleUris.has(a.at_uri) && (a.category || 'general') === categoryFilter) {
        items.push({ type: 'article', article: a, sortDate: a.created_at });
      }
    }
    for (const s of allUserSeries) {
      if ((s.category || 'general') !== categoryFilter) continue;
      const totalArticles = (seriesArticleMap.get(s.id) || []).length;
      items.push({ type: 'series', series: s, articleCount: totalArticles, sortDate: s.created_at });
    }
    items.sort((a, b) => b.sortDate.localeCompare(a.sortDate));
    return items;
  }

  // Dynamically derive all categories from user's articles and series
  let userCategories = $derived.by((): { key: string; label: string; count: number }[] => {
    const counts = new Map<string, number>();
    const deduped = deduplicateByTranslation(articles, locale);
    for (const a of deduped) {
      const cat = a.category || 'general';
      if (!seriesArticleUris.has(a.at_uri)) {
        counts.set(cat, (counts.get(cat) || 0) + 1);
      }
    }
    for (const s of allUserSeries) {
      const cat = s.category || 'general';
      counts.set(cat, (counts.get(cat) || 0) + 1);
    }
    // Build ordered list: 'general' first, then sorted by count desc
    const cats: { key: string; label: string; count: number }[] = [];
    const knownLabels: Record<string, string> = {
      general: t('category.general'),
      lecture: t('category.lecture'),
      paper: t('category.paper'),
      review: t('category.review'),
    };
    const allKeys = Array.from(counts.keys());
    // Ensure 'general' is always first
    if (!allKeys.includes('general')) allKeys.unshift('general');
    const sorted = allKeys.sort((a, b) => {
      if (a === 'general') return -1;
      if (b === 'general') return 1;
      return (counts.get(b) || 0) - (counts.get(a) || 0);
    });
    for (const key of sorted) {
      cats.push({
        key,
        label: knownLabels[key] || key,
        count: counts.get(key) || 0,
      });
    }
    return cats;
  });

  let currentFeed = $derived(buildFeed(profileTab));

  // "全部文章" tab: articles grouped by series
  interface ArticleGroup {
    series: Series | null;
    articles: Article[];
  }
  let allArticleGroups = $derived.by((): ArticleGroup[] => {
    const deduped = deduplicateByTranslation(articles, locale);
    const groups: ArticleGroup[] = [];
    const assignedUris = new Set<string>();

    for (const s of allUserSeries) {
      const uriList = seriesArticleMap.get(s.id) || [];
      const seriesArts = uriList
        .map(uri => deduped.find(a => a.at_uri === uri))
        .filter(Boolean) as Article[];
      if (seriesArts.length > 0) {
        groups.push({ series: s, articles: seriesArts });
        seriesArts.forEach(a => assignedUris.add(a.at_uri));
      }
    }

    const standalone = deduped.filter(a => !assignedUris.has(a.at_uri));
    if (standalone.length > 0) {
      groups.push({ series: null, articles: standalone });
    }

    return groups;
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

      // Check block + follow status
      userBlocked = isBlocked(did);
      if (getAuth() && !isOwnProfile) {
        try {
          const follows = await listFollows();
          isFollowing = follows.some(f => f.follows_did === did);
          // Mark as seen
          if (isFollowing) markFollowSeen(did).catch(() => {});
        } catch { /* */ }
      }

      // Load public bookmarks (for other users)
      if (!isOwnProfile) {
        try {
          publicBookmarks = await listPublicBookmarks(did);
        } catch { publicBookmarks = []; }
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

  async function toggleBlock() {
    if (userBlocked) {
      try {
        await apiUnblockUser(did);
        removeBlocked(did);
        userBlocked = false;
      } catch { /* */ }
    } else {
      if (!confirm(t('block.confirm'))) return;
      try {
        await apiBlockUser(did);
        addBlocked(did);
        userBlocked = true;
      } catch { /* */ }
    }
  }

  async function submitReport() {
    if (!reportReason.trim()) return;
    try {
      await createReport(did, 'user', reportReason.trim());
      reportOpen = false;
      reportReason = '';
      alert(t('report.success'));
    } catch {
      alert(t('report.failed'));
    }
  }

  function startEditEmail() {
    editEmail = profile?.email || '';
    editingEmail = true;
  }

  async function saveEmail() {
    try {
      const s = await getSettings();
      await setSettings({ ...s, email: editEmail.trim() || null });
      if (profile) profile.email = editEmail.trim() || null;
      editingEmail = false;
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

{#snippet seriesTree(s: Series, totalArticles?: number)}
  {@const articleUris = seriesArticleMap.get(s.id) || []}
  {@const count = totalArticles ?? articleUris.length}
  {@const isExpanded = expandedSeries.has(s.id)}

  <div class="series-tree-node">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="series-tree-card" onclick={(e) => count > 0 ? toggleExpand(e, s.id) : null}>
      <div class="series-tree-top">
        {#if count > 0}
          <span class="expand-arrow" class:expanded={isExpanded}>&#9654;</span>
        {:else}
          <span class="expand-arrow-placeholder"></span>
        {/if}
        <a href="/series?id={encodeURIComponent(s.id)}" class="series-tree-title" onclick={(e) => e.stopPropagation()}>
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
      {#each articleUris as uri}
        {@const art = articles.find(a => a.at_uri === uri)}
        {#if art}
          <PostCard
            article={art}
            articleTeaches={articleTeaches.get(art.at_uri) || []}
            variant="profile"
          />
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
      {#if profile.affiliation}
        <p class="credential-line">
          {profile.affiliation}
          {#if profile.credentials_verified}
            <span class="verified-badge" title={t('profile.verified')}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--accent)" stroke="white" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
            </span>
          {/if}
        </p>
      {/if}
      {#if profile.education.length > 0}
        <div class="education-list">
          {#each profile.education as edu}
            <span class="education-entry">
              {#if edu.current}{t('profile.enrolledIn')}{/if}{edu.degree}{#if edu.degree && edu.school}, {/if}{edu.school}{#if edu.year} ({edu.year}){/if}
            </span>
          {/each}
          {#if profile.credentials_verified && !profile.affiliation}
            <span class="verified-badge" title={t('profile.verified')}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--accent)" stroke="white" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
            </span>
          {/if}
        </div>
      {/if}
      {#if profile.email || isOwnProfile}
        <div class="profile-email">
          {#if editingEmail}
            <input
              type="email"
              bind:value={editEmail}
              placeholder="user@example.com"
              class="email-input"
            />
            <button class="email-save" onclick={saveEmail}>{t('common.save')}</button>
            <button class="email-cancel" onclick={() => { editingEmail = false; }}>{t('common.cancel')}</button>
          {:else}
            {#if profile.email}
              <a href="mailto:{profile.email}" class="email-link">{profile.email}</a>
            {/if}
            {#if isOwnProfile}
              <button class="edit-email-btn" onclick={startEditEmail}>
                {profile.email ? t('common.edit') : t('settings.email')}
              </button>
            {/if}
          {/if}
        </div>
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
    {#if getAuth() && !isOwnProfile}
      <div class="profile-actions-secondary">
        <button class="action-btn" class:active={userBlocked} onclick={toggleBlock}>
          {userBlocked ? t('block.unblock') : t('block.block')}
        </button>
        <button class="action-btn" onclick={() => { reportOpen = true; }}>
          {t('report.report')}
        </button>
      </div>
    {/if}
    {#if isOwnProfile}
      <a href="/settings" class="settings-link">{t('profile.settings')}</a>
    {/if}
  </div>

  <!-- Report modal -->
  {#if reportOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="links-overlay" onclick={() => { reportOpen = false; }}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="links-modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('report.title')}</h3>
        <p class="report-target">
          {t('report.kindUser')}: {profile?.display_name || profile?.handle || did}
        </p>
        <textarea
          bind:value={reportReason}
          placeholder={t('report.reasonPlaceholder')}
          class="report-textarea"
          rows="4"
        ></textarea>
        <div class="link-actions">
          <button class="link-cancel" onclick={() => { reportOpen = false; }}>{t('common.cancel')}</button>
          <button class="link-save" onclick={submitReport} disabled={!reportReason.trim()}>{t('report.submit')}</button>
        </div>
      </div>
    </div>
  {/if}

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
          <a href="/profile?did={encodeURIComponent(u.did)}" class="follow-item">
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
    {#each userCategories as cat (cat.key)}
      {#if cat.count > 0 || cat.key === 'general' || isOwnProfile}
        <button class="tab-btn" class:active={profileTab === cat.key} onclick={() => { profileTab = cat.key; }}>
          {cat.label}
          {#if cat.count > 0}<span class="tab-count">{cat.count}</span>{/if}
        </button>
      {/if}
    {/each}
    <button class="tab-btn" class:active={profileTab === 'qa'} onclick={() => { profileTab = 'qa'; }}>
      {t('profile.questions')}
      {#if questions.length + answers.length > 0}
        <span class="tab-count">{questions.length + answers.length}</span>
      {/if}
    </button>
    {#if !isOwnProfile && publicBookmarks.length > 0}
      <button class="tab-btn" class:active={profileTab === 'bookmarks'} onclick={() => { profileTab = 'bookmarks'; }}>
        {t('profile.publicBookmarks')}
        <span class="tab-count">{publicBookmarks.length}</span>
      </button>
    {/if}
    {#if articles.length > 0}
      <button class="tab-btn" class:active={profileTab === 'all'} onclick={() => { profileTab = 'all'; }}>
        {t('profile.tabAllArticles')}
        <span class="tab-count">{articles.length}</span>
      </button>
    {/if}
  </div>

  {#if profileTab !== 'qa' && profileTab !== 'bookmarks' && profileTab !== 'all'}
    {#each currentFeed as item}
      {#if item.type === 'article' && item.article}
        <PostCard
          article={item.article}
          articleTeaches={articleTeaches.get(item.article.at_uri) || []}
          variant="profile"
        />
      {:else if item.type === 'series' && item.series}
        {@render seriesTree(item.series, item.articleCount)}
      {/if}
    {/each}

    {#if currentFeed.length === 0}
      <p class="empty-text">{t('profile.noWorks')}</p>
    {/if}

    {#if isOwnProfile}
      <div class="create-actions">
        <a href="/new" class="create-link">{t('profile.writeArticle')}</a>
        <a href="/new-series" class="create-link">{t('profile.createSeries')}</a>
      </div>
    {/if}
  {:else if profileTab === 'qa'}
    {#if questions.length > 0}
      <h3 class="section-title">{t('qa.myQuestions')}</h3>
      {#each questions as q}
        <a href="/question?uri={encodeURIComponent(q.at_uri)}" class="qa-card question">
          <span class="qa-badge question-badge">{t('qa.questionBadge')}</span>
          <span class="qa-title">{q.title}</span>
          <span class="qa-stat">{t('qa.answerCount', q.answer_count)}</span>
        </a>
      {/each}
    {/if}

    {#if answers.length > 0}
      <h3 class="section-title">{t('qa.myAnswers')}</h3>
      {#each answers as a}
        <a href="/question?uri={encodeURIComponent(a.question_uri || '')}" class="qa-card answer">
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
        <a href="/new-question" class="create-link">{t('qa.askQuestion')}</a>
      </div>
    {/if}
  {:else if profileTab === 'bookmarks'}
    {#each publicBookmarks as bm}
      <a href="/article?uri={encodeURIComponent(bm.article_uri)}" class="bookmark-card">
        <div class="bookmark-info">
          <span class="bookmark-title">{bm.title}</span>
          {#if bm.folder_path && bm.folder_path !== '/'}
            <span class="bookmark-folder">{bm.folder_path}</span>
          {/if}
        </div>
        <span class="bookmark-date">{bm.created_at.split(' ')[0]}</span>
      </a>
    {/each}
    {#if publicBookmarks.length === 0}
      <p class="empty-text">{t('profile.noWorks')}</p>
    {/if}
  {:else if profileTab === 'all'}
    {#each allArticleGroups() as group}
      {#if group.series}
        <div class="all-series-group">
          <a href="/series?id={group.series.id}" class="all-series-title">
            {group.series.title}
            <span class="all-series-count">{group.articles.length} 篇</span>
          </a>
          <div class="all-series-articles">
            {#each group.articles as art}
              <a href="/article?uri={encodeURIComponent(art.at_uri)}" class="all-article-row">
                <span class="all-article-title">{art.title || '（无标题）'}</span>
                {#if art.lang}<span class="all-article-lang">{art.lang}</span>{/if}
              </a>
            {/each}
          </div>
        </div>
      {:else}
        {#each group.articles as art}
          <PostCard article={art} articleTeaches={articleTeaches.get(art.at_uri) || []} variant="profile" />
        {/each}
      {/if}
    {/each}
    {#if allArticleGroups().length === 0}
      <p class="empty-text">{t('profile.noWorks')}</p>
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
  .credential-line {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }
  .verified-badge {
    display: inline-flex;
    align-items: center;
  }
  .education-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    margin-top: 2px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .education-entry {
    display: inline;
  }
  .profile-email {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    font-size: 13px;
  }
  .email-link {
    color: var(--text-secondary);
    text-decoration: none;
  }
  .email-link:hover { color: var(--accent); }
  .edit-email-btn {
    font-size: 12px;
    color: var(--text-hint);
    background: none;
    border: 1px dashed var(--border);
    border-radius: 3px;
    padding: 2px 8px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .edit-email-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .email-input {
    padding: 3px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-sans);
    width: 200px;
  }
  .email-save {
    font-size: 12px;
    padding: 3px 10px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .email-cancel {
    font-size: 12px;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .settings-link {
    font-size: 13px;
    color: var(--text-secondary);
    text-decoration: none;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    transition: all 0.15s;
    align-self: center;
    flex-shrink: 0;
  }
  .settings-link:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .profile-actions-secondary {
    display: flex;
    gap: 6px;
    align-self: center;
    flex-shrink: 0;
  }
  .action-btn {
    font-size: 12px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-hint);
    cursor: pointer;
    transition: all 0.15s;
  }
  .action-btn:hover {
    border-color: var(--text-secondary);
    color: var(--text-secondary);
  }
  .action-btn.active {
    border-color: #dc2626;
    color: #dc2626;
  }
  .report-target {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 10px;
  }
  .report-textarea {
    width: 100%;
    padding: 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
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

  /* Bookmark cards */
  .bookmark-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 6px;
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s;
  }
  .bookmark-card:hover {
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .bookmark-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }
  .bookmark-title {
    font-size: 14px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bookmark-folder {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-dim);
    padding: 1px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .bookmark-date {
    font-size: 12px;
    color: var(--text-hint);
    flex-shrink: 0;
    margin-left: 12px;
  }

  /* ── 全部文章 tab ── */
  .all-series-group {
    margin-bottom: 20px;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .all-series-title {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bg-dim, var(--bg));
    font-family: var(--font-serif);
    font-size: 1rem;
    color: var(--text-primary);
    text-decoration: none;
    border-bottom: 1px solid var(--border);
  }
  .all-series-title:hover { color: var(--accent); }
  .all-series-count {
    font-size: 12px;
    color: var(--text-hint);
    font-family: var(--font-sans);
    margin-left: auto;
  }
  .all-series-articles {
    display: flex;
    flex-direction: column;
  }
  .all-article-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--border);
    text-decoration: none;
    color: var(--text-primary);
    font-size: 0.9rem;
  }
  .all-article-row:last-child { border-bottom: none; }
  .all-article-row:hover { background: var(--bg-hover, rgba(0,0,0,.03)); color: var(--accent); }
  .all-article-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .all-article-lang {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-dim);
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
  }
</style>
