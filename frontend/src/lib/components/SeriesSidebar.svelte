<script lang="ts">
  import { getSeries } from '../api';
  import { getCachedSeries, setCachedSeries } from '../seriesCache';
  import { t } from '../i18n/index.svelte';
  import type { SeriesDetail } from '../types';

  let { seriesId, currentUri }: { seriesId: string; currentUri: string } = $props();

  let detail = $state<SeriesDetail | null>(null);
  let loading = $state(true);

  function articleHref(uri: string): string {
    return `#/article?uri=${encodeURIComponent(uri)}&series_id=${encodeURIComponent(seriesId)}`;
  }

  $effect(() => {
    if (!seriesId) return;
    loading = true;

    const cached = getCachedSeries(seriesId);
    if (cached) {
      detail = cached.detail;
      loading = false;
      return;
    }

    getSeries(seriesId).then((d) => {
      detail = d;
      setCachedSeries(seriesId, d);
      loading = false;
    }).catch(() => { loading = false; });
  });
</script>

{#if loading}
  <nav class="series-sidebar">
    <p class="ss-loading">...</p>
  </nav>
{:else if detail}
  <nav class="series-sidebar">
    <a href="#/series?id={encodeURIComponent(seriesId)}" class="ss-title">
      {detail.series.title}
    </a>
    {#each detail.articles as article, i (article.article_uri)}
      <a
        href={articleHref(article.article_uri)}
        class="ss-item"
        class:active={article.article_uri === currentUri}
      >
        <span class="ss-num">{i + 1}</span>
        <span class="ss-item-title">{article.title}</span>
      </a>
    {/each}
  </nav>
{/if}

<style>
  .series-sidebar {
    width: 260px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    max-height: calc(100vh - 4rem);
    position: sticky;
    top: 4rem;
    font-size: 13px;
    padding: 12px 0;
  }
  .ss-loading {
    padding: 12px 16px;
    color: var(--text-hint);
    margin: 0;
  }
  .ss-title {
    display: block;
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 14px;
    color: var(--accent);
    text-decoration: none;
    padding: 4px 16px 12px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .ss-title:hover { text-decoration: underline; }

  .ss-section {
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--text-hint);
    padding: 10px 16px 4px;
  }

  .ss-item {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 5px 16px;
    color: var(--text-secondary);
    text-decoration: none;
    line-height: 1.4;
    border-left: 2px solid transparent;
    transition: all 0.1s;
  }
  .ss-item:hover {
    background: var(--bg-hover, #f5f5f5);
    text-decoration: none;
  }
  .ss-item.active {
    color: var(--accent);
    border-left-color: var(--accent);
    background: rgba(95, 155, 101, 0.06);
    font-weight: 500;
  }
  .ss-num {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-hint);
    min-width: 14px;
  }
  .ss-item.active .ss-num { color: var(--accent); }
  .ss-item-title {
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  @media (max-width: 860px) {
    .series-sidebar { display: none; }
  }
</style>
