<script lang="ts">
  import {
    getBookSeries, rateBookSeries, unrateBookSeries,
    upsertSeriesShortReview, listSeriesShortReviews, deleteSeriesShortReview,
  } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import ShortReviewComposer from '../lib/components/ShortReviewComposer.svelte';
  import ShortReviewCard from '../lib/components/ShortReviewCard.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import type { BookSeriesDetail, SeriesShortReview } from '../lib/types';

  let { id } = $props<{ id: string }>();

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  function formatRating(r: number) { return (r / 2).toFixed(r % 2 === 0 ? 0 : 1); }

  let detail = $state<BookSeriesDetail | null>(null);
  let shortReviews = $state<SeriesShortReview[]>([]);
  let loading = $state(true);
  let error = $state('');

  // Rating state
  let hoverRating = $state(0);
  let myRating = $state(0);
  let avgSeriesRating = $state(0);
  let seriesRatingCount = $state(0);

  let showComposer = $state(false);

  const contentUri = $derived(`at://nightboat/book-series/${id}`);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      const [d, sr] = await Promise.all([
        getBookSeries(id),
        listSeriesShortReviews(id),
      ]);
      detail = d;
      shortReviews = sr;
      myRating = d.my_series_rating || 0;
      avgSeriesRating = d.series_rating.avg_rating;
      seriesRatingCount = d.series_rating.rating_count;
    } catch (e: any) {
      error = e?.message || 'Error loading series';
    } finally {
      loading = false;
    }
  }

  async function submitRating(val: number) {
    if (!getAuth()) return;
    myRating = val;
    const stats = await rateBookSeries(id, val);
    avgSeriesRating = stats.avg_rating;
    seriesRatingCount = stats.rating_count;
  }

  async function clearRating() {
    if (!getAuth()) return;
    myRating = 0;
    const stats = await unrateBookSeries(id);
    avgSeriesRating = stats.avg_rating;
    seriesRatingCount = stats.rating_count;
  }

  async function handleShortReviewSubmit(body: string) {
    await upsertSeriesShortReview(id, { body });
    shortReviews = await listSeriesShortReviews(id);
    showComposer = false;
  }

  async function handleDeleteShortReview() {
    await deleteSeriesShortReview(id);
    shortReviews = await listSeriesShortReviews(id);
  }

  let myShortReview = $derived(
    getAuth() ? shortReviews.find(r => r.did === getAuth()!.did) || null : null
  );
</script>

