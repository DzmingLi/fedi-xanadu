<script lang="ts">
  import { tagName } from '../display';
  import { authorName } from '../display';
  import { t } from '../i18n/index.svelte';
  import { getArticleContent } from '../api';
  import type { Article, ArticleContent, ContentTeachRow, ContentPrereqBulkRow, Series } from '../types';

  type CardVariant = 'home' | 'profile';

  let {
    article = undefined,
    series = undefined,
    articleCount = 0,
    articleTeaches = [],
    articlePrereqs = [],
    variant = 'home' as CardVariant,
  }: {
    article?: Article;
    series?: Series;
    articleCount?: number;
    articleTeaches?: ContentTeachRow[];
    articlePrereqs?: ContentPrereqBulkRow[];
    variant?: CardVariant;
  } = $props();

  function navToTag(e: MouseEvent | KeyboardEvent, tagId: string) {
    if (e instanceof KeyboardEvent && e.key !== 'Enter') return;
    e.preventDefault();
    e.stopPropagation();
    window.location.hash = `#/tag?id=${encodeURIComponent(tagId)}`;
  }

  let expanded = $state(false);
  let expandedContent = $state<ArticleContent | null>(null);
  let expandLoading = $state(false);
  let contentEl = $state<HTMLDivElement | undefined>(undefined);

  interface TocItem { id: string; text: string; level: number }
  let tocItems = $state<TocItem[]>([]);

  async function toggleExpand(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (expanded) {
      expanded = false;
      return;
    }
    if (!article) return;
    if (!expandedContent) {
      expandLoading = true;
      try {
        expandedContent = await getArticleContent(article.at_uri);
      } catch { /* */ }
      expandLoading = false;
    }
    expanded = true;
  }

  $effect(() => {
    if (!contentEl || !expandedContent) return;
    const headings = contentEl.querySelectorAll('h2, h3, h4');
    const items: TocItem[] = [];
    const usedIds = new Set<string>();
    headings.forEach(h => {
      let id = h.id || h.textContent!.trim().toLowerCase().replace(/[^\w\u4e00-\u9fff]+/g, '-').replace(/^-|-$/g, '');
      let finalId = id;
      let n = 1;
      while (usedIds.has(finalId)) { finalId = `${id}-${n++}`; }
      usedIds.add(finalId);
      h.id = finalId;
      items.push({ id: finalId, text: h.textContent!.trim(), level: parseInt(h.tagName[1]) });
    });
    tocItems = items;
  });

  function seriesAuthor(s: Series): string {
    if (s.author_handle) return `@${s.author_handle}`;
    return s.created_by.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
  }
</script>

