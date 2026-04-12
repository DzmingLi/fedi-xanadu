<script lang="ts">
  import { createSeries, listTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { tagName } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Tag, Category } from '../lib/types';

  let title = $state('');
  let description = $state('');
  let tagSearch = $state('');
  let selectedTagId = $state('');
  let allTags = $state<Tag[]>([]);
  let category = $state<Category>('general');
  let lang = $state('zh');
  let error = $state('');
  let creating = $state(false);

  let filteredTags = $derived(
    tagSearch ? allTags.filter(t => t.name.toLowerCase().includes(tagSearch.toLowerCase())).slice(0, 10) : []
  );

  $effect(() => {
    listTags().then(tags => { allTags = tags; });
  });

  function selectTag(tag: Tag) {
    selectedTagId = tag.id;
    tagSearch = tagName(tag.names, tag.name, tag.id);
    filteredTags = [];
  }

  async function submit() {
    if (!getAuth()) { error = t('auth.submit'); return; }
    if (!title.trim()) { error = t('newSeries.errTitle'); return; }
    creating = true;
    error = '';
    try {
      const series = await createSeries({
        title,
        description: description || undefined,
        topics: selectedTagId ? [selectedTagId] : undefined,
        category,
        lang,
      });
      window.location.href = `/series-editor?id=${encodeURIComponent(series.id)}`;
    } catch (e: any) {
      error = e.message || t('newSeries.errCreate');
    }
    creating = false;
  }
</script>

<h1>{t('newSeries.title')}</h1>

{#if error}
  <p class="error">{error}</p>
{/if}

<div class="form">
  <label>
    {t('newSeries.titleLabel')}
    <input type="text" bind:value={title} placeholder={t('newSeries.titlePlaceholder')} />
  </label>

  <label>
    {t('newArticle.descLabel')}
    <textarea bind:value={description} rows="2" placeholder={t('newSeries.descPlaceholder')}></textarea>
  </label>

  <label>
    {t('newArticle.categoryLabel')}
    <select bind:value={category}>
      <option value="general">{t('category.general')}</option>
      <option value="lecture">{t('category.lecture')}</option>
    </select>
  </label>

  <label>
    {t('newArticle.langLabel')}
    <select bind:value={lang}>
      <option value="zh">中文</option>
      <option value="en">English</option>
    </select>
  </label>

  <label>
    {t('newSeries.tagLabel')}
    <div class="tag-wrap">
      <input type="text" bind:value={tagSearch} placeholder={t('newSeries.tagSearch')} />
      {#if filteredTags.length > 0}
        <div class="dropdown">
          {#each filteredTags as tag}
            <button class="dropdown-item" onclick={() => selectTag(tag)}>
              {tag.name}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    {#if selectedTagId}
      <span class="selected-tag">{tagSearch} <button class="clear-btn" onclick={() => { selectedTagId = ''; tagSearch = ''; }}>×</button></span>
    {/if}
  </label>

  <div class="form-actions">
    <button class="submit-btn" onclick={submit} disabled={creating}>
      {creating ? t('newSeries.creating') : t('newSeries.create')}
    </button>
  </div>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 480px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
    color: var(--text-secondary);
  }
  input, textarea, select {
    font-family: var(--font-sans);
    font-size: 14px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
  }
  .tag-wrap {
    position: relative;
  }
  .tag-wrap input { width: 100%; box-sizing: border-box; }
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0; right: 0;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10;
    max-height: 200px;
    overflow-y: auto;
  }
  .dropdown-item {
    display: block;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 14px;
  }
  .dropdown-item:hover { background: var(--bg-gray, #f5f5f5); }
  .selected-tag {
    font-size: 12px;
    color: var(--accent);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .clear-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 14px;
    padding: 0 2px;
  }
  .form-actions { margin-top: 8px; }
  .submit-btn {
    padding: 10px 24px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { color: var(--error, #c33); font-size: 14px; }
</style>
