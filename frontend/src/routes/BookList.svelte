<script lang="ts">
  import { listBooks } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import type { Book } from '../lib/types';

  let books = $state<Book[]>([]);
  let loading = $state(true);
  let activeField = $state('');

  // Derive unique fields from book tags
  const FIELDS = ['math', 'cs', 'physics', 'economics'];
  let fieldCounts = $derived.by(() => {
    const m = new Map<string, number>();
    for (const b of books) {
      for (const tag of (b.tags || [])) {
        for (const f of FIELDS) {
          if (tag === f || tag.startsWith(f + '-') || tag.startsWith(f + '/')) {
            m.set(f, (m.get(f) || 0) + 1);
          }
        }
      }
    }
    return m;
  });

  let filteredBooks = $derived.by(() => {
    if (!activeField) return books;
    return books.filter(b => (b.tags || []).some(tag =>
      tag === activeField || tag.startsWith(activeField + '-') || tag.startsWith(activeField + '/')
    ));
  });

  $effect(() => {
    document.title = `${t('books.title')} — NightBoat`;
    loadBooks();
  });

  async function loadBooks() {
    loading = true;
    try { books = await listBooks(200, 0); } catch { /* */ }
    loading = false;
  }

  function fmtRating(r: number): string {
    return r > 0 ? r.toFixed(1) : '—';
  }

  const FIELD_LABELS: Record<string, string> = {
    math: 'Math', cs: 'CS', physics: 'Physics', economics: 'Econ',
  };
</script>

<div class="books-page">
  <div class="page-header">
    <div>
      <h1>{t('books.title')}</h1>
      <p class="subtitle">{t('books.subtitle')}</p>
    </div>
    {#if getAuth()}
      <a href="/new-book" class="add-book-btn">{t('books.addBook')}</a>
    {/if}
  </div>

  <div class="field-tabs">
    <button class="field-tab" class:active={activeField === ''} onclick={() => activeField = ''}>
      {t('home.all')} <span class="tab-count">{books.length}</span>
    </button>
    {#each FIELDS as f}
      {#if fieldCounts.get(f)}
        <button class="field-tab" class:active={activeField === f} onclick={() => activeField = f}>
          {FIELD_LABELS[f] || f} <span class="tab-count">{fieldCounts.get(f)}</span>
        </button>
      {/if}
    {/each}
  </div>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if filteredBooks.length === 0}
    <p class="empty">{t('books.empty')}</p>
  {:else}
    <div class="book-grid">
      {#each filteredBooks as book}
        <a href="/book?id={encodeURIComponent(book.id)}" class="book-card">
          {#if book.cover_url}
            <img src={book.cover_url} alt={book.title} class="book-cover" />
          {:else}
            <div class="book-cover placeholder">
              <span>{book.title.charAt(0)}</span>
            </div>
          {/if}
          <div class="book-info">
            <h3 class="book-title">{book.title}</h3>
            <p class="book-authors">{book.authors.join(', ')}</p>

            <div class="book-stats">
              <span class="rating" class:has-rating={book.avg_rating && book.avg_rating > 0}>
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2l3 7h7l-5.5 4.5L18.5 21 12 16.5 5.5 21l2-7.5L2 9h7z"/></svg>
                {fmtRating(book.avg_rating || 0)}
              </span>
              {#if book.rating_count}
                <span class="stat-text">({book.rating_count})</span>
              {/if}
              {#if book.reader_count}
                <span class="stat-text">{book.reader_count} readers</span>
              {/if}
            </div>

            {#if book.description}
              <p class="book-desc">{book.description.slice(0, 100)}{book.description.length > 100 ? '...' : ''}</p>
            {/if}

            {#if book.tags && book.tags.length > 0}
              <div class="book-tags">
                {#each book.tags.slice(0, 4) as tag}
                  <span class="tag">{tag}</span>
                {/each}
              </div>
            {/if}
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .books-page { max-width: 1080px; margin: 0 auto; }
  .page-header { display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 1rem; }
  .page-header h1 { margin: 0; }
  .subtitle { color: var(--text-secondary); font-size: 14px; margin: 4px 0 0; }
  .add-book-btn {
    display: inline-block; padding: 6px 16px; font-size: 13px;
    color: var(--accent); border: 1px solid var(--accent); border-radius: 4px;
    text-decoration: none; transition: all 0.15s; margin-top: 4px;
  }
  .add-book-btn:hover { background: var(--accent); color: white; text-decoration: none; }

  .field-tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; flex-wrap: wrap; }
  .field-tab { padding: 8px 14px; font-size: 13px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-secondary); cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .field-tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .tab-count { font-size: 11px; background: var(--border); color: var(--text-hint); padding: 1px 5px; border-radius: 8px; }

  .empty { color: var(--text-hint); font-size: 14px; }

  .book-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }
  .book-card {
    display: flex;
    gap: 14px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-white);
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .book-card:hover {
    border-color: var(--accent);
    box-shadow: 0 2px 12px rgba(0,0,0,0.06);
    text-decoration: none;
  }
  .book-cover {
    width: 80px;
    height: 110px;
    object-fit: cover;
    border-radius: 4px;
    flex-shrink: 0;
    box-shadow: 0 1px 4px rgba(0,0,0,0.1);
  }
  .book-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--border) 0%, var(--bg-page) 100%);
    color: var(--text-hint);
    font-size: 28px;
    font-family: var(--font-serif);
  }
  .book-info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .book-title {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    font-family: var(--font-serif);
    color: var(--text-primary);
    line-height: 1.3;
  }
  .book-authors {
    margin: 3px 0 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .book-stats {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    font-size: 12px;
  }
  .rating {
    display: flex;
    align-items: center;
    gap: 2px;
    color: var(--text-hint);
    font-weight: 600;
  }
  .rating.has-rating { color: #d97706; }
  .stat-text { color: var(--text-hint); }
  .book-desc {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.4;
    flex: 1;
  }
  .book-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
  .tag {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
    background: rgba(95,155,101,0.1);
    color: var(--accent);
  }
</style>