{#if article}
  <a href="#/article?uri={encodeURIComponent(article.at_uri)}" class="post-card">
    <div class="card-top">
      {#if article.kind === 'question'}
        <span class="question-badge">{t('qa.questionBadge')}</span>
      {/if}
      <span class="post-title">{article.title}</span>
    </div>

    {#if articleTeaches.length > 0 || articlePrereqs.length > 0}
      <div class="card-tags">
        {#each articleTeaches as t}
          <span class="tag" role="link" tabindex="0" onclick={(e) => navToTag(e, t.tag_id)} onkeydown={(e) => navToTag(e, t.tag_id)}>{tagName(t.tag_names, t.tag_name, t.tag_id)}</span>
        {/each}
        {#each articlePrereqs as p}
          <span class="tag {p.prereq_type}" role="link" tabindex="0" onclick={(e) => navToTag(e, p.tag_id)} onkeydown={(e) => navToTag(e, p.tag_id)}>{tagName(p.tag_names, p.tag_name, p.tag_id)}</span>
        {/each}
      </div>
    {/if}

    {#if article.description}
      <p class="post-desc">{article.description}</p>
    {/if}

    <div class="card-bottom">
      <span class="post-meta">
        {#if variant === 'home'}{authorName(article)} &middot; {/if}{article.created_at.split(' ')[0]}
      </span>
      {#if variant === 'home'}
        <span class="card-stats">
          {#if article.vote_score !== 0}
            <span class="stat" title={t('home.votes')}>&#9650; {article.vote_score}</span>
          {/if}
          {#if article.bookmark_count > 0}
            <span class="stat" title={t('home.bookmarks')}>&#9733; {article.bookmark_count}</span>
          {/if}
        </span>
      {/if}
      <button class="expand-btn" onclick={toggleExpand} title={expanded ? t('home.collapse') : t('home.expand')}>
        {#if expandLoading}...{:else}{expanded ? '▲' : '▼'}{/if}
      </button>
    </div>
  </a>

  {#if expanded && expandedContent}
    <div class="expanded-article">
      <div class="expanded-body">
        {#if tocItems.length > 1}
          <nav class="expanded-toc">
            {#each tocItems as item}
              <a href="#{item.id}" class="toc-item toc-h{item.level}">{item.text}</a>
            {/each}
          </nav>
        {/if}
        <div class="content" bind:this={contentEl}>{@html expandedContent.html}</div>
      </div>
    </div>
  {/if}
{:else if series}
  <a href="#/series?id={encodeURIComponent(series.id)}" class="post-card series-card">
    <div class="card-top">
      <span class="post-title">{series.title}</span>
      <span class="series-badge">{t('home.series')}</span>
    </div>

    {#if series.description}
      <p class="post-desc">{series.description}</p>
    {/if}

    <div class="card-bottom">
      <span class="post-meta">
        {#if variant === 'home'}{seriesAuthor(series)} &middot; {/if}{series.created_at.split(' ')[0]}
      </span>
      <span class="card-stats">
        <span class="stat">{articleCount} {variant === 'home' ? t('home.lectures') : t('profile.lectureCount')}</span>
      </span>
    </div>
  </a>
{/if}

<style>
  .post-card {
    display: block;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px 20px;
    margin-bottom: 12px;
    transition: border-color 0.15s, box-shadow 0.15s;
    text-decoration: none;
    color: inherit;
  }
  .post-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .card-top {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .post-title {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    color: var(--text-primary);
    line-height: 1.35;
    flex: 1;
    min-width: 0;
  }
  .post-card:hover .post-title {
    color: var(--accent);
  }
  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
    margin-top: 6px;
  }
  .post-desc {
    margin: 8px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.55;
  }
  .card-bottom {
    margin-top: 10px;
    display: flex;
    align-items: center;
  }
  .post-meta {
    font-size: 13px;
    color: var(--text-hint);
  }
  .card-stats {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }
  .stat {
    font-size: 12px;
    color: var(--text-hint);
  }

  /* Series card */
  .series-card {
    border-left: 3px solid var(--accent);
  }
  .question-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  /* Expand button */
  .expand-btn {
    background: none;
    border: none;
    font-size: 11px;
    color: var(--text-hint);
    cursor: pointer;
    padding: 2px 6px;
    margin-left: 8px;
    border-radius: 3px;
    transition: all 0.15s;
  }
  .expand-btn:hover { background: var(--bg-hover); color: var(--accent); }

  /* Expanded article content */
  .expanded-article {
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 4px 4px;
    margin-top: -13px;
    margin-bottom: 12px;
    background: var(--bg-white);
    overflow: hidden;
  }
  .expanded-body {
    display: flex;
    gap: 0;
  }
  .expanded-toc {
    width: 180px;
    flex-shrink: 0;
    padding: 16px 12px;
    border-right: 1px solid var(--border);
    position: sticky;
    top: 0;
    align-self: flex-start;
    max-height: 80vh;
    overflow-y: auto;
  }
  .toc-item {
    display: block;
    font-size: 12px;
    color: var(--text-hint);
    text-decoration: none;
    padding: 3px 0;
    line-height: 1.4;
  }
  .toc-item:hover { color: var(--accent); }
  .toc-h3 { padding-left: 12px; }
  .toc-h4 { padding-left: 24px; }
  .expanded-article :global(.content) {
    flex: 1;
    min-width: 0;
    padding: 20px 28px;
    max-height: none;
  }

  @media (max-width: 768px) {
    .expanded-toc { display: none; }
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
</style>
