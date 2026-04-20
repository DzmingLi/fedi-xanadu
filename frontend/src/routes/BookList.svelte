<script lang="ts">
  import { listBooks } from '../lib/api';
  import { tagStore } from '../lib/tagStore.svelte';
  $effect(() => { tagStore.ensure(); });
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import type { Book } from '../lib/types';

  /** Resolve a localized field (Record<string, string>) to the current locale with fallback. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const locale = getLocale();
    return field[locale] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  let books = $state<Book[]>([]);
  let loading = $state(true);
  let activeField = $state('');
  let search = $state('');

  // Field tabs: a book lives under a field if its topic closure (which
  // already expands group siblings and walks tag_parents) includes any
  // of the field's anchor tag ids. Anchors cover both en + zh siblings
  // so the tabs work regardless of which label was attached.
  const FIELDS: { key: string; anchors: string[] }[] = [
    { key: 'math',     anchors: ['Math', '数学', 'Mathematics'] },
    { key: 'cs',       anchors: ['Computer Science', 'Cs', '计算机科学'] },
    { key: 'physics',  anchors: ['Physics', '物理学'] },
    { key: 'economics',anchors: ['Economics', '经济学'] },
  ];
  function bookInField(b: Book, fieldKey: string): boolean {
    const f = FIELDS.find(x => x.key === fieldKey);
    if (!f) return false;
    const topics = b.topics || [];
    return f.anchors.some(a => topics.includes(a));
  }
  let fieldCounts = $derived.by(() => {
    const m = new Map<string, number>();
    for (const f of FIELDS) {
      m.set(f.key, books.filter(b => bookInField(b, f.key)).length);
    }
    return m;
  });

  let filteredBooks = $derived.by(() => {
    let list = books;
    if (activeField) {
      list = list.filter(b => bookInField(b, activeField));
    }
    const q = search.trim().toLowerCase();
    if (!q) return list;
    return list.filter(b => {
      const haystacks = [
        ...Object.values(b.title || {}),
        ...Object.values(b.subtitle || {}),
        ...(b.authors || []),
        b.abbreviation || '',
        ...(b.tags || []),
      ];
      return haystacks.some(s => s && s.toLowerCase().includes(q));
    });
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

  <div class="book-search">
    <input
      type="search"
      bind:value={search}
      placeholder={t('books.searchPlaceholder')}
      class="search-input"
    />
    {#if search}
      <button class="search-clear" onclick={() => search = ''} aria-label="clear">×</button>
    {/if}
  </div>

  <div class="field-tabs">
    <button class="field-tab" class:active={activeField === ''} onclick={() => activeField = ''}>
      {t('home.all')} <span class="tab-count">{books.length}</span>
    </button>
    {#each FIELDS as f}
      {#if fieldCounts.get(f.key)}
        <button class="field-tab" class:active={activeField === f.key} onclick={() => activeField = f.key}>
          {FIELD_LABELS[f.key] || f.key} <span class="tab-count">{fieldCounts.get(f.key)}</span>
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
            <img src={book.cover_url} alt={loc(book.title)} class="book-cover" />
          {:else}
            <div class="book-cover placeholder">
              <span>{loc(book.title).charAt(0)}</span>
            </div>
          {/if}
          <div class="book-info">
            <h3 class="book-title">{loc(book.title)}</h3>
            {#if book.subtitle && loc(book.subtitle)}
              <p class="book-subtitle">{loc(book.subtitle)}</p>
            {/if}
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

            {#if loc(book.description)}
              <p class="book-desc">{loc(book.description).slice(0, 100)}{loc(book.description).length > 100 ? '...' : ''}</p>
            {/if}

            {#if book.tags && book.tags.length > 0}
              <div class="book-tags">
                {#each book.tags.slice(0, 4) as tag}
                  <span class="tag">{tagStore.localize(tag)}</span>
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

  .book-search { position: relative; margin-bottom: 12px; }
  .search-input {
    width: 100%; padding: 8px 32px 8px 12px; font-size: 14px;
    border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg-white); color: var(--text-primary);
    font-family: var(--font-sans);
  }
  .search-input:focus { outline: none; border-color: var(--accent); }
  .search-clear {
    position: absolute; right: 6px; top: 50%; transform: translateY(-50%);
    background: none; border: none; font-size: 18px; line-height: 1;
    color: var(--text-hint); cursor: pointer; padding: 2px 8px;
  }
  .search-clear:hover { color: var(--text-primary); }

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
  .book-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    font-style: italic;
    line-height: 1.35;
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
