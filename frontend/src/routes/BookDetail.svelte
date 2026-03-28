<script lang="ts">
  import { getBook, rateBook, setReadingStatus, removeReadingStatus } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { BookDetail, BookEdition } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let detail = $state<BookDetail | null>(null);
  let loading = $state(true);

  // Rating state
  let hoverRating = $state(0);
  let myRating = $state(0);
  let avgRating = $state(0);
  let ratingCount = $state(0);

  // Reading status state
  let readingStatus = $state('');
  let readingProgress = $state(0);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      detail = await getBook(id);
      avgRating = detail.rating.avg_rating;
      ratingCount = detail.rating.rating_count;
      myRating = detail.my_rating || 0;
      readingStatus = detail.my_reading_status?.status || '';
      readingProgress = detail.my_reading_status?.progress || 0;
    } catch { /* */ }
    loading = false;
  }

  function langLabel(lang: string): string {
    const map: Record<string, string> = {
      zh: '中文', en: 'English', ja: '日本語', ko: '한국어',
      fr: 'Français', de: 'Deutsch', es: 'Español', ru: 'Русский',
    };
    return map[lang] || lang;
  }

  function formatRating(val: number): string {
    return (val / 2).toFixed(1);
  }

  async function submitRating(r: number) {
    if (!getAuth()) return;
    myRating = r;
    try {
      const stats = await rateBook(id, r);
      avgRating = stats.avg_rating;
      ratingCount = stats.rating_count;
    } catch { /* */ }
  }

  async function setStatus(status: string) {
    if (!getAuth()) return;
    if (readingStatus === status) {
      // Toggle off
      readingStatus = '';
      readingProgress = 0;
      try { await removeReadingStatus(id); } catch { /* */ }
    } else {
      readingStatus = status;
      if (status === 'finished') readingProgress = 100;
      else if (status === 'want_to_read') readingProgress = 0;
      try { await setReadingStatus(id, status, readingProgress); } catch { /* */ }
    }
  }

  async function updateProgress() {
    if (!getAuth() || readingStatus !== 'reading') return;
    try { await setReadingStatus(id, 'reading', readingProgress); } catch { /* */ }
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if detail}
  <div class="book-layout">
    <div class="book-main">
      <!-- Header -->
      <div class="book-header">
        {#if detail.book.cover_url}
          <img src={detail.book.cover_url} alt={detail.book.title} class="cover" />
        {:else}
          <div class="cover placeholder">
            <span>{detail.book.title.charAt(0)}</span>
          </div>
        {/if}
        <div class="book-meta">
          <h1>{detail.book.title}</h1>
          <p class="authors">{detail.book.authors.join(', ')}</p>
          {#if detail.book.description}
            <p class="description">{detail.book.description}</p>
          {/if}

          <!-- Rating display -->
          <div class="rating-row">
            <span class="rating-stars-display">
              {#each [1,2,3,4,5] as star}
                {@const val = avgRating / 2}
                {@const filled = val >= star}
                {@const half = !filled && val >= star - 0.5}
                <svg class="star-svg" viewBox="0 0 24 24" width="28" height="28">
                  <defs>
                    <clipPath id="star-left-{star}"><rect x="0" y="0" width="12" height="24"/></clipPath>
                    <clipPath id="star-right-{star}"><rect x="12" y="0" width="12" height="24"/></clipPath>
                  </defs>
                  {#if filled}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                  {:else if half}
                    <path clip-path="url(#star-left-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                    <path clip-path="url(#star-right-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {:else}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {/if}
                </svg>
              {/each}
            </span>
            <span class="rating-value">{formatRating(avgRating)}</span>
            <span class="rating-count">({ratingCount})</span>
          </div>

          <!-- User rating -->
          {#if getAuth()}
            <div class="my-rating">
              <span class="my-rating-label">{t('books.myRating')}:</span>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <span class="star-picker" onmouseleave={() => { hoverRating = 0; }}>
                {#each [1,2,3,4,5] as star}
                  {@const activeVal = hoverRating || myRating}
                  {@const leftActive = activeVal >= star * 2 - 1}
                  {@const rightActive = activeVal >= star * 2}
                  <svg class="star-svg" viewBox="0 0 24 24" width="24" height="24">
                    <!-- Left half (odd value) -->
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <g clip-path="inset(0 50% 0 0)"
                       onmouseenter={() => { hoverRating = star * 2 - 1; }}
                       onclick={() => submitRating(star * 2 - 1)}
                       role="button" tabindex="-1">
                      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                            fill={leftActive ? '#f59e0b' : 'none'}
                            stroke={leftActive ? '#f59e0b' : '#ccc'}
                            stroke-width="1.5"/>
                    </g>
                    <!-- Right half (even value) -->
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <g clip-path="inset(0 0 0 50%)"
                       onmouseenter={() => { hoverRating = star * 2; }}
                       onclick={() => submitRating(star * 2)}
                       role="button" tabindex="-1">
                      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                            fill={rightActive ? '#f59e0b' : 'none'}
                            stroke={rightActive ? '#f59e0b' : '#ccc'}
                            stroke-width="1.5"/>
                    </g>
                  </svg>
                {/each}
              </span>
              {#if myRating > 0}
                <span class="my-rating-value">{formatRating(myRating)}</span>
              {/if}
            </div>
          {/if}

          <!-- Reading status + actions -->
          <div class="actions">
            {#if getAuth()}
              <button class="action-btn" class:active={readingStatus === 'want_to_read'} onclick={() => setStatus('want_to_read')}>
                {t('books.wantToRead')}
              </button>
              <button class="action-btn" class:active={readingStatus === 'reading'} onclick={() => setStatus('reading')}>
                {t('books.reading')}
              </button>
              <button class="action-btn" class:active={readingStatus === 'finished'} onclick={() => setStatus('finished')}>
                {t('books.finished')}
              </button>
            {/if}
            {#if getAuth()}
              <a href="#/new?category=review&book_id={encodeURIComponent(id)}" class="action-btn primary">
                {t('books.writeReview')}
              </a>
            {/if}
            <a href="#/book-edit?id={encodeURIComponent(id)}" class="action-btn">
              {t('books.editInfo')}
            </a>
          </div>

          <!-- Progress bar for "reading" -->
          {#if readingStatus === 'reading'}
            <div class="progress-section">
              <label class="progress-label">
                {t('books.progress')}: {readingProgress}%
                <input type="range" min="0" max="100" bind:value={readingProgress} onchange={updateProgress} class="progress-slider" />
              </label>
            </div>
          {/if}
        </div>
      </div>

      <!-- Reviews -->
      <div class="reviews-section">
        <h2>{t('books.reviews')}</h2>
        {#if detail.reviews.length === 0}
          <p class="empty">{t('books.noReviews')}</p>
        {:else}
          {#each detail.reviews as review}
            <PostCard article={review} articleTeaches={[]} variant="profile" />
          {/each}
        {/if}
      </div>
    </div>

    <!-- Right sidebar: Editions -->
    <aside class="book-sidebar">
      <h3>{t('books.editions')}</h3>
      {#each detail.editions as ed}
        <div class="edition-card">
          <div class="edition-top">
            <strong>{ed.title}</strong>
            <span class="edition-lang">{langLabel(ed.lang)}</span>
          </div>
          <div class="edition-details">
            {#if ed.isbn}<span>ISBN: {ed.isbn}</span>{/if}
            {#if ed.publisher}<span>{ed.publisher}</span>{/if}
            {#if ed.year}<span>{ed.year}</span>{/if}
            {#if ed.translators.length > 0}
              <span>{t('books.translators')}: {ed.translators.join(', ')}</span>
            {/if}
          </div>
          {#if ed.purchase_links.length > 0}
            <div class="purchase-links">
              {#each ed.purchase_links as link}
                <a href={link.url} target="_blank" rel="noopener" class="purchase-link">{link.label}</a>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
      {#if getAuth()}
        <a href="#/book-edition?book_id={encodeURIComponent(id)}" class="add-edition-btn">
          + {t('books.addEdition')}
        </a>
      {/if}

      <div class="sidebar-stats">
        <span>{detail.editions.length} {t('books.editionCount')}</span>
        <span>{detail.review_count} {t('books.reviewCount')}</span>
      </div>
    </aside>
  </div>
{/if}

<style>
  .book-layout {
    display: flex;
    gap: 32px;
  }
  .book-main {
    flex: 1;
    min-width: 0;
  }
  .book-sidebar {
    width: 280px;
    flex-shrink: 0;
  }
  .book-sidebar h3 {
    font-family: var(--font-serif);
    font-size: 1rem;
    font-weight: 400;
    margin: 0 0 12px;
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .book-layout {
      flex-direction: column;
    }
    .book-sidebar {
      width: 100%;
    }
  }

  .book-header {
    display: flex;
    gap: 24px;
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border);
  }
  .cover {
    width: 140px;
    height: 200px;
    object-fit: cover;
    border-radius: 6px;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }
  .cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--border);
    color: var(--text-hint);
    font-size: 48px;
    font-family: var(--font-serif);
  }
  .book-meta { flex: 1; }
  .book-meta h1 {
    margin: 0;
    font-family: var(--font-serif);
    font-size: 1.6rem;
  }
  .authors {
    margin: 4px 0 0;
    font-size: 15px;
    color: var(--text-secondary);
  }
  .description {
    margin: 12px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  /* Rating display */
  .rating-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .rating-stars-display {
    display: inline-flex;
    gap: 2px;
    align-items: center;
  }
  .rating-value {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .rating-count {
    font-size: 13px;
    color: var(--text-hint);
  }

  /* User rating picker */
  .my-rating {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .my-rating-label { flex-shrink: 0; }
  .my-rating-value {
    font-weight: 600;
    color: #f59e0b;
  }
  .star-picker {
    display: inline-flex;
    gap: 2px;
    cursor: pointer;
    align-items: center;
  }
  .star-svg {
    display: block;
  }
  .star-svg g {
    cursor: pointer;
  }

  /* Actions */
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    flex-wrap: wrap;
  }
  .action-btn {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    color: var(--text-secondary);
    text-decoration: none;
    cursor: pointer;
    transition: all 0.15s;
  }
  .action-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .action-btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .action-btn.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .action-btn.primary:hover { opacity: 0.9; }

  /* Progress */
  .progress-section {
    margin-top: 10px;
  }
  .progress-label {
    font-size: 13px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .progress-slider {
    flex: 1;
    max-width: 200px;
    accent-color: var(--accent);
  }

  /* Editions (sidebar) */
  .edition-card {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 8px;
    background: var(--bg-white);
  }
  .edition-top {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .edition-lang {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-dim);
    color: var(--text-hint);
  }
  .edition-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-hint);
  }
  .purchase-links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .purchase-link {
    font-size: 11px;
    padding: 2px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    text-decoration: none;
    transition: all 0.15s;
  }
  .purchase-link:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }
  .add-edition-btn {
    display: inline-block;
    font-size: 12px;
    color: var(--text-hint);
    text-decoration: none;
    padding: 4px 0;
    transition: color 0.15s;
  }
  .add-edition-btn:hover { color: var(--accent); text-decoration: none; }
  .sidebar-stats {
    display: flex;
    gap: 12px;
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-hint);
  }

  /* Reviews */
  .reviews-section h2 {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 400;
    margin: 0 0 12px;
  }
  .empty {
    color: var(--text-hint);
    font-size: 14px;
  }
</style>