{#if loading}
  <div class="page"><p class="loading">{t('common.loading')}</p></div>
{:else if error}
  <div class="page"><p class="error">{error}</p></div>
{:else if detail}
  {@const s = detail.series}
  <div class="series-detail-page">
    <div class="series-header">
      <div class="cover-area">
        {#if s.cover_url}
          <img src={s.cover_url} alt={loc(s.title)} class="series-cover" />
        {:else}
          <div class="series-cover placeholder"></div>
        {/if}
      </div>
      <div class="header-info">
        <h1 class="series-title">{loc(s.title)}</h1>
        {#if s.subtitle && Object.keys(s.subtitle).length > 0}
          <p class="series-subtitle">{loc(s.subtitle)}</p>
        {/if}
        {#if s.description && Object.keys(s.description).length > 0}
          <p class="series-desc">{loc(s.description)}</p>
        {/if}

        <!-- Member avg rating -->
        {#if detail.member_avg_rating > 0}
          <div class="rating-row">
            <span class="rating-label">{t('bookSeries.memberAvgRating')}:</span>
            <span class="rating-stars">
              {#each [1,2,3,4,5] as star}
                {@const val = detail.member_avg_rating / 2}
                <svg class="star-svg" viewBox="0 0 24 24" width="18" height="18">
                  {#if val >= star}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                  {:else if val >= star - 0.5}
                    <defs><clipPath id="sl-{star}"><rect x="0" y="0" width="12" height="24"/></clipPath></defs>
                    <path clip-path="url(#sl-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {:else}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {/if}
                </svg>
              {/each}
            </span>
            <span class="rating-value">{formatRating(detail.member_avg_rating)}</span>
            <span class="rating-count">({detail.member_rating_count})</span>
          </div>
        {/if}

        <!-- Series direct rating -->
        <div class="rating-row">
          <span class="rating-label">{t('bookSeries.seriesRating')}:</span>
          <span class="rating-stars">
            {#each [1,2,3,4,5] as star}
              {@const val = avgSeriesRating / 2}
              <svg class="star-svg" viewBox="0 0 24 24" width="18" height="18">
                {#if val >= star}
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                {:else if val >= star - 0.5}
                  <defs><clipPath id="ss-{star}"><rect x="0" y="0" width="12" height="24"/></clipPath></defs>
                  <path clip-path="url(#ss-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                {:else}
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                {/if}
              </svg>
            {/each}
          </span>
          {#if avgSeriesRating > 0}
            <span class="rating-value">{formatRating(avgSeriesRating)}</span>
            <span class="rating-count">({seriesRatingCount})</span>
          {:else}
            <span class="rating-none">—</span>
          {/if}
        </div>

        <!-- My rating -->
        {#if getAuth()}
          <div class="my-rating">
            <span class="my-rating-label">{t('books.myRating')}:</span>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span class="star-picker" onmouseleave={() => { hoverRating = 0; }}>
              {#each [1,2,3,4,5] as star}
                {@const activeVal = hoverRating || myRating}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <svg class="star-svg" viewBox="0 0 24 24" width="22" height="22">
                  <g clip-path="inset(0 50% 0 0)"
                     onmouseenter={() => { hoverRating = star * 2 - 1; }}
                     onclick={() => submitRating(star * 2 - 1)}
                     role="button" tabindex="-1">
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                          fill={activeVal >= star * 2 - 1 ? '#f59e0b' : 'none'}
                          stroke={activeVal >= star * 2 - 1 ? '#f59e0b' : '#ccc'}
                          stroke-width="1.5"/>
                  </g>
                  <g clip-path="inset(0 0 0 50%)"
                     onmouseenter={() => { hoverRating = star * 2; }}
                     onclick={() => submitRating(star * 2)}
                     role="button" tabindex="-1">
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                          fill={activeVal >= star * 2 ? '#f59e0b' : 'none'}
                          stroke={activeVal >= star * 2 ? '#f59e0b' : '#ccc'}
                          stroke-width="1.5"/>
                  </g>
                </svg>
              {/each}
            </span>
            {#if myRating > 0}
              <span class="my-rating-value">{formatRating(myRating)}</span>
              <button class="clear-rating" onclick={clearRating}>×</button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Member books -->
    <section class="section">
      <h2>{t('bookSeries.members')} ({detail.members.length})</h2>
      <div class="member-grid">
        {#each detail.members as book}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="member-card" onclick={() => navigate(`/book?id=${encodeURIComponent(book.id)}`)}>
            <div class="member-cover">
              {#if book.cover_url}
                <img src={book.cover_url} alt={loc(book.title)} />
              {:else}
                <div class="cover-placeholder"></div>
              {/if}
            </div>
            <div class="member-info">
              {#if book.position !== undefined}
                <span class="member-pos">#{book.position}</span>
              {/if}
              <span class="member-title">{loc(book.title)}</span>
              {#if book.avg_rating > 0}
                <span class="member-rating">★ {(book.avg_rating / 2).toFixed(1)}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </section>

    <!-- Short reviews -->
    <section class="section">
      <div class="section-header">
        <h2>{t('bookSeries.shortReviews')} ({shortReviews.length})</h2>
        {#if getAuth() && !myShortReview}
          <button class="write-btn" onclick={() => { showComposer = !showComposer; }}>
            {t('bookSeries.writeShortReview')}
          </button>
        {/if}
      </div>

      {#if showComposer}
        <div class="composer-wrap">
          <ShortReviewComposer
            onSubmit={handleShortReviewSubmit}
            placeholder={t('bookSeries.shortReviewPlaceholder')}
          />
        </div>
      {/if}

      {#if myShortReview}
        <div class="my-review-section">
          <ShortReviewCard review={myShortReview} onDelete={handleDeleteShortReview} />
          <button class="edit-btn" onclick={() => { showComposer = !showComposer; }}>
            {t('bookSeries.editShortReview')}
          </button>
        </div>
      {/if}

      {#if shortReviews.filter(r => !myShortReview || r.id !== myShortReview.id).length === 0 && !myShortReview}
        <p class="empty">{t('bookSeries.noShortReviews')}</p>
      {:else}
        <div class="reviews-list">
          {#each shortReviews.filter(r => !myShortReview || r.id !== myShortReview.id) as review}
            <ShortReviewCard {review} />
          {/each}
        </div>
      {/if}
    </section>

    <!-- Discussion / comments -->
    <section class="section">
      <h2>{t('bookSeries.discussion')}</h2>
      <CommentThread contentUri={contentUri} />
    </section>
  </div>
{/if}

<style>
  .series-detail-page {
    max-width: 960px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }
  .series-header {
    display: flex;
    gap: 2rem;
    margin-bottom: 2.5rem;
  }
  .cover-area { flex-shrink: 0; }
  .series-cover {
    width: 160px;
    aspect-ratio: 3/4;
    object-fit: cover;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  }
  .series-cover.placeholder {
    width: 160px;
    background: var(--border, #e5e7eb);
    border-radius: 8px;
    display: block;
  }
  .header-info { flex: 1; }
  .series-title { margin: 0 0 4px 0; font-size: 1.6rem; }
  .series-subtitle { margin: 0 0 8px 0; color: var(--text-muted, #6b7280); font-size: 1rem; }
  .series-desc { margin: 0 0 16px 0; font-size: 0.95rem; line-height: 1.6; }
  .rating-row { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
  .rating-label { font-size: 0.85rem; color: var(--text-muted, #6b7280); }
  .rating-stars { display: flex; }
  .star-svg { display: block; }
  .rating-value { font-weight: 600; font-size: 0.9rem; }
  .rating-count, .rating-none { font-size: 0.8rem; color: var(--text-muted, #6b7280); }
  .my-rating { display: flex; align-items: center; gap: 6px; margin-top: 8px; }
  .my-rating-label { font-size: 0.85rem; color: var(--text-muted, #6b7280); }
  .star-picker { display: flex; cursor: pointer; }
  .my-rating-value { font-size: 0.9rem; font-weight: 600; color: #f59e0b; }
  .clear-rating { background: none; border: none; cursor: pointer; color: var(--text-muted, #6b7280); font-size: 1rem; padding: 0; }
  .section { margin-top: 2.5rem; }
  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
  h2 { margin: 0 0 1rem 0; font-size: 1.15rem; }
  .section-header h2 { margin: 0; }
  .write-btn, .edit-btn {
    padding: 6px 14px;
    background: var(--accent, #6366f1);
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .edit-btn { background: var(--surface, #fff); color: var(--accent, #6366f1); border: 1px solid var(--accent, #6366f1); margin-top: 6px; }
  .composer-wrap { margin-bottom: 1.5rem; }
  .my-review-section { margin-bottom: 1.5rem; }
  .reviews-list { display: flex; flex-direction: column; gap: 1rem; }
  .empty { color: var(--text-muted, #6b7280); }
  .member-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 1rem;
  }
  .member-card {
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    transition: box-shadow 0.15s;
    background: var(--surface, #fff);
  }
  .member-card:hover { box-shadow: 0 3px 10px rgba(0,0,0,0.1); }
  .member-cover { aspect-ratio: 3/4; overflow: hidden; }
  .member-cover img { width: 100%; height: 100%; object-fit: cover; }
  .cover-placeholder { width: 100%; height: 100%; background: var(--border, #e5e7eb); }
  .member-info { padding: 8px 10px; display: flex; flex-direction: column; gap: 2px; }
  .member-pos { font-size: 0.75rem; color: var(--text-muted, #6b7280); }
  .member-title { font-size: 0.85rem; font-weight: 600; line-height: 1.3; }
  .member-rating { font-size: 0.78rem; color: #f59e0b; }
  .loading { color: var(--text-muted, #6b7280); }
  .error { color: #dc2626; }
  @media (max-width: 600px) {
    .series-header { flex-direction: column; }
    .series-cover { width: 120px; }
  }
</style>
