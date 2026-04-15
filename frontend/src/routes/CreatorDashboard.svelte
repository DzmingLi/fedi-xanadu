<script lang="ts">
  import {
    getCreatorStats, getCreatorArticles, getCreatorSeries, getCreatorTimeline,
    publishSeries, listDrafts, deleteDraft,
  } from '../lib/api';
  import type { CreatorStats, ArticleStats, TimelinePoint } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';

  let stats = $state<CreatorStats | null>(null);
  let articles = $state<ArticleStats[]>([]);
  let series = $state<any[]>([]);
  let drafts = $state<any[]>([]);
  let timeline = $state<TimelinePoint[]>([]);
  let loading = $state(true);
  let activeTab = $state<'published' | 'drafts' | 'analytics'>('published');

  $effect(() => { load(); });

  async function load() {
    loading = true;
    const [s, a, sr, d, tl] = await Promise.all([
      getCreatorStats().catch(() => null),
      getCreatorArticles().catch(() => []),
      getCreatorSeries().catch(() => []),
      listDrafts().catch(() => []),
      getCreatorTimeline().catch(() => []),
    ]);
    stats = s;
    articles = a;
    series = sr;
    drafts = d;
    timeline = tl;
    loading = false;
  }

  async function doPublish(id: string) {
    await publishSeries(id);
    await load();
  }

  async function doDeleteDraft(id: string) {
    await deleteDraft(id);
    drafts = drafts.filter(d => d.id !== id);
  }

  function maxVal(arr: TimelinePoint[], key: 'views' | 'comments' | 'bookmarks') {
    return Math.max(1, ...arr.map(p => p[key]));
  }

  let publishedSeries = $derived(series.filter(s => s.is_published));
  let draftSeries = $derived(series.filter(s => !s.is_published));
</script>

