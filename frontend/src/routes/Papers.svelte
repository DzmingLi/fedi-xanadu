<script lang="ts">
  import { listPapers, importPaper } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import type { PaperListItem } from '../lib/generated/PaperListItem';

  let papers = $state<PaperListItem[]>([]);
  let loading = $state(true);
  let error = $state('');

  // DOI / arxiv autofill via OpenAlex. Pasting a DOI → POST /papers/import
  // hits OpenAlex, builds a paper + version rows + author links in one
  // round-trip. The user then lands on the new paper page.
  let importInput = $state('');
  let importing = $state(false);
  let importError = $state('');

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  $effect(() => { load(); });
  async function load() {
    loading = true;
    try {
      papers = await listPapers(100, 0);
      document.title = `${t('paper.papers')} — NightBoat`;
    } catch (e: any) {
      error = e?.message || 'Error';
    } finally {
      loading = false;
    }
  }

  async function doImport() {
    const raw = importInput.trim();
    if (!raw) return;
    importing = true;
    importError = '';
    try {
      // Heuristic: looks like a DOI if it starts with 10. or doi.org/;
      // otherwise treat as arxiv id (digits.digits).
      const body = raw.includes('10.') || raw.includes('doi.org')
        ? { doi: raw }
        : { arxiv_id: raw };
      const result = await importPaper(body);
      navigate(`/paper?id=${encodeURIComponent(result.paper.id)}`);
    } catch (e: any) {
      importError = e?.message || 'Import failed';
    } finally {
      importing = false;
    }
  }
</script>

<div class="papers-page">
  <header class="papers-header">
    <h1>{t('paper.papers')}</h1>
    <p class="papers-blurb">
      {t('paper.directoryBlurb')}
    </p>
    {#if getAuth()}
      <form class="paper-import" onsubmit={(e) => { e.preventDefault(); doImport(); }}>
        <input
          type="text"
          bind:value={importInput}
          placeholder={t('paper.importPlaceholder')}
          disabled={importing}
        />
        <button type="submit" disabled={importing || !importInput.trim()}>
          {importing ? t('paper.importing') : t('paper.importBtn')}
        </button>
        {#if importError}<span class="error">{importError}</span>{/if}
      </form>
    {/if}
  </header>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if papers.length === 0}
    <p class="meta">{t('paper.empty')}</p>
  {:else}
    <ul class="paper-grid">
      {#each papers as p}
        <li class="paper-card">
          <a href="/paper?id={encodeURIComponent(p.id)}" class="paper-card-title">{loc(p.title)}</a>
          {#if p.authors.length > 0}
            <div class="paper-card-authors">{p.authors.join(', ')}</div>
          {/if}
          <div class="paper-card-meta">
            {#if p.venue}<span>{p.venue}</span>{/if}
            {#if p.venue && p.year}<span class="dot">·</span>{/if}
            {#if p.year}<span>{p.year}</span>{/if}
            <span class="paper-card-stats">
              {#if p.vote_score > 0}↑{p.vote_score}{/if}
              {#if p.comment_count > 0}<span class="cmt">💬 {p.comment_count}</span>{/if}
            </span>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .papers-page { max-width: 960px; margin: 0 auto; padding: 24px 16px; }
  .papers-header h1 { margin: 0 0 6px 0; }
  .papers-blurb { color: var(--text-secondary); margin: 0 0 12px 0; }
  .paper-import { display: flex; gap: 8px; margin-bottom: 20px; align-items: center; }
  .paper-import input { flex: 1; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; background: var(--surface); color: var(--text-primary); }
  .paper-import button { padding: 8px 14px; border: 1px solid var(--accent); background: var(--accent); color: var(--surface); border-radius: 4px; cursor: pointer; }
  .paper-import button:disabled { opacity: 0.6; cursor: not-allowed; }
  .paper-import .error { color: red; font-size: 13px; }
  .paper-grid { list-style: none; padding: 0; margin: 0; display: grid; gap: 12px; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); }
  .paper-card { padding: 14px; border: 1px solid var(--border); border-radius: 6px; background: var(--surface); display: flex; flex-direction: column; gap: 6px; }
  .paper-card-title { color: var(--text-primary); text-decoration: none; font-weight: 600; line-height: 1.35; }
  .paper-card-title:hover { color: var(--accent); }
  .paper-card-authors { font-size: 13px; color: var(--text-secondary); }
  .paper-card-meta { font-size: 12px; color: var(--text-secondary); display: flex; gap: 6px; align-items: baseline; }
  .paper-card-meta .dot { opacity: 0.6; }
  .paper-card-stats { margin-left: auto; }
  .paper-card-stats .cmt { margin-left: 8px; }
  .meta { color: var(--text-secondary); }
  .error { color: red; }
</style>
