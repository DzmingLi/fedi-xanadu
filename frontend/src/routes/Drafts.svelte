<script lang="ts">
  import { listDrafts, deleteDraft, publishDraft } from '../lib/api';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import type { Draft } from '../lib/types';

  let locale = $derived(getLocale());

  let drafts = $state<Draft[]>([]);
  let loading = $state(true);
  let error = $state('');

  $effect(() => {
    listDrafts()
      .then(data => { drafts = data; })
      .catch(e => { error = e.message; })
      .finally(() => { loading = false; });
  });

  async function doPublish(draft: Draft) {
    error = '';
    try {
      const article = await publishDraft(draft.id);
      drafts = drafts.filter(d => d.id !== draft.id);
      window.location.href = `/article?uri=${encodeURIComponent(article.at_uri)}`;
    } catch (e: any) {
      error = e.message;
    }
  }

  async function doDelete(id: string) {
    if (!confirm(t('drafts.deleteConfirm'))) return;
    error = '';
    try {
      await deleteDraft(id);
      drafts = drafts.filter(d => d.id !== id);
    } catch (e: any) {
      error = e.message;
    }
  }

  function editDraft(id: string) {
    window.location.href = `/new?draft=${encodeURIComponent(id)}`;
  }

  function formatDate(s: string) {
    return s.replace('T', ' ').slice(0, 16);
  }
</script>

<h1>{t('drafts.title')}</h1>

{#if error}
  <p class="error-msg">{error}</p>
{/if}

{#if loading}
  <p class="meta">{t('drafts.loading')}</p>
{:else if drafts.length === 0}
  <p class="meta">{t('drafts.empty')}</p>
{:else}
  <div class="draft-list">
    {#each drafts as draft}
      <div class="draft-card">
        <div class="draft-header">
          <button class="draft-title" onclick={() => editDraft(draft.id)}>
            {draft.title || t('drafts.untitled')}
          </button>
          <span class="draft-format">{draft.content_format === 'markdown' ? 'MD' : draft.content_format === 'html' ? 'HTML' : 'Typst'}</span>
        </div>
        {#if draft.description}
          <p class="draft-desc">{draft.description}</p>
        {/if}
        <div class="draft-meta">
          <span>{formatDate(draft.updated_at)}</span>
          <span>{draft.lang}</span>
        </div>
        <div class="draft-actions">
          <button class="btn-edit" onclick={() => editDraft(draft.id)}>
            {t('drafts.edit')}
          </button>
          <button class="btn-publish" onclick={() => doPublish(draft)}>
            {t('drafts.publish')}
          </button>
          <button class="btn-delete" onclick={() => doDelete(draft.id)}>
            {t('drafts.delete')}
          </button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .error-msg { color: #dc2626; margin-bottom: 1rem; }
  .draft-list { display: flex; flex-direction: column; gap: 12px; }
  .draft-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 14px 18px;
    background: var(--bg-white);
    transition: box-shadow 0.15s;
  }
  .draft-card:hover { box-shadow: 0 2px 8px rgba(0,0,0,0.06); }
  .draft-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .draft-title {
    font-family: var(--font-serif);
    font-size: 16px;
    font-weight: 500;
    margin: 0;
    cursor: pointer;
    flex: 1;
  }
  .draft-title:hover { color: var(--accent); }
  .draft-format {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--bg-hover, #f0f0f0);
    color: var(--text-secondary);
    font-family: var(--font-mono, monospace);
  }
  .draft-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 2px 0 6px;
    line-height: 1.4;
  }
  .draft-meta {
    font-size: 12px;
    color: var(--text-hint);
    display: flex;
    gap: 12px;
    margin-bottom: 8px;
  }
  .draft-actions { display: flex; gap: 6px; }
  .draft-actions button {
    font-size: 12px;
    padding: 4px 12px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-white);
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-edit:hover { border-color: var(--accent); color: var(--accent); }
  .btn-publish {
    background: var(--accent) !important;
    color: white !important;
    border-color: var(--accent) !important;
  }
  .btn-publish:hover { opacity: 0.85; }
  .btn-delete:hover { border-color: #dc2626; color: #dc2626; }
</style>
