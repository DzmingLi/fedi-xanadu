<script lang="ts">
  import { getSeries, getSeriesTree, getArticleVotes, castVote, addBookmark, removeBookmark, listBookmarks } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { SeriesDetail, SeriesArticle, SeriesArticlePrereq, SeriesTreeNode, VoteSummary, BookmarkWithTitle } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let detail = $state<SeriesDetail | null>(null);
  let tree = $state<SeriesTreeNode | null>(null);
  let loading = $state(true);
  let error = $state('');
  let viewMode = $state<'flat' | 'tree'>('flat');

  // Votes per article
  let articleVotes = $state(new Map<string, VoteSummary>());

  // Bookmarks
  let bookmarkedUris = $state(new Set<string>());

  let isLoggedIn = $derived(!!getAuth());

  // Auto-switch to tree view if this series has children
  let hasChildren = $derived(detail ? detail.children.length > 0 : false);

  $effect(() => {
    loadSeries();
  });

  async function loadSeries() {
    loading = true;
    error = '';
    try {
      const d = await getSeries(id);
      detail = d;

      // If has children, load the full tree
      if (d.children.length > 0) {
        viewMode = 'tree';
        tree = await getSeriesTree(id);
      }

      // Collect all article URIs (flat + tree) for vote fetching
      const allArticleUris = new Set<string>();
      for (const a of d.articles) allArticleUris.add(a.article_uri);
      if (tree) collectTreeArticleUris(tree, allArticleUris);

      // Fetch votes
      const voteMap = new Map<string, VoteSummary>();
      const results = await Promise.all(
        [...allArticleUris].map(uri =>
          getArticleVotes(uri).catch(() => ({
            target_uri: uri, score: 0, upvotes: 0, downvotes: 0,
          }))
        )
      );
      for (const v of results) voteMap.set(v.target_uri, v);
      articleVotes = voteMap;

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

  function collectTreeArticleUris(node: SeriesTreeNode, set: Set<string>) {
    for (const a of node.articles) set.add(a.article_uri);
    for (const child of node.children) collectTreeArticleUris(child, set);
  }

  function countTreeArticles(node: SeriesTreeNode): number {
    let count = node.articles.length;
    for (const child of node.children) count += countTreeArticles(child);
    return count;
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
              <a href="#/article?uri={encodeURIComponent(pUri)}" class="prereq-link">{pArticle.title}</a>
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

{#snippet treeNode(node: SeriesTreeNode, depth: number)}
  <div class="tree-section" style="margin-left: {depth * 24}px">
    {#if depth > 0}
      <h3 class="section-title">
        <a href="#/series?id={encodeURIComponent(node.series.id)}">{node.series.title}</a>
      </h3>
      {#if node.series.description}
        <p class="section-desc">{node.series.description}</p>
      {/if}
    {/if}
    {#if node.articles.length > 0}
      <div class="series-articles">
        {#each node.articles as article, i (article.article_uri)}
          {@render articleItem(article, i)}
        {/each}
      </div>
    {/if}
    {#each node.children as child (child.series.id)}
      {@render treeNode(child, depth + 1)}
    {/each}
  </div>
{/snippet}

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <div class="series-header">
    <h1>{detail.series.title}</h1>
    {#if detail.series.long_description}
      <p class="series-long-desc">{detail.series.long_description}</p>
    {:else if detail.series.description}
      <p class="series-desc">{detail.series.description}</p>
    {/if}
    <div class="series-meta">
      {#if hasChildren && tree}
        {@const totalArticles = countTreeArticles(tree)}
        <span class="meta">{totalArticles} {t('series.articles')}</span>
        <span class="meta">{detail.children.length} {t('series.sections')}</span>
      {:else}
        <span class="meta">{detail.articles.length} {t('series.articles')}</span>
      {/if}
      <span class="meta"><a href="#/profile?did={encodeURIComponent(detail.series.created_by)}">{detail.series.created_by}</a></span>
    </div>
    {#if detail.translations && detail.translations.length > 0}
      <div class="series-translations">
        <span class="lang-current">{detail.series.lang}</span>
        {#each detail.translations as tr (tr.id)}
          <a href="#/series?id={encodeURIComponent(tr.id)}" class="lang-link">{tr.lang}</a>
        {/each}
      </div>
    {/if}
    {#if detail.series.parent_id}
      <div class="series-parent">
        <a href="#/series?id={encodeURIComponent(detail.series.parent_id)}">{t('series.backToParent')}</a>
      </div>
    {/if}
  </div>

  {#if viewMode === 'tree' && tree}
    {@render treeNode(tree, 0)}
  {:else}
    <div class="series-articles">
      {#each detail.articles as article, i (article.article_uri)}
        {@render articleItem(article, i)}
      {/each}
    </div>

    {#if detail.children.length > 0}
      <div class="children-list">
        <h2>{t('series.sections')}</h2>
        {#each detail.children as child (child.id)}
          <a href="#/series?id={encodeURIComponent(child.id)}" class="child-link">
            <span class="child-title">{child.title}</span>
            {#if child.description}
              <span class="child-desc">{child.description}</span>
            {/if}
          </a>
        {/each}
      </div>
    {/if}
  {/if}
{/if}

<style>
  .series-header {
    margin-bottom: 24px;
  }
  .series-header h1 {
    margin: 0 0 8px;
  }
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

  /* Parent link */
  .series-parent {
    margin-top: 8px;
    font-size: 13px;
  }
  .series-parent a {
    color: var(--accent);
    text-decoration: none;
  }
  .series-parent a:hover {
    text-decoration: underline;
  }

  /* Tree view */
  .tree-section {
    margin-bottom: 8px;
  }
  .section-title {
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 1.15rem;
    margin: 20px 0 4px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }
  .section-title a {
    color: var(--text-primary);
    text-decoration: none;
  }
  .section-title a:hover {
    color: var(--accent);
  }
  .section-desc {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 8px;
    line-height: 1.4;
  }

  /* Children list (flat view fallback) */
  .children-list {
    margin-top: 24px;
  }
  .children-list h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 0 0 12px;
  }
  .child-link {
    display: block;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: 6px;
    text-decoration: none;
    margin-bottom: 8px;
    transition: border-color 0.15s;
  }
  .child-link:hover {
    border-color: var(--accent);
  }
  .child-title {
    font-family: var(--font-serif);
    font-size: 1.05rem;
    color: var(--text-primary);
    display: block;
  }
  .child-desc {
    font-size: 13px;
    color: var(--text-secondary);
    display: block;
    margin-top: 4px;
  }
</style>
