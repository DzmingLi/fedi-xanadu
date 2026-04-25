<script lang="ts">
  import { listPapers } from '../lib/api';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import type { PaperListItem } from '../lib/generated/PaperListItem';

  let papers = $state<PaperListItem[]>([]);
  let loading = $state(true);
  let error = $state('');

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
      document.title = `${t('paper.papers') || 'Papers'} — NightBoat`;
    } catch (e: any) {
      error = e?.message || 'Error';
    } finally {
      loading = false;
    }
  }
</script>

<div class="papers-page">
  <header class="papers-header">
    <h1>{t('paper.papers') || 'Papers'}</h1>
    <p class="papers-blurb">
      {t('paper.directoryBlurb') || 'Discussion and notes for academic papers, aggregated across mirrors.'}
    </p>
  </header>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if papers.length === 0}
    <p class="meta">{t('paper.empty') || 'No papers yet.'}</p>
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
  .papers-blurb { color: var(--text-secondary); margin: 0 0 20px 0; }
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