{#if loading}
  <div class="loading">{t('common.loading')}</div>
{:else}
  <div class="dashboard">
    <h1 class="page-title">{t('nav.creator')}</h1>

    <!-- Stats overview -->
    {#if stats}
      <div class="stats-grid">
        <div class="stat-card">
          <span class="stat-value">{stats.total_articles}</span>
          <span class="stat-label">{t('creator.articles')}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{stats.total_series}</span>
          <span class="stat-label">{t('creator.series')}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{stats.total_drafts}</span>
          <span class="stat-label">{t('creator.drafts')}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{stats.total_views}</span>
          <span class="stat-label">{t('creator.views')}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{stats.total_comments}</span>
          <span class="stat-label">{t('creator.comments')}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{stats.total_bookmarks}</span>
          <span class="stat-label">{t('creator.bookmarks')}</span>
        </div>
      </div>
    {/if}

    <!-- Tabs -->
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'published'} onclick={() => activeTab = 'published'}>
        {t('creator.published')}
      </button>
      <button class="tab" class:active={activeTab === 'drafts'} onclick={() => activeTab = 'drafts'}>
        {t('creator.drafts')} ({drafts.length + draftSeries.length})
      </button>
      <button class="tab" class:active={activeTab === 'analytics'} onclick={() => activeTab = 'analytics'}>
        {t('creator.analytics')}
      </button>
    </div>

    <!-- Published -->
    {#if activeTab === 'published'}
      <div class="content-list">
        {#if publishedSeries.length > 0}
          <h3 class="section-title">{t('creator.series')}</h3>
          {#each publishedSeries as s}
            <div class="content-item">
              <a href="/series?id={encodeURIComponent(s.id)}" class="item-title">{s.title}</a>
              <span class="item-meta">{s.lang} &middot; {s.category}</span>
            </div>
          {/each}
        {/if}

        <h3 class="section-title">{t('creator.articles')}</h3>
        {#if articles.length === 0}
          <p class="empty">{t('creator.noArticles')}</p>
        {:else}
          <table class="articles-table">
            <thead>
              <tr>
                <th>{t('creator.titleCol')}</th>
                <th>{t('creator.views')}</th>
                <th>{t('creator.comments')}</th>
                <th>{t('creator.bookmarks')}</th>
                <th>{t('creator.votes')}</th>
              </tr>
            </thead>
            <tbody>
              {#each articles as a}
                <tr>
                  <td><a href="/article?uri={encodeURIComponent(a.at_uri)}">{a.title}</a></td>
                  <td class="num">{a.views}</td>
                  <td class="num">{a.comments}</td>
                  <td class="num">{a.bookmarks}</td>
                  <td class="num">{a.votes}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

    <!-- Drafts -->
    {:else if activeTab === 'drafts'}
      <div class="content-list">
        {#if draftSeries.length > 0}
          <h3 class="section-title">{t('creator.draftSeries')}</h3>
          {#each draftSeries as s}
            <div class="content-item">
              <a href="/series-editor?id={encodeURIComponent(s.id)}" class="item-title">{s.title}</a>
              <span class="item-meta">{s.lang}</span>
              <button class="btn-sm btn-accent" onclick={() => doPublish(s.id)}>{t('creator.publish')}</button>
            </div>
          {/each}
        {/if}

        <h3 class="section-title">{t('creator.articleDrafts')}</h3>
        {#if drafts.length === 0}
          <p class="empty">{t('creator.noDrafts')}</p>
        {:else}
          {#each drafts as d}
            <div class="content-item">
              <a href="/article/new?draft={encodeURIComponent(d.id)}" class="item-title">{d.title}</a>
              <span class="item-meta">{d.content_format}</span>
              <button class="btn-sm btn-danger" onclick={() => doDeleteDraft(d.id)}>{t('common.delete')}</button>
            </div>
          {/each}
        {/if}
      </div>

    <!-- Analytics -->
    {:else if activeTab === 'analytics'}
      <div class="analytics">
        {#if timeline.length === 0}
          <p class="empty">{t('creator.noData')}</p>
        {:else}
          <h3 class="section-title">{t('creator.last30days')}</h3>
          <div class="chart-container">
            <div class="chart-label">{t('creator.views')}</div>
            <div class="chart">
              {#each timeline as point}
                <div class="bar-group" title="{point.day}: {point.views}">
                  <div class="bar bar-views" style="height: {(point.views / maxVal(timeline, 'views')) * 100}%"></div>
                </div>
              {/each}
            </div>
            <div class="chart-label">{t('creator.comments')}</div>
            <div class="chart">
              {#each timeline as point}
                <div class="bar-group" title="{point.day}: {point.comments}">
                  <div class="bar bar-comments" style="height: {(point.comments / maxVal(timeline, 'comments')) * 100}%"></div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .dashboard { max-width: 900px; margin: 0 auto; padding: 24px 16px; }
  .page-title { font-size: 1.5rem; font-weight: 600; margin: 0 0 20px; }
  .loading { text-align: center; padding: 60px; color: #888; }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 12px;
    margin-bottom: 24px;
  }
  .stat-card {
    background: #f8f8f8;
    border-radius: 8px;
    padding: 16px;
    text-align: center;
  }
  .stat-value { display: block; font-size: 1.8rem; font-weight: 700; color: var(--accent, #5f9b65); }
  .stat-label { display: block; font-size: 12px; color: #888; margin-top: 4px; }

  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid #e5e5e5;
    margin-bottom: 16px;
  }
  .tab {
    padding: 10px 20px;
    font-size: 14px;
    border: none;
    background: none;
    cursor: pointer;
    color: #666;
    border-bottom: 2px solid transparent;
  }
  .tab:hover { color: #333; }
  .tab.active { color: var(--accent, #5f9b65); border-bottom-color: var(--accent, #5f9b65); }

  .content-list { display: flex; flex-direction: column; gap: 8px; }
  .section-title { font-size: 14px; font-weight: 600; color: #666; margin: 16px 0 8px; text-transform: uppercase; letter-spacing: 0.04em; }
  .content-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: #fafafa;
    border-radius: 6px;
  }
  .item-title { flex: 1; font-size: 14px; color: #333; text-decoration: none; }
  .item-title:hover { color: var(--accent, #5f9b65); }
  .item-meta { font-size: 12px; color: #999; }
  .btn-sm {
    padding: 4px 10px; font-size: 11px; border: 1px solid #ddd;
    border-radius: 3px; background: none; cursor: pointer; color: #666;
  }
  .btn-sm:hover { border-color: var(--accent, #5f9b65); color: var(--accent, #5f9b65); }
  .btn-accent { background: var(--accent, #5f9b65); color: white; border-color: var(--accent, #5f9b65); }
  .btn-accent:hover { opacity: 0.9; color: white; }
  .btn-danger:hover { border-color: #c33; color: #c33; }
  .empty { color: #999; text-align: center; padding: 24px; font-size: 14px; }

  .articles-table { width: 100%; border-collapse: collapse; font-size: 14px; }
  .articles-table th { text-align: left; padding: 8px 10px; font-size: 12px; color: #888; font-weight: 500; border-bottom: 1px solid #e5e5e5; }
  .articles-table td { padding: 8px 10px; border-bottom: 1px solid #f0f0f0; }
  .articles-table a { color: #333; text-decoration: none; }
  .articles-table a:hover { color: var(--accent, #5f9b65); }
  .num { text-align: right; font-variant-numeric: tabular-nums; color: #666; }

  .analytics { display: flex; flex-direction: column; gap: 16px; }
  .chart-container { display: flex; flex-direction: column; gap: 20px; }
  .chart-label { font-size: 12px; font-weight: 600; color: #888; text-transform: uppercase; }
  .chart {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 100px;
    background: #f8f8f8;
    border-radius: 6px;
    padding: 8px 4px;
  }
  .bar-group { flex: 1; display: flex; align-items: flex-end; height: 100%; }
  .bar { width: 100%; min-height: 1px; border-radius: 2px 2px 0 0; transition: height 0.3s; }
  .bar-views { background: var(--accent, #5f9b65); }
  .bar-comments { background: #4a9eff; }
</style>
