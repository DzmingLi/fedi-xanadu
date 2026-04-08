<script lang="ts">
  import { createSeries, listTags, listArticles, addSeriesArticle, addSeriesPrereq } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { tagName } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Tag, Article, Category } from '../lib/types';

  let title = $state('');
  let description = $state('');
  let longDescription = $state('');
  let tagSearch = $state('');
  let selectedTagId = $state('');
  let allTags = $state<Tag[]>([]);
  let allArticles = $state<Article[]>([]);
  let category = $state<Category>('general');
  let error = $state('');
  let creating = $state(false);

  // Articles to add to the series
  let seriesArticles = $state<{ uri: string; title: string }[]>([]);
  let articleSearch = $state('');

  // Intra-series prereqs: [articleIndex, prereqIndex]
  let seriesPrereqs = $state<[number, number][]>([]);

  let filteredTags = $derived(
    tagSearch ? allTags.filter(t => t.name.toLowerCase().includes(tagSearch.toLowerCase())).slice(0, 10) : []
  );

  let filteredArticles = $derived(
    articleSearch
      ? allArticles
          .filter(a => a.title.toLowerCase().includes(articleSearch.toLowerCase()))
          .filter(a => !seriesArticles.some(sa => sa.uri === a.at_uri))
          .slice(0, 10)
      : []
  );

  $effect(() => {
    Promise.all([listTags(), listArticles()]).then(([tags, arts]) => {
      allTags = tags;
      allArticles = arts;
    });
  });

  function selectTag(tag: Tag) {
    selectedTagId = tag.id;
    tagSearch = tagName(tag.names, tag.name, tag.id);
  }

  function addArticle(a: Article) {
    seriesArticles = [...seriesArticles, { uri: a.at_uri, title: a.title }];
    articleSearch = '';
  }

  function removeArticle(idx: number) {
    seriesArticles = seriesArticles.filter((_, i) => i !== idx);
    seriesPrereqs = seriesPrereqs
      .filter(([a, p]) => a !== idx && p !== idx)
      .map(([a, p]) => [a > idx ? a - 1 : a, p > idx ? p - 1 : p] as [number, number]);
  }

  function moveArticle(idx: number, dir: -1 | 1) {
    const newIdx = idx + dir;
    if (newIdx < 0 || newIdx >= seriesArticles.length) return;
    const arr = [...seriesArticles];
    [arr[idx], arr[newIdx]] = [arr[newIdx], arr[idx]];
    seriesArticles = arr;
    // Update prereq indices
    seriesPrereqs = seriesPrereqs.map(([a, p]) => {
      let na = a === idx ? newIdx : a === newIdx ? idx : a;
      let np = p === idx ? newIdx : p === newIdx ? idx : p;
      return [na, np] as [number, number];
    });
  }

  function addPrereq(articleIdx: number, prereqIdx: number) {
    if (articleIdx === prereqIdx) return;
    if (seriesPrereqs.some(([a, p]) => a === articleIdx && p === prereqIdx)) return;
    seriesPrereqs = [...seriesPrereqs, [articleIdx, prereqIdx]];
  }

  function removePrereq(i: number) {
    seriesPrereqs = seriesPrereqs.filter((_, idx) => idx !== i);
  }

  async function submit() {
    if (!getAuth()) { error = t('auth.submit'); return; }
    if (!title.trim()) { error = t('newSeries.errTitle'); return; }
    if (seriesArticles.length === 0) { error = t('newSeries.errArticles'); return; }

    creating = true;
    error = '';
    try {
      const series = await createSeries({
        title,
        description,
        long_description: longDescription || undefined,
        topics: selectedTagId ? [selectedTagId] : undefined,
        category,
      });

      // Add articles in order
      await Promise.all(
        seriesArticles.map((a, i) => addSeriesArticle(series.id, a.uri, i + 1))
      );

      // Add intra-series prereqs
      await Promise.all(
        seriesPrereqs.map(([aIdx, pIdx]) =>
          addSeriesPrereq(series.id, seriesArticles[aIdx].uri, seriesArticles[pIdx].uri)
        )
      );

      window.location.hash = `#/series?id=${encodeURIComponent(series.id)}`;
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
    <input type="text" bind:value={title} />
  </label>

  <label>
    {t('newArticle.descLabel')}
    <textarea bind:value={description} rows="2" placeholder={t('newSeries.descPlaceholder')}></textarea>
  </label>

  <label>
    {t('newSeries.longDescLabel')}
    <textarea bind:value={longDescription} rows="5" placeholder={t('newSeries.longDescPlaceholder')}></textarea>
  </label>

  <label>
    {t('newArticle.categoryLabel')}
    <select bind:value={category}>
      <option value="general">{t('category.general')}</option>
      <option value="lecture">{t('category.lecture')}</option>
    </select>
  </label>

  <label>
    {t('newSeries.tagLabel')}
    <input type="text" bind:value={tagSearch} placeholder={t('newSeries.tagSearch')} />
    {#if filteredTags.length > 0}
      <div class="dropdown">
        {#each filteredTags as t}
          <button class="dropdown-item" onclick={() => selectTag(t)}>
            {t.name}
          </button>
        {/each}
      </div>
    {/if}
    {#if selectedTagId}
      <span class="selected-tag">{t('newSeries.selectedTag', tagSearch)}</span>
    {/if}
  </label>

  <h2>{t('newSeries.articleList')}</h2>
  <p class="hint">{t('newSeries.articleHint')}</p>

  <div class="article-search">
    <input type="text" bind:value={articleSearch} placeholder={t('newSeries.articleSearch')} />
    {#if filteredArticles.length > 0}
      <div class="dropdown">
        {#each filteredArticles as a}
          <button class="dropdown-item" onclick={() => addArticle(a)}>
            {a.title}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if seriesArticles.length > 0}
    <div class="article-list">
      {#each seriesArticles as sa, i (sa.uri)}
        <div class="article-row">
          <span class="row-num">{i + 1}</span>
          <span class="row-title">{sa.title}</span>
          <div class="row-actions">
            <button onclick={() => moveArticle(i, -1)} disabled={i === 0} title={t('newSeries.moveUp')}>↑</button>
            <button onclick={() => moveArticle(i, 1)} disabled={i === seriesArticles.length - 1} title={t('newSeries.moveDown')}>↓</button>
            <button onclick={() => removeArticle(i)} title={t('common.remove')}>×</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if seriesArticles.length > 1}
    <h3>{t('newSeries.prereqTitle')}</h3>
    <p class="hint">{t('newSeries.prereqHint')}</p>
    <div class="prereq-builder">
      <select id="prereq-article">
        {#each seriesArticles as sa, i}
          <option value={i}>#{i + 1} {sa.title}</option>
        {/each}
      </select>
      <span>{t('newSeries.prereqNeedsReading')}</span>
      <select id="prereq-dep">
        {#each seriesArticles as sa, i}
          <option value={i}>#{i + 1} {sa.title}</option>
        {/each}
      </select>
      <button onclick={() => {
        const aEl = document.getElementById('prereq-article') as HTMLSelectElement;
        const pEl = document.getElementById('prereq-dep') as HTMLSelectElement;
        addPrereq(parseInt(aEl.value), parseInt(pEl.value));
      }}>{t('common.add')}</button>
    </div>
    {#if seriesPrereqs.length > 0}
      <div class="prereq-list">
        {#each seriesPrereqs as [aIdx, pIdx], i}
          <div class="prereq-row">
            #{aIdx + 1} {seriesArticles[aIdx].title} → {t('series.prereqLabel')} #{pIdx + 1} {seriesArticles[pIdx].title}
            <button onclick={() => removePrereq(i)}>×</button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

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
    max-width: 640px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
    color: var(--text-secondary);
    position: relative;
  }
  input, textarea {
    font-family: var(--font-sans);
    font-size: 14px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
  }
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
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
  .dropdown-item:hover {
    background: var(--bg-gray, #f5f5f5);
  }
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
  .hint {
    font-size: 13px;
    color: var(--text-hint);
    margin: 0;
  }
  .article-search {
    position: relative;
  }
  .article-search input {
    width: 100%;
    box-sizing: border-box;
  }
  .article-search .dropdown {
    position: absolute;
    top: 100%;
  }
  .article-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .article-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
  }
  .row-num {
    font-family: var(--font-serif);
    font-size: 14px;
    color: var(--text-hint);
    width: 24px;
    text-align: center;
    flex-shrink: 0;
  }
  .row-title {
    flex: 1;
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .row-actions button {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    cursor: pointer;
    padding: 2px 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .row-actions button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .row-actions button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .prereq-builder {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
  }
  .prereq-builder select {
    padding: 4px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    max-width: 200px;
  }
  .prereq-builder button {
    padding: 4px 12px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .prereq-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
  }
  .prereq-row {
    font-size: 13px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .prereq-row button {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 14px;
  }

  .form-actions {
    margin-top: 16px;
  }
  .submit-btn {
    padding: 10px 24px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .error {
    color: var(--error, #c33);
    font-size: 14px;
  }

  h2, h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 8px 0 4px;
  }
</style>
