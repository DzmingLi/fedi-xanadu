<script lang="ts">
  import { getAuthor, setAuthorNames } from '../lib/api';
  import { t, getLocale, LOCALES } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import { authorDisplayName } from '../lib/display';

  let { id } = $props<{ id: string }>();

  interface Author {
    id: string;
    name: string;
    did: string | null;
    orcid: string | null;
    affiliation: string | null;
    homepage: string | null;
    original_names?: Record<string, string> | null;
    official_translations?: Record<string, string> | null;
    translations?: Record<string, string> | null;
  }
  interface AuthorBook {
    book_id: string;
    title: Record<string, string>;
    cover_url: string | null;
  }
  interface AuthorTerm {
    term_id: string;
    title: string;
    code: string | null;
    institution: string | null;
    semester: string | null;
  }
  interface AuthorDetail {
    author: Author;
    books: AuthorBook[];
    terms: AuthorTerm[];
    article_count: number;
  }

  let detail = $state<AuthorDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  // Name editing — three buckets in display priority:
  //   original_names → official_translations → (fallback: canonical `name`)
  // `translations` is a stored-but-not-displayed pool for search + "other".
  let showEdit = $state(false);
  let editOriginalNames = $state<Record<string, string>>({});
  let editOfficialTranslations = $state<Record<string, string>>({});
  let editTranslations = $state<Record<string, string>>({});
  let editSaving = $state(false);
  let editError = $state('');

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || Object.values(field)[0] || '';
  }

  function openEdit() {
    if (!detail) return;
    editOriginalNames = { ...(detail.author.original_names || {}) };
    editOfficialTranslations = { ...(detail.author.official_translations || {}) };
    editTranslations = { ...(detail.author.translations || {}) };
    for (const l of LOCALES) {
      if (!(l.code in editOriginalNames)) editOriginalNames[l.code] = '';
      if (!(l.code in editOfficialTranslations)) editOfficialTranslations[l.code] = '';
      if (!(l.code in editTranslations)) editTranslations[l.code] = '';
    }
    editError = '';
    showEdit = true;
  }

  async function saveNames() {
    if (!detail) return;
    editSaving = true;
    editError = '';
    try {
      const clean = (m: Record<string, string>) => Object.fromEntries(
        Object.entries(m).filter(([_, v]) => v.trim()).map(([k, v]) => [k, v.trim()])
      );
      const updated = await setAuthorNames(
        detail.author.id,
        clean(editOriginalNames),
        clean(editOfficialTranslations),
        clean(editTranslations),
      );
      detail = { ...detail, author: { ...detail.author,
        original_names: updated.original_names,
        official_translations: updated.official_translations,
        translations: updated.translations,
      } };
      showEdit = false;
    } catch (e: any) {
      editError = e.message || 'Failed to save';
    } finally {
      editSaving = false;
    }
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
      <h1>
        {authorDisplayName(detail.author)}
        {#if authorDisplayName(detail.author) !== detail.author.name}
          <span class="original-name">· {detail.author.name}</span>
        {/if}
      </h1>
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
        {#if getAuth()}
          <button class="author-link-btn" onclick={openEdit}>{t('authors.editNames')}</button>
        {/if}
      </div>
    </div>

    {#if showEdit}
      <div class="edit-panel">
        <h3>{t('authors.editNamesTitle', detail.author.name)}</h3>
        {#if editError}<p class="edit-error">{editError}</p>{/if}

        <p class="edit-section-hint">{t('authors.originalNamesHint')}</p>
        {#each LOCALES as loc (loc.code)}
          <label class="edit-row">
            <span class="edit-label">{loc.label}</span>
            <input bind:value={editOriginalNames[loc.code]} placeholder={loc.code === 'en' ? detail.author.name : ''} />
          </label>
        {/each}

        <p class="edit-section-hint">{t('authors.officialTranslationsHint')}</p>
        {#each LOCALES as loc (loc.code)}
          <label class="edit-row">
            <span class="edit-label">{loc.label}</span>
            <input bind:value={editOfficialTranslations[loc.code]} />
          </label>
        {/each}

        <p class="edit-section-hint">{t('authors.translationsHint')}</p>
        {#each LOCALES as loc (loc.code)}
          <label class="edit-row">
            <span class="edit-label">{loc.label}</span>
            <input bind:value={editTranslations[loc.code]} />
          </label>
        {/each}

        <div class="edit-actions">
          <button class="author-link-btn" onclick={() => showEdit = false} disabled={editSaving}>{t('common.cancel')}</button>
          <button class="author-link-btn primary" onclick={saveNames} disabled={editSaving}>
            {editSaving ? t('common.saving') : t('common.save')}
          </button>
        </div>
      </div>
    {/if}

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

    {#if detail.terms.length > 0}
      <section class="author-section">
        <h2>Terms ({detail.terms.length})</h2>
        <ul class="term-list">
          {#each detail.terms as term}
            <li class="term-item">
              <a href="/term?id={encodeURIComponent(term.term_id)}" class="term-link">
                {#if term.code}<span class="term-code">{term.code}</span>{/if}
                <span class="term-title">{term.title}</span>
              </a>
              {#if term.institution || term.semester}
                <span class="term-meta">
                  {term.institution ?? ''}{#if term.institution && term.semester} · {/if}{term.semester ?? ''}
                </span>
              {/if}
            </li>
          {/each}
        </ul>
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
  .original-name { font-size: 0.9rem; color: var(--text-hint); font-weight: 400; font-family: var(--font-sans); margin-left: 8px; }
  .edit-panel { margin-top: 16px; padding: 14px; background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px; max-width: 520px; }
  .edit-panel h3 { margin: 0 0 12px; font-size: 14px; font-family: var(--font-serif); font-weight: 500; }
  .edit-section-hint { font-size: 12px; color: var(--text-hint); margin: 10px 0 6px; }
  .edit-row { display: grid; grid-template-columns: 120px 1fr; gap: 10px; align-items: center; margin-bottom: 8px; }
  .edit-label { font-size: 13px; color: var(--text-secondary); }
  .edit-row input { padding: 6px 8px; font-size: 13px; border: 1px solid var(--border); border-radius: 4px; }
  .edit-error { color: #c62828; font-size: 13px; margin: 0 0 10px; }
  .edit-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 10px; }
  .author-link-btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
  .author-link-btn.primary:hover { background: var(--accent); filter: brightness(0.95); }
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

  .term-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 10px; }
  .term-item { display: flex; flex-direction: column; gap: 2px; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); }
  .term-link { display: flex; gap: 8px; align-items: baseline; color: var(--text-primary); text-decoration: none; }
  .term-link:hover .term-title { color: var(--accent); }
  .term-code { font-family: monospace; font-size: 12px; color: var(--text-hint); }
  .term-title { font-family: var(--font-serif); font-size: 15px; }
  .term-meta { font-size: 12px; color: var(--text-hint); }
</style>
