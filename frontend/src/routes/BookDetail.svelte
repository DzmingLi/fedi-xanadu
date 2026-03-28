<script lang="ts">
  import { getBook, createArticle } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { BookDetail, BookEdition } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let detail = $state<BookDetail | null>(null);
  let loading = $state(true);
  let showEditions = $state(true);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      detail = await getBook(id);
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
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if detail}
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
      <div class="stats">
        <span>{detail.editions.length} {t('books.editionCount')}</span>
        <span>{detail.review_count} {t('books.reviewCount')}</span>
      </div>
      <div class="actions">
        {#if getAuth()}
          <a href="#/new?category=review&book_id={encodeURIComponent(id)}" class="action-btn primary">
            {t('books.writeReview')}
          </a>
        {/if}
        <a href="#/book-edit?id={encodeURIComponent(id)}" class="action-btn">
          {t('books.editInfo')}
        </a>
        <button class="action-btn" onclick={() => { showEditions = !showEditions; }}>
          {showEditions ? t('books.hideEditions') : t('books.showEditions')}
        </button>
      </div>
    </div>
  </div>

  <!-- Editions -->
  {#if showEditions}
    <div class="editions-section">
      <h2>{t('books.editions')}</h2>
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
    </div>
  {/if}

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
{/if}

<style>
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
  .stats {
    display: flex;
    gap: 16px;
    margin-top: 12px;
    font-size: 13px;
    color: var(--text-hint);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 16px;
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
  .action-btn.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .action-btn.primary:hover {
    opacity: 0.9;
  }

  /* Editions */
  .editions-section {
    margin-bottom: 32px;
  }
  .editions-section h2 {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 400;
    margin: 0 0 12px;
  }
  .edition-card {
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 8px;
    background: var(--bg-white);
  }
  .edition-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .edition-lang {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--bg-dim);
    color: var(--text-hint);
  }
  .edition-details {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .purchase-links {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .purchase-link {
    font-size: 12px;
    padding: 3px 10px;
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
    font-size: 13px;
    color: var(--text-hint);
    text-decoration: none;
    padding: 6px 0;
    transition: color 0.15s;
  }
  .add-edition-btn:hover { color: var(--accent); text-decoration: none; }

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
