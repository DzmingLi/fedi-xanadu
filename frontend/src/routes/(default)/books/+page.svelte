<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { keys } from '$lib/queries';
  import { listBooks } from '$lib/api';
  import { t, getLocale, onLocaleChange } from '$lib/i18n/index.svelte';
  import { getAuth } from '$lib/auth.svelte';
  import type { Book } from '$lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  const booksQuery = createQuery({
    queryKey: keys.books.all,
    queryFn: () => listBooks(100, 0),
  });

  let books = $derived($booksQuery.data ?? []);
  let loading = $derived($booksQuery.isPending);
</script>

<h1>{t('books.title')}</h1>
<p class="subtitle">{t('books.subtitle')}</p>

{#if getAuth()}
  <a href="/new-book" class="add-book-btn">{t('books.addBook')}</a>
{/if}

{#if loading}
  <p class="meta">Loading...</p>
{:else if books.length === 0}
  <p class="empty">{t('books.empty')}</p>
{:else}
  <div class="book-grid">
    {#each books as book}
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
          {#if book.description}
            <p class="book-desc">{book.description.slice(0, 120)}{book.description.length > 120 ? '...' : ''}</p>
          {/if}
        </div>
      </a>
    {/each}
  </div>
{/if}

<style>
  h1 { margin-bottom: 0; }
  .subtitle {
    color: var(--text-secondary);
    font-size: 14px;
    margin: 4px 0 16px;
  }
  .add-book-btn {
    display: inline-block;
    padding: 6px 16px;
    font-size: 13px;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    text-decoration: none;
    margin-bottom: 16px;
    transition: all 0.15s;
  }
  .add-book-btn:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }
  .empty { color: var(--text-hint); font-size: 14px; }
  .book-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }
  .book-card {
    display: flex;
    gap: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-white);
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .book-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 2px 8px rgba(0,0,0,0.05);
    text-decoration: none;
  }
  .book-cover {
    width: 60px;
    height: 80px;
    object-fit: cover;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .book-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--border);
    color: var(--text-hint);
    font-size: 24px;
    font-family: var(--font-serif);
  }
  .book-info { flex: 1; min-width: 0; }
  .book-title {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    font-family: var(--font-serif);
    color: var(--text-primary);
  }
  .book-authors {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .book-desc {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.4;
  }
</style>
