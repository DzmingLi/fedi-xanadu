<script lang="ts">
  import { getSeries, getArticleVotes, castVote, addBookmark, removeBookmark, listBookmarks } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import type { SeriesDetail, SeriesArticle, SeriesArticlePrereq, VoteSummary, BookmarkWithTitle } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let detail = $state<SeriesDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  // Votes for the series tag (treated as series vote)
  let seriesVotes = $state<VoteSummary | null>(null);
  let mySeriesVote = $state(0);

  // Votes per article
  let articleVotes = $state(new Map<string, VoteSummary>());

  // Bookmarks
  let bookmarkedUris = $state(new Set<string>());

  let isLoggedIn = $derived(!!getAuth());

  $effect(() => {
    loadSeries();
  });

  async function loadSeries() {
    loading = true;
    error = '';
    try {
      const d = await getSeries(id);
      detail = d;

      // Fetch votes for all articles in series
      const voteMap = new Map<string, VoteSummary>();
      const results = await Promise.all(
        d.articles.map(a =>
          getArticleVotes(a.article_uri).catch(() => ({
            target_uri: a.article_uri, score: 0, upvotes: 0, downvotes: 0,
          }))
        )
      );
      for (const v of results) {
        voteMap.set(v.target_uri, v);
      }
      articleVotes = voteMap;

      // Load bookmarks if logged in
      if (getAuth()) {
        try {
          const bks = await listBookmarks();
          bookmarkedUris = new Set(bks.map(b => b.article_uri));
        } catch { /* ok */ }
      }
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

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <div class="series-header">
    <h1>{detail.series.title}</h1>
    {#if detail.series.description}
      <p class="series-desc">{detail.series.description}</p>
    {/if}
    <div class="series-meta">
      <a href="#/tag?id={encodeURIComponent(detail.series.tag_id)}" class="tag">{detail.series.tag_id}</a>
      <span class="meta">{detail.articles.length} 篇文章</span>
      <span class="meta"><a href="#/profile?did={encodeURIComponent(detail.series.created_by)}">{detail.series.created_by}</a></span>
    </div>
  </div>

  <div class="series-articles">
    {#each detail.articles as article, i (article.article_uri)}
      <div class="series-item">
        <div class="item-number">{i + 1}</div>
        <div class="item-content">
          {#if prereqMap.has(article.article_uri)}
            <div class="item-prereqs">
              前置:
              {#each prereqMap.get(article.article_uri)! as pUri}
                {@const pArticle = articleByUri.get(pUri)}
                {#if pArticle}
                  <a href="#/article?uri={encodeURIComponent(pUri)}" class="prereq-link">#{pArticle.position} {pArticle.title}</a>
                {/if}
              {/each}
            </div>
          {/if}
          <a href="#/article?uri={encodeURIComponent(article.article_uri)}" class="item-title">
            {article.title}
          </a>
          {#if article.description}
            <p class="item-desc">{article.description}</p>
          {/if}
          <div class="item-actions">
            <span class="vote-score">{(articleVotes.get(article.article_uri)?.score) ?? 0}</span>
            <button class="action-btn" onclick={() => voteArticle(article.article_uri, 1)} disabled={!isLoggedIn} title="赞">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-6 0v4H5a2 2 0 00-2 2v7a2 2 0 002 2h14l-5-16z"/></svg>
            </button>
            <button class="action-btn" onclick={() => toggleBookmark(article.article_uri)} disabled={!isLoggedIn} title="收藏">
              <svg width="14" height="14" viewBox="0 0 24 24" fill={bookmarkedUris.has(article.article_uri) ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/></svg>
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .series-header {
    margin-bottom: 24px;
  }
  .series-header h1 {
    margin: 0 0 8px;
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
</style>
