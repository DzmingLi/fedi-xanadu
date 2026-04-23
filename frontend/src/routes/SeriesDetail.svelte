<script lang="ts">
  import { getSeries, getSeriesHeadings, getVotesBatch, castVote, addBookmark, removeBookmark, listBookmarks, getMyVote, forkSeries, uploadSeriesCover, removeSeriesCover } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import type { SeriesDetail, SeriesArticle, SeriesArticlePrereq, SeriesHeading, VoteSummary, BookmarkWithTitle } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let detail = $state<SeriesDetail | null>(null);
  let headings = $state<SeriesHeading[]>([]);
  let loading = $state(true);
  let error = $state('');

  // Build heading tree: group level-2+ headings under their level-1 parent
  interface HeadingGroup {
    heading: SeriesHeading;           // level-1 chapter heading
    sections: SeriesHeading[];        // level-2+ section headings (with article_uri)
  }

  let headingGroups = $derived.by((): HeadingGroup[] => {
    if (!headings.length) return [];
    const groups: HeadingGroup[] = [];
    const idMap = new Map<number, SeriesHeading>();
    for (const h of headings) idMap.set(h.id, h);

    // Top-level headings become chapter groups
    const topLevel = headings.filter(h => !h.parent_heading_id);
    for (const ch of topLevel) {
      const sections = headings.filter(
        h => h.parent_heading_id === ch.id && h.article_uri
      );
      groups.push({ heading: ch, sections });
    }
    return groups;
  });

  // Votes per article
  let articleVotes = $state(new Map<string, VoteSummary>());

  // Bookmarks
  let bookmarkedUris = $state(new Set<string>());

  // Series-level vote
  let seriesVotes = $state<VoteSummary | null>(null);
  let mySeriesVote = $state(0);
  let commentThread: CommentThread | undefined = $state();
  let forking = $state(false);

  let isLoggedIn = $derived(!!getAuth());
  let isOwner = $derived(isLoggedIn && detail?.series.created_by === getAuth()?.did);


  $effect(() => {
    loadSeries();
  });

  async function loadSeries() {
    loading = true;
    error = '';
    try {
      // Fire bookmarks in parallel with series/headings (doesn't depend on them)
      const bksPromise = getAuth() ? listBookmarks().catch(() => null) : Promise.resolve(null);
      const [d, h] = await Promise.all([getSeries(id), getSeriesHeadings(id)]);
      detail = d;
      headings = h;
      document.title = `${d.series.title} — NightBoat`;

      // Collect all article URIs for vote fetching
      const allArticleUris = new Set<string>();
      for (const a of d.articles) allArticleUris.add(a.article_uri);

      // Fetch votes and settle bookmarks in parallel
      const [voteResults, bks] = await Promise.all([
        allArticleUris.size > 0
          ? getVotesBatch([...allArticleUris]).catch(() => null)
          : Promise.resolve(null),
        bksPromise,
      ]);

      const voteMap = new Map<string, VoteSummary>();
      if (voteResults) {
        for (const v of voteResults) voteMap.set(v.target_uri, v);
      } else {
        for (const uri of allArticleUris) {
          voteMap.set(uri, { target_uri: uri, score: 0, upvotes: 0, downvotes: 0 });
        }
      }
      articleVotes = voteMap;

      if (bks) bookmarkedUris = new Set(bks.map(b => b.article_uri));

      // Fetch series-level vote
      const seriesUri = `series:${id}`;
      const [sv, mv] = await Promise.all([
        getVotesBatch([seriesUri]).then(r => r?.[0] ?? null).catch(() => null),
        getAuth() ? getMyVote(seriesUri).then(r => r.value).catch(() => 0) : Promise.resolve(0),
      ]);
      seriesVotes = sv ?? { target_uri: seriesUri, score: 0, upvotes: 0, downvotes: 0 };
      mySeriesVote = mv;
    } catch (e: any) {
      error = e.message || 'Failed to load series';
    }
    loading = false;
  }

  async function voteArticle(uri: string, value: number) {
    if (!isLoggedIn) return;
    try {
      const result = await castVote(uri, value);
      articleVotes.set(uri, result);
      articleVotes = new Map(articleVotes);
    } catch { /* */ }
  }

  async function toggleBookmark(uri: string) {
    if (!isLoggedIn) return;
    try {
      if (bookmarkedUris.has(uri)) {
        await removeBookmark(uri);
        bookmarkedUris.delete(uri);
        bookmarkedUris = new Set(bookmarkedUris);
      } else {
        await addBookmark(uri);
        bookmarkedUris.add(uri);
        bookmarkedUris = new Set(bookmarkedUris);
      }
    } catch { /* */ }
  }

  async function doSeriesVote(value: number) {
    if (!isLoggedIn) return;
    const seriesUri = `series:${id}`;
    const newValue = mySeriesVote === value ? 0 : value;
    seriesVotes = await castVote(seriesUri, newValue);
    mySeriesVote = newValue;
  }

  async function doForkSeries() {
    if (!isLoggedIn || forking) return;
    forking = true;
    try {
      const forked = await forkSeries(id);
      window.location.href = `/series?id=${encodeURIComponent(forked.id)}`;
    } catch {
      forking = false;
    }
  }

  // Build a map of prereqs for visualization
  let prereqMap = $derived.by(() => {
    if (!detail) return new Map<string, string[]>();
    const m = new Map<string, string[]>();
    for (const p of detail.prereqs) {
      const arr = m.get(p.article_uri) || [];
      arr.push(p.prereq_article_uri);
      m.set(p.article_uri, arr);
    }
    return m;
  });

  // Article lookup by URI
  let articleByUri = $derived.by(() => {
    if (!detail) return new Map<string, SeriesArticle>();
    return new Map(detail.articles.map(a => [a.article_uri, a]));
  });

