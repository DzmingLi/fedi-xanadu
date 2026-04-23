<script lang="ts">
  import { listBookSeries } from '../lib/api';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import type { BookSeriesListItem } from '../lib/types';

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  let series = $state<BookSeriesListItem[]>([]);
  let loading = $state(true);

  $effect(() => {
    listBookSeries().then(r => { series = r; loading = false; }).catch(() => { loading = false; });
  });
</script>

<div class="series-list-page">
  <div class="page-header">
    <h1>{t('bookSeries.title')}</h1>
  </div>

  {#if loading}
    <p class="loading">{t('common.loading')}</p>
  {:else if series.length === 0}
    <p class="empty">{t('bookSeries.empty')}</p>
  {:else}
    <div class="series-grid">
      {#each series as s}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="series-card" onclick={() => navigate(`/book-series-detail?id=${encodeURIComponent(s.id)}`)}>
          <div class="card-cover">
            {#if s.cover_url}
              <img src={s.cover_url} alt={loc(s.title)} />
            {:else}
              <div class="cover-placeholder"></div>
            {/if}
          </div>
          <div class="card-info">
            <div class="card-title">{loc(s.title)}</div>
            {#if s.subtitle && Object.keys(s.subtitle).length > 0}
              <div class="card-subtitle">{loc(s.subtitle)}</div>
            {/if}
            <div class="card-stats">
              <span>{s.member_count} {t('bookSeries.memberCount')}</span>
              {#if s.member_avg_rating > 0}
                <span>★ {(s.member_avg_rating / 2).toFixed(1)}</span>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .series-list-page {
    max-width: 960px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
  }
  h1 { margin: 0; font-size: 1.5rem; }
  .loading, .empty { color: var(--text-muted, #6b7280); }
  .series-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1.25rem;
  }
  .series-card {
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 10px;
    overflow: hidden;
    cursor: pointer;
    transition: box-shadow 0.15s;
    background: var(--surface, #fff);
  }
  .series-card:hover { box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
  .card-cover { aspect-ratio: 3/4; overflow: hidden; }
  .card-cover img { width: 100%; height: 100%; object-fit: cover; }
  .cover-placeholder { width: 100%; height: 100%; background: var(--border, #e5e7eb); }
  .card-info { padding: 10px 12px; }
  .card-title { font-weight: 600; font-size: 0.95rem; line-height: 1.3; margin-bottom: 2px; }
  .card-subtitle { font-size: 0.8rem; color: var(--text-muted, #6b7280); margin-bottom: 4px; }
  .card-stats { font-size: 0.8rem; color: var(--text-muted, #6b7280); display: flex; gap: 8px; }
</style>
