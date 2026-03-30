<script lang="ts">
  import { tagName } from '../display';
  import { authorName } from '../display';
  import { t } from '../i18n/index.svelte';
  import type { Article, ContentTeachRow, ContentPrereqBulkRow, Series } from '../types';

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
    </div>
  </a>
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
