<script lang="ts">
  import { listDrafts, deleteDraft, publishDraft } from '../lib/api';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import type { Draft } from '../lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

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
      window.location.hash = `#/article?uri=${encodeURIComponent(article.at_uri)}`;
    } catch (e: any) {
      error = e.message;
    }
  }

  async function doDelete(id: string) {
    if (!confirm(locale === 'zh' ? '确定删除此草稿？' : 'Delete this draft?')) return;
    error = '';
    try {
      await deleteDraft(id);
      drafts = drafts.filter(d => d.id !== id);
    } catch (e: any) {
      error = e.message;
    }
  }

  function editDraft(id: string) {
    window.location.hash = `#/new?draft=${encodeURIComponent(id)}`;
  }

  function formatDate(s: string) {
    return s.replace('T', ' ').slice(0, 16);
  }
</script>

<h1>{locale === 'zh' ? '草稿箱' : 'Drafts'}</h1>

{#if error}
  <p class="error-msg">{error}</p>
{/if}

{#if loading}
  <p class="meta">{locale === 'zh' ? '加载中...' : 'Loading...'}</p>
{:else if drafts.length === 0}
  <p class="meta">{locale === 'zh' ? '暂无草稿' : 'No drafts yet'}</p>
{:else}
  <div class="draft-list">
    {#each drafts as draft}
      <div class="draft-card">
        <div class="draft-header">
          <h3 class="draft-title" onclick={() => editDraft(draft.id)}>
            {draft.title || (locale === 'zh' ? '无标题' : 'Untitled')}
          </h3>
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
            {locale === 'zh' ? '编辑' : 'Edit'}
          </button>
          <button class="btn-publish" onclick={() => doPublish(draft)}>
            {locale === 'zh' ? '发布' : 'Publish'}
          </button>
          <button class="btn-delete" onclick={() => doDelete(draft.id)}>
            {locale === 'zh' ? '删除' : 'Delete'}
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
