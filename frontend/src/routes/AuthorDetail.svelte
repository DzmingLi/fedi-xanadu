<script lang="ts">
  import { getAuthor } from '../lib/api';
  import { t, getLocale } from '../lib/i18n/index.svelte';

  let { id } = $props<{ id: string }>();

  interface Author {
    id: string;
    name: string;
    did: string | null;
    orcid: string | null;
    affiliation: string | null;
    homepage: string | null;
  }
  interface AuthorBook {
    book_id: string;
    title: Record<string, string>;
    cover_url: string | null;
  }
  interface AuthorDetail {
    author: Author;
    books: AuthorBook[];
    article_count: number;
  }

  let detail = $state<AuthorDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || Object.values(field)[0] || '';
  }

  $effect(() => {
    loading = true;
    error = '';
    getAuthor(id).then((d: AuthorDetail) => {
      detail = d;
    }).catch(e => {
      error = e.message || 'Failed to load author';
    }).finally(() => {
      loading = false;
    });
  });
</script>

{#if loading}
  <p class="loading">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <div class="author-page">
    <div class="author-header">
      <h1>{detail.author.name}</h1>
      {#if detail.author.affiliation}
        <p class="affiliation">{detail.author.affiliation}</p>
      {/if}
      <div class="author-links">
        {#if detail.author.did}
          <a href="/profile?did={encodeURIComponent(detail.author.did)}" class="author-link-btn">Profile</a>
        {/if}
        {#if detail.author.orcid}
          <a href="https://orcid.org/{detail.author.orcid}" target="_blank" rel="noopener" class="author-link-btn">ORCID</a>
        {/if}
        {#if detail.author.homepage}
          <a href={detail.author.homepage} target="_blank" rel="noopener" class="author-link-btn">Homepage</a>
        {/if}
      </div>
    </div>

    {#if detail.books.length > 0}
      <section class="author-section">
        <h2>Books ({detail.books.length})</h2>
        <div class="book-grid">
          {#each detail.books as book}
            <a href="/book?id={encodeURIComponent(book.book_id)}" class="book-card">
              {#if book.cover_url}
                <img src={book.cover_url} alt={loc(book.title)} class="book-cover" />
              {:else}
                <div class="book-cover placeholder">
                  <span>{loc(book.title).charAt(0)}</span>
                </div>
              {/if}
              <span class="book-title">{loc(book.title)}</span>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if detail.article_count > 0 && detail.author.did}
      <section class="author-section">
        <h2>Articles ({detail.article_count})</h2>
        <p><a href="/profile?did={encodeURIComponent(detail.author.did)}">View all articles →</a></p>
      </section>
    {/if}
  </div>
{/if}

<style>
  .loading, .error { text-align: center; padding: 2rem; color: var(--text-hint); }
  .error { color: #c62828; }
  .author-page { max-width: 800px; margin: 0 auto; padding: 2rem 1rem; }
  .author-header { margin-bottom: 2rem; }
  .author-header h1 { font-family: var(--font-serif); font-size: 2rem; margin: 0 0 4px; }
  .affiliation { color: var(--text-secondary); margin: 0 0 8px; }
  .author-links { display: flex; gap: 8px; flex-wrap: wrap; }
  .author-link-btn {
    padding: 4px 12px; border-radius: 4px; font-size: 13px;
    background: var(--bg-secondary); color: var(--accent);
    text-decoration: none; border: 1px solid var(--border);
  }
  .author-link-btn:hover { background: var(--border); }
  .author-section { margin-bottom: 2rem; }
  .author-section h2 { font-size: 1.2rem; margin-bottom: 12px; color: var(--text-secondary); }
  .book-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 16px; }
  .book-card {
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    text-decoration: none; color: var(--text-primary); text-align: center;
  }
  .book-card:hover .book-title { color: var(--accent); }
  .book-cover { width: 120px; height: 160px; object-fit: cover; border-radius: 4px; box-shadow: 0 2px 6px rgba(0,0,0,0.1); }
  .book-cover.placeholder {
    display: flex; align-items: center; justify-content: center;
    background: var(--bg-secondary); color: var(--text-hint); font-size: 2rem;
    border: 1px solid var(--border);
  }
  .book-title { font-size: 13px; line-height: 1.3; }
</style>