</script>

{#snippet articleItem(article: SeriesArticle, idx: number)}
  <div class="series-item">
    <div class="item-number">{idx + 1}</div>
    <div class="item-content">
      {#if prereqMap.has(article.article_uri)}
        <div class="item-prereqs">
          {t('series.prereqLabel')}
          {#each prereqMap.get(article.article_uri)! as pUri}
            {@const pArticle = articleByUri.get(pUri)}
            {#if pArticle}
              <a href="/article?uri={encodeURIComponent(pUri)}&series_id={encodeURIComponent(id)}" class="prereq-link">{pArticle.title}</a>
            {/if}
          {/each}
        </div>
      {/if}
      <a href="/article?uri={encodeURIComponent(article.article_uri)}&series_id={encodeURIComponent(id)}" class="item-title">
        {article.title}
      </a>
      {#if article.summary}
        <p class="item-desc">{article.summary}</p>
      {/if}
      <div class="item-actions">
        <span class="vote-score">{(articleVotes.get(article.article_uri)?.score) ?? 0}</span>
        <button class="action-btn" onclick={() => voteArticle(article.article_uri, 1)} disabled={!isLoggedIn} title={t('common.upvote')}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-6 0v4H5a2 2 0 00-2 2v7a2 2 0 002 2h14l-5-16z"/></svg>
        </button>
        <button class="action-btn" onclick={() => toggleBookmark(article.article_uri)} disabled={!isLoggedIn} title={t('article.bookmark')}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill={bookmarkedUris.has(article.article_uri) ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/></svg>
        </button>
      </div>
    </div>
  </div>
{/snippet}

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <div class="series-header">
    {#if isOwner}
      <div class="cover-strip">
        {#if detail.series.cover_url}
          <img src={detail.series.cover_url} alt="" class="cover-thumb" />
        {:else}
          <div class="cover-thumb placeholder">{t('article.noCover')}</div>
        {/if}
        <label class="cover-btn">
          <input type="file" accept="image/*" class="sr-only" onchange={async (e) => {
            const file = (e.target as HTMLInputElement).files?.[0];
            if (!file || !detail) return;
            try {
              const url = await uploadSeriesCover(detail.series.id, file);
              detail.series.cover_url = url;
            } catch (err) { alert(err instanceof Error ? err.message : String(err)); }
          }} />
          {detail.series.cover_url ? t('article.changeCover') : t('article.uploadCover')}
        </label>
        {#if detail.series.cover_url}
          <button class="cover-btn danger" onclick={async () => {
            if (!detail) return;
            try { await removeSeriesCover(detail.series.id); detail.series.cover_url = null; }
            catch (err) { alert(err instanceof Error ? err.message : String(err)); }
          }}>{t('article.removeCover')}</button>
        {/if}
      </div>
    {/if}
    <div class="series-title-row">
      <h1>{detail.series.title}</h1>
    </div>
    {#if detail.series.long_description}
      <p class="series-long-desc">{detail.series.long_description}</p>
    {:else if detail.series.summary}
      <p class="series-desc">{detail.series.summary}</p>
    {/if}
    <div class="series-meta">
      <span class="meta">{detail.articles.length} {t('series.articles')}</span>
      <span class="meta"><a href="/profile?did={encodeURIComponent(detail.series.created_by)}">{detail.series.author_display_name || '@' + (detail.series.author_handle || detail.series.created_by)}</a></span>
    </div>
    {#if detail.translations && detail.translations.length > 0}
      <div class="series-translations">
        <span class="lang-current">{detail.series.lang}</span>
        {#each detail.translations as tr (tr.id)}
          <a href="/series?id={encodeURIComponent(tr.id)}" class="lang-link">{tr.lang}</a>
        {/each}
      </div>
    {/if}
  </div>

  {#if headingGroups.length > 0}
    <!-- Hierarchical TOC: chapter groups with sections -->
    <div class="toc-chapters">
      {#each headingGroups as group (group.heading.id)}
        <div class="toc-chapter">
          <h2 class="chapter-title">{group.heading.title}</h2>
          {#if group.sections.length > 0}
            <div class="series-articles">
              {#each group.sections as sec, i (sec.id)}
                {@const article = detail.articles.find(a => a.article_uri === sec.article_uri)}
                {#if article}
                  {@render articleItem(article, i)}
                {:else}
                  <!-- article_uri exists but not in detail.articles — render as plain link -->
                  <div class="series-item">
                    <div class="item-number">{i + 1}</div>
                    <div class="item-content">
                      <a href="/article?uri={encodeURIComponent(sec.article_uri!)}&series_id={encodeURIComponent(id)}" class="item-title">
                        {sec.title}
                      </a>
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          {:else}
            <!-- chapter heading with no child sections — show articles directly -->
            <p class="toc-empty">（暂无章节）</p>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="series-articles">
      {#each detail.articles as article, i (article.article_uri)}
        {@render articleItem(article, i)}
      {/each}
    </div>
  {/if}

  <!-- Action bar -->
  <div class="action-bar">
    <div class="action-group">
      <button class="action-btn" class:active={mySeriesVote > 0} onclick={() => doSeriesVote(1)} disabled={!isLoggedIn} title={isLoggedIn ? t('article.upvote') : t('article.loginToVote')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill={mySeriesVote > 0 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14z"/><path d="M7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3"/></svg>
      </button>
      <span class="action-score">{seriesVotes?.score ?? 0}</span>
      <button class="action-btn" class:active={mySeriesVote < 0} onclick={() => doSeriesVote(-1)} disabled={!isLoggedIn} title={isLoggedIn ? t('article.downvote') : t('article.loginToVote')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill={mySeriesVote < 0 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M10 15v4a3 3 0 003 3l4-9V2H5.72a2 2 0 00-2 1.7l-1.38 9a2 2 0 002 2.3H10z"/><path d="M17 2h2.67A2.31 2.31 0 0122 4v7a2.31 2.31 0 01-2.33 2H17"/></svg>
      </button>
    </div>

    <button class="action-btn labeled-btn" onclick={doForkSeries} disabled={!isLoggedIn || forking} title={t('article.fork')}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9"/><path d="M12 12v3"/></svg>
      <span class="btn-label">{forking ? '...' : t('article.fork')}</span>
    </button>

    {#if isOwner}
      <div class="action-separator"></div>
      <a href="/series-editor?id={encodeURIComponent(id)}" class="action-btn labeled-btn" title={t('common.edit')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
        <span class="btn-label">{t('common.edit')}</span>
      </a>
    {/if}
  </div>

  <!-- Comments -->
  <CommentThread bind:this={commentThread} contentUri={`series:${id}`} />
{/if}

<style>
  .cover-strip {
    display: flex; align-items: center; gap: 10px; margin-bottom: 12px;
    padding: 8px; border: 1px dashed var(--border); border-radius: 4px;
    background: var(--bg-hover, #f6f6f1);
  }
  .cover-thumb {
    width: 80px; height: 80px; object-fit: cover; border-radius: 3px;
    background: var(--bg-white);
  }
  .cover-thumb.placeholder {
    display: flex; align-items: center; justify-content: center;
    font-size: 11px; color: var(--text-hint); text-align: center;
    border: 1px solid var(--border);
  }
  .cover-btn {
    font-size: 12px; padding: 4px 10px; border: 1px solid var(--border);
    border-radius: 3px; background: var(--bg-white); color: var(--text-secondary);
    cursor: pointer;
  }
  .cover-btn:hover { border-color: var(--accent); color: var(--accent); }
  .cover-btn.danger:hover { border-color: #dc2626; color: #dc2626; }
  .sr-only {
    position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0,0,0,0); border: 0;
  }
  .series-header {
    margin-bottom: 24px;
  }
  .series-title-row {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 8px;
  }
  .series-title-row h1 { margin: 0; }
  .series-long-desc {
    font-size: 15px;
    color: var(--text-secondary);
    margin: 0 0 16px;
    line-height: 1.6;
    white-space: pre-line;
  }
  .series-desc {
    font-size: 15px;
    color: var(--text-secondary);
    margin: 0 0 12px;
    line-height: 1.5;
  }
  .series-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .series-translations {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 13px;
  }
  .lang-current {
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
  }
  .lang-link {
    color: var(--accent);
    text-decoration: none;
    text-transform: uppercase;
  }
  .lang-link:hover {
    text-decoration: underline;
  }

  .series-articles {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .series-item {
    display: flex;
    gap: 16px;
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
  }
  .series-item:last-child {
    border-bottom: none;
  }
  .item-number {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-gray, #f5f5f5);
    border-radius: 50%;
    font-family: var(--font-serif);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .item-content {
    flex: 1;
    min-width: 0;
  }
  .item-prereqs {
    font-size: 12px;
    color: var(--text-hint);
    margin-bottom: 4px;
  }
  .prereq-link {
    color: var(--accent);
    text-decoration: none;
    margin-left: 4px;
  }
  .prereq-link:hover {
    text-decoration: underline;
  }
  .item-title {
    font-family: var(--font-serif);
    font-size: 1.1rem;
    color: var(--text-primary);
    text-decoration: none;
    line-height: 1.4;
  }
  .item-title:hover {
    color: var(--accent);
  }
  .item-desc {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 4px 0 0;
    line-height: 1.5;
  }
  .item-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .vote-score {
    font-size: 13px;
    color: var(--text-hint);
    min-width: 20px;
    text-align: center;
  }
  .action-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    padding: 2px;
    display: flex;
    transition: color 0.15s;
  }
  .action-btn:hover:not(:disabled) {
    color: var(--accent);
  }
  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Hierarchical TOC */
  .toc-chapters {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .toc-chapter {
    margin-bottom: 8px;
  }
  .chapter-title {
    font-family: var(--font-serif);
    font-size: 1.1rem;
    font-weight: 600;
    margin: 28px 0 0;
    padding-bottom: 6px;
    border-bottom: 2px solid var(--border);
    color: var(--text-secondary);
  }
  .toc-empty {
    font-size: 13px;
    color: var(--text-hint);
    margin: 8px 0;
  }

  /* Action bar */
  .action-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2rem;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .action-group {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-right: 6px;
  }
  .action-bar .action-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    color: var(--text-hint);
    transition: all 0.15s;
    text-decoration: none;
  }
  .action-bar .action-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .action-bar .action-btn.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .action-bar .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .labeled-btn {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .btn-label {
    font-size: 12px;
  }
  .action-score {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 20px;
    text-align: center;
  }
  .action-separator {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 4px;
  }
</style>
