<script lang="ts">
  import { createQuestion, searchTags } from '../lib/api';
  import { t, onLocaleChange, getLocale } from '../lib/i18n';
  import { getAuth } from '../lib/auth';
  import { toast } from '../lib/components/Toast.svelte';
  import type { Tag } from '../lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let title = $state('');
  let description = $state('');
  let content = $state('');
  let contentFormat = $state('markdown');
  let lang = $state('zh');
  let tags = $state<string[]>([]);
  let tagQuery = $state('');
  let tagResults = $state<Tag[]>([]);
  let publishing = $state(false);

  let tagTimer: ReturnType<typeof setTimeout>;

  function onTagInput() {
    clearTimeout(tagTimer);
    const q = tagQuery.trim();
    if (!q) { tagResults = []; return; }
    tagTimer = setTimeout(async () => {
      try { tagResults = await searchTags(q); } catch { tagResults = []; }
    }, 150);
  }

  function addTag(id: string) {
    if (!tags.includes(id)) tags = [...tags, id];
    tagQuery = '';
    tagResults = [];
  }

  function removeTag(id: string) {
    tags = tags.filter(t => t !== id);
  }

  function addTagOnEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const id = tagQuery.trim().toLowerCase().replace(/\s+/g, '-');
    if (id) addTag(id);
  }

  async function submit() {
    if (!title.trim() || !content.trim()) {
      toast(t('newArticle.fillRequired'), 'error');
      return;
    }
    publishing = true;
    try {
      const q = await createQuestion({
        title: title.trim(),
        description: description.trim() || undefined,
        content: content.trim(),
        content_format: contentFormat,
        lang,
        tags,
        prereqs: [],
      });
      window.location.hash = `#/question?uri=${encodeURIComponent(q.at_uri)}`;
    } catch (e: any) {
      toast(e.message || 'Failed', 'error');
    }
    publishing = false;
  }
</script>

{#if !getAuth()}
  <p class="meta">{t('article.loginToComment')}</p>
{:else}
  <h1 class="page-title">{t('qa.askQuestion')}</h1>

  <div class="form-group">
    <label>{t('newArticle.titleLabel')}</label>
    <input bind:value={title} type="text" placeholder="你的问题是什么？" />
  </div>

  <div class="form-group">
    <label>{t('newArticle.descLabel')}</label>
    <input bind:value={description} type="text" placeholder={t('newArticle.descPlaceholder')} />
  </div>

  <div class="form-row">
    <div class="form-group">
      <label>{t('newArticle.formatLabel')}</label>
      <select bind:value={contentFormat}>
        <option value="markdown">Markdown</option>
        <option value="typst">Typst</option>
        <option value="html">HTML</option>
      </select>
    </div>
    <div class="form-group">
      <label>{t('newArticle.langLabel')}</label>
      <select bind:value={lang}>
        <option value="zh">中文</option>
        <option value="en">English</option>
        <option value="fr">Français</option>
      </select>
    </div>
  </div>

  <div class="form-group">
    <label>{t('newArticle.contentLabel')}</label>
    <textarea bind:value={content} rows="10" placeholder="详细描述你的问题..."></textarea>
  </div>

  <div class="form-group">
    <label>{t('newArticle.tagsLabel')}</label>
    <div class="tag-input-wrap">
      {#each tags as tag}
        <span class="tag-chip">
          {tag}
          <button type="button" onclick={() => removeTag(tag)}>&times;</button>
        </span>
      {/each}
      <input
        bind:value={tagQuery}
        oninput={onTagInput}
        onkeydown={addTagOnEnter}
        placeholder={t('newArticle.tagInput')}
        class="tag-input"
      />
    </div>
    {#if tagResults.length > 0}
      <div class="tag-dropdown">
        {#each tagResults as tag}
          <button type="button" class="tag-option" onclick={() => addTag(tag.id)}>{tag.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="form-actions">
    <button class="btn-publish" onclick={submit} disabled={publishing}>
      {publishing ? t('newArticle.publishing') : t('newArticle.publish')}
    </button>
  </div>
{/if}

<style>
  .page-title {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.5rem;
    margin: 0 0 24px;
  }
  .form-group {
    margin-bottom: 16px;
  }
  .form-group label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }
  .form-group input[type="text"],
  .form-group textarea,
  .form-group select {
    width: 100%;
    padding: 8px 10px;
    font-size: 14px;
    font-family: var(--font-sans);
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .form-group textarea {
    font-family: var(--font-mono, monospace);
    resize: vertical;
  }
  .form-row {
    display: flex;
    gap: 16px;
  }
  .form-row .form-group {
    flex: 1;
  }
  .form-row select {
    width: 100%;
  }

  .tag-input-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
  }
  .tag-chip {
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 12px;
    background: rgba(95, 155, 101, 0.1);
    color: var(--accent);
    padding: 2px 6px;
    border-radius: 3px;
  }
  .tag-chip button {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--accent);
    font-size: 14px;
    padding: 0 2px;
  }
  .tag-input {
    border: none;
    outline: none;
    flex: 1;
    min-width: 120px;
    font-size: 13px;
    background: transparent;
    color: var(--text-primary);
  }
  .tag-dropdown {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-top: 2px;
    max-height: 160px;
    overflow-y: auto;
  }
  .tag-option {
    display: block;
    width: 100%;
    padding: 6px 10px;
    text-align: left;
    border: none;
    background: none;
    font-size: 13px;
    cursor: pointer;
    color: var(--text-primary);
  }
  .tag-option:hover {
    background: var(--bg-hover);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 20px;
  }
  .btn-publish {
    padding: 8px 24px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-publish:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-publish:hover:not(:disabled) { opacity: 0.9; }
</style>
