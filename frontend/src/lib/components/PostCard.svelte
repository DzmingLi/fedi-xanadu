<script lang="ts">
  import { tagName } from '../display';
  import { authorName } from '../display';
  import { t } from '../i18n';
  import type { Article, ArticleTagRow, ArticlePrereqBulkRow, Series } from '../types';

  type CardVariant = 'home' | 'profile';

  let {
    article = undefined,
    series = undefined,
    articleCount = 0,
    articleTags = [],
    articlePrereqs = [],
    variant = 'home' as CardVariant,
  }: {
    article?: Article;
    series?: Series;
    articleCount?: number;
    articleTags?: ArticleTagRow[];
    articlePrereqs?: ArticlePrereqBulkRow[];
    variant?: CardVariant;
  } = $props();

  function navToTag(e: MouseEvent | KeyboardEvent, tagId: string) {
    if (e instanceof KeyboardEvent && e.key !== 'Enter') return;
    e.preventDefault();
    e.stopPropagation();
    window.location.hash = `#/tag?id=${encodeURIComponent(tagId)}`;
  }
</script>

{#if article}
  <a href="#/article?uri={encodeURIComponent(article.at_uri)}" class="post-card">
    <div class="card-top">
      <span class="post-title">{article.title}</span>
      <div class="card-tags">
        {#each articleTags as t}
          <span class="tag" role="link" tabindex="0" onclick={(e) => navToTag(e, t.tag_id)} onkeydown={(e) => navToTag(e, t.tag_id)}>{tagName(t.tag_names, t.tag_name, t.tag_id)}</span>
        {/each}
        {#each articlePrereqs as p}
          <span class="tag {p.prereq_type}" role="link" tabindex="0" onclick={(e) => navToTag(e, p.tag_id)} onkeydown={(e) => navToTag(e, p.tag_id)}>{tagName(p.tag_names, p.tag_name, p.tag_id)}</span>
        {/each}
      </div>
    </div>

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
    {#if variant === 'home'}
      <div class="series-badge-abs">{t('home.series')}</div>
    {/if}
    <div class="card-top">
      <span class="post-title">{series.title}</span>
      {#if variant === 'profile'}
        <span class="series-badge-inline">{t('profile.seriesBadge')}</span>
      {/if}
      {#if series.tag_name && variant === 'home'}
        <div class="card-tags">
          <span class="tag" role="link" tabindex="0" onclick={(e) => navToTag(e, series.tag_id)} onkeydown={(e) => navToTag(e, series.tag_id)}>{tagName(series.tag_names, series.tag_name || '', series.tag_id)}</span>
        </div>
      {/if}
    </div>

    {#if series.description}
      <p class="post-desc">{series.description}</p>
    {/if}

    <div class="card-bottom">
      <span class="post-meta">{series.created_at.split(' ')[0]}</span>
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
    flex-shrink: 0;
    padding-top: 3px;
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

  /* Series card variants */
  .series-card {
    border-left: 3px solid var(--accent);
    position: relative;
  }
  .series-badge-abs {
    position: absolute;
    top: 8px;
    right: 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
  }
  .series-badge-inline {
    font-size: 11px;
    background: rgba(95, 155, 101, 0.12);
    color: var(--accent);
    padding: 1px 8px;
    border-radius: 3px;
    flex-shrink: 0;
  }
</style>
