<script lang="ts">
  import { listTerms } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { TermListItem } from '../lib/types';

  let terms = $state<TermListItem[]>([]);
  let loading = $state(true);

  $effect(() => {
    listTerms().then(c => { terms = c; }).catch(() => {}).finally(() => { loading = false; });
  });

  function formatRating(r: number) {
    return (r / 2).toFixed(1);
  }
</script>

<div class="terms-page">
  <div class="page-header">
    <h1>{t('terms.title')}</h1>
    {#if getAuth()}
      <a href="/new-term" class="create-btn">+ {t('terms.create')}</a>
    {/if}
  </div>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if terms.length === 0}
    <p class="empty">{t('terms.empty')}</p>
  {:else}
    <div class="term-grid">
      {#each terms as term}
        <a href="/term?id={encodeURIComponent(term.id)}" class="term-card">
          <div class="card-top">
            {#if term.code}
              <span class="term-code">{term.code}</span>
            {/if}
            <h2 class="term-title">{term.title}</h2>
          </div>
          {#if term.institution || term.semester}
            <p class="term-meta">
              {#if term.institution}{term.institution}{/if}
              {#if term.institution && term.semester} &middot; {/if}
              {#if term.semester}{term.semester}{/if}
            </p>
          {/if}
          {#if term.description}
            <p class="term-desc">{term.description}</p>
          {/if}
          <div class="card-bottom">
            {#if term.rating_count > 0}
              <span class="stat rating">
                <svg viewBox="0 0 24 24" width="14" height="14"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/></svg>
                {formatRating(term.avg_rating)} ({term.rating_count})
              </span>
            {/if}
            <span class="stat">{term.session_count} {t('terms.sessions')}</span>
            {#if term.author_names && term.author_names.length > 0}
              <span class="author">{term.author_names.join(', ')}</span>
            {/if}
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .terms-page { max-width: 960px; margin: 0 auto; }
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
  .page-header h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.8rem; margin: 0; }
  .create-btn {
    font-size: 13px; padding: 6px 16px; border: 1px solid var(--accent);
    border-radius: 4px; color: var(--accent); text-decoration: none; transition: all 0.15s;
  }
  .create-btn:hover { background: var(--accent); color: white; text-decoration: none; }
  .term-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 16px; }
  .term-card {
    display: block; background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px;
    padding: 20px; text-decoration: none; color: inherit; transition: border-color 0.15s, box-shadow 0.15s;
  }
  .term-card:hover { border-color: var(--accent); box-shadow: 0 2px 8px rgba(0,0,0,0.06); text-decoration: none; }
  .card-top { margin-bottom: 8px; }
  .term-code { font-size: 12px; font-weight: 600; color: var(--accent); background: rgba(95,155,101,0.1); padding: 2px 8px; border-radius: 3px; margin-bottom: 6px; display: inline-block; }
  .term-title { font-family: var(--font-serif); font-size: 1.15rem; margin: 4px 0 0; line-height: 1.35; }
  .term-meta { font-size: 13px; color: var(--text-secondary); margin: 4px 0; }
  .term-desc { font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin: 8px 0; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .card-bottom { display: flex; gap: 12px; align-items: center; margin-top: 12px; font-size: 12px; color: var(--text-hint); }
  .rating { display: flex; align-items: center; gap: 3px; color: #f59e0b; font-weight: 500; }
  .rating svg { display: block; }
  .author { margin-left: auto; }
  .empty { color: var(--text-hint); }
</style>
