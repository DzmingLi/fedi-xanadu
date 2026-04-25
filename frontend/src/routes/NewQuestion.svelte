<script lang="ts">
  import { createQuestion, searchTags, lookupTag } from '../lib/api';
  import { t, getLocale, LANG_NAMES } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import { toast } from '../lib/components/Toast.svelte';
  import { tagStore } from '../lib/tagStore.svelte';
  import type { Tag, ContentFormat, PrereqType } from '../lib/types';
  import MarkdownEditor from 'nbt-editor/MarkdownEditor.svelte';

  let locale = $derived(getLocale());
  $effect(() => { tagStore.ensure(); });
  const localTag = (id: string) => tagStore.localize(id);

  let title = $state('');
  let description = $state('');
  let content = $state('');
  let contentFormat = $state<ContentFormat>('markdown');
  let lang = $state('zh');
  let tags = $state<string[]>([]);
  let tagQuery = $state('');
  let tagResults = $state<Tag[]>([]);
  let prereqs = $state<Array<{ tag_id: string; prereq_type: PrereqType }>>([]);
  let prereqQuery = $state('');
  let prereqResults = $state<Tag[]>([]);
  let relatedTags = $state<string[]>([]);
  let relatedQuery = $state('');
  let relatedResults = $state<Tag[]>([]);
  let topicTags = $state<string[]>([]);
  let topicQuery = $state('');
  let topicResults = $state<Tag[]>([]);
  let publishing = $state(false);

  // Translation versions
  interface LangVersion {
    lang: string;
    title: string;
    content: string;
    contentFormat: ContentFormat;
  }
  let extraLangs = $state<LangVersion[]>([]);

  function addLangVersion() {
    const allLangs = Object.keys(LANG_NAMES);
    const usedLangs = new Set([lang, ...extraLangs.map(l => l.lang)]);
    const available = allLangs.filter(l => !usedLangs.has(l));
    if (available.length === 0) return;
    extraLangs = [...extraLangs, { lang: available[0], title: '', content: '', contentFormat }];
  }

  function removeLangVersion(idx: number) {
    extraLangs = extraLangs.filter((_, i) => i !== idx);
  }

  let tagTimer: ReturnType<typeof setTimeout>;
  let prereqTimer: ReturnType<typeof setTimeout>;
  let relatedTimer: ReturnType<typeof setTimeout>;
  let topicTimer: ReturnType<typeof setTimeout>;

  function onTagInput() {
    clearTimeout(tagTimer);
    const q = tagQuery.trim();
    if (!q) { tagResults = []; return; }
    tagTimer = setTimeout(async () => {
      try { tagResults = await searchTags(q); } catch { tagResults = []; }
    }, 150);
  }
  function onPrereqInput() {
    clearTimeout(prereqTimer);
    const q = prereqQuery.trim();
    if (!q) { prereqResults = []; return; }
    prereqTimer = setTimeout(async () => {
      try { prereqResults = await searchTags(q); } catch { prereqResults = []; }
    }, 150);
  }
  function onRelatedInput() {
    clearTimeout(relatedTimer);
    const q = relatedQuery.trim();
    if (!q) { relatedResults = []; return; }
    relatedTimer = setTimeout(async () => {
      try { relatedResults = await searchTags(q); } catch { relatedResults = []; }
    }, 150);
  }
  function onTopicInput() {
    clearTimeout(topicTimer);
    const q = topicQuery.trim();
    if (!q) { topicResults = []; return; }
    topicTimer = setTimeout(async () => {
      try { topicResults = await searchTags(q); } catch { topicResults = []; }
    }, 150);
  }

  // Resolve a free-text input to an existing tag_id. Lookup only — never
  // create from the question form; unknown names surface an error pointing
  // the user to /hierarchy.
  async function resolveToTagId(input: string): Promise<string | null> {
    const s = input.trim();
    if (!s) return null;
    if (s.startsWith('tg-')) return s;
    try { return (await lookupTag(s)).tag_id; }
    catch {
      toast(t('books.tagNotFound').replace('{name}', s), 'error');
      return null;
    }
  }

  function addTag(id: string) {
    if (!tags.includes(id)) tags = [...tags, id];
    tagQuery = '';
    tagResults = [];
  }
  function removeTag(id: string) { tags = tags.filter(t => t !== id); }
  async function addTagOnEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const id = await resolveToTagId(tagQuery);
    if (id) addTag(id);
  }

  function addPrereq(tagId: string, kind: PrereqType = 'required') {
    if (prereqs.some(p => p.tag_id === tagId)) return;
    prereqs = [...prereqs, { tag_id: tagId, prereq_type: kind }];
    prereqQuery = '';
    prereqResults = [];
  }
  function removePrereq(tagId: string) { prereqs = prereqs.filter(p => p.tag_id !== tagId); }
  function togglePrereqType(tagId: string) {
    prereqs = prereqs.map(p => p.tag_id === tagId
      ? { ...p, prereq_type: p.prereq_type === 'required' ? 'recommended' : 'required' }
      : p);
  }
  async function addPrereqOnEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const id = await resolveToTagId(prereqQuery);
    if (id) addPrereq(id);
  }

  function addRelated(tagId: string) {
    if (!relatedTags.includes(tagId)) relatedTags = [...relatedTags, tagId];
    relatedQuery = '';
    relatedResults = [];
  }
  function removeRelated(tagId: string) { relatedTags = relatedTags.filter(t => t !== tagId); }
  async function addRelatedOnEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const id = await resolveToTagId(relatedQuery);
    if (id) addRelated(id);
  }

  function addTopic(tagId: string) {
    if (!topicTags.includes(tagId)) topicTags = [...topicTags, tagId];
    topicQuery = '';
    topicResults = [];
  }
  function removeTopic(tagId: string) { topicTags = topicTags.filter(t => t !== tagId); }
  async function addTopicOnEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const id = await resolveToTagId(topicQuery);
    if (id) addTopic(id);
  }

  async function submit() {
    if (!title.trim()) {
      toast(t('newArticle.fillRequired'), 'error');
      return;
    }
    publishing = true;
    try {
      const q = await createQuestion({
        title: title.trim(),
        summary: description.trim() || undefined,
        content: content.trim(),
        content_format: contentFormat,
        lang,
        tags,
        prereqs,
        related: relatedTags,
        topics: topicTags,
      });

      // Create translation versions
      for (const lv of extraLangs) {
        if (!lv.title.trim()) continue;
        try {
          await createQuestion({
            title: lv.title.trim(),
            content: lv.content.trim(),
            content_format: lv.contentFormat,
            lang: lv.lang,
            translation_of: q.at_uri,
            tags,
            prereqs,
            related: relatedTags,
            topics: topicTags,
          });
        } catch (e: any) {
          console.warn(`Failed to create ${lv.lang} translation:`, e);
        }
      }

      window.location.href = `/question?uri=${encodeURIComponent(q.at_uri)}`;
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
    <label>{t('newArticle.titleLabel')} *</label>
    <input bind:value={title} type="text" placeholder={t('qa.titlePlaceholder')} />
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
        {#each Object.entries(LANG_NAMES) as [code, name]}
          <option value={code}>{name}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="form-group">
    <label>{t('newArticle.contentLabel')}</label>
    {#if contentFormat === 'markdown'}
      <MarkdownEditor bind:value={content} placeholder={t('qa.contentPlaceholder')} />
    {:else}
      <textarea bind:value={content} rows="10" placeholder={t('qa.contentPlaceholder')}></textarea>
    {/if}
  </div>

  <div class="form-group">
    <label>{t('newArticle.tagsLabel')}</label>
    <div class="tag-input-wrap">
      {#each tags as tag}
        <span class="tag-chip">
          {localTag(tag)}
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
          <button type="button" class="tag-option" onclick={() => addTag(tag.tag_id)}>{tag.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="form-group">
    <label>{t('books.topicsLabel')}</label>
    <p class="hint">{t('books.topicsHint')}</p>
    <div class="tag-input-wrap">
      {#each topicTags as tag}
        <span class="tag-chip">
          {localTag(tag)}
          <button type="button" onclick={() => removeTopic(tag)}>&times;</button>
        </span>
      {/each}
      <input
        bind:value={topicQuery}
        oninput={onTopicInput}
        onkeydown={addTopicOnEnter}
        placeholder={t('newArticle.tagInput')}
        class="tag-input"
      />
    </div>
    {#if topicResults.length > 0}
      <div class="tag-dropdown">
        {#each topicResults as tag}
          <button type="button" class="tag-option" onclick={() => addTopic(tag.tag_id)}>{tag.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="form-group">
    <label>{t('newArticle.prereqsLabel')}</label>
    <p class="hint">{t('newArticle.prereqsHint')}</p>
    <div class="tag-input-wrap">
      {#each prereqs as p (p.tag_id)}
        <span class="tag-chip prereq {p.prereq_type === 'recommended' ? 'recommended' : ''}">
          {localTag(p.tag_id)}
          <button type="button" class="chip-toggle"
            title={p.prereq_type === 'required' ? t('books.prereqClickRecommended') : t('books.prereqClickRequired')}
            onclick={() => togglePrereqType(p.tag_id)}>
            {p.prereq_type === 'required' ? t('newArticle.required') : t('newArticle.recommended')}
          </button>
          <button type="button" onclick={() => removePrereq(p.tag_id)}>&times;</button>
        </span>
      {/each}
      <input
        bind:value={prereqQuery}
        oninput={onPrereqInput}
        onkeydown={addPrereqOnEnter}
        placeholder={t('newArticle.tagInput')}
        class="tag-input"
      />
    </div>
    {#if prereqResults.length > 0}
      <div class="tag-dropdown">
        {#each prereqResults as tag}
          <button type="button" class="tag-option" onclick={() => addPrereq(tag.tag_id)}>{tag.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="form-group">
    <label>{t('newArticle.relatedLabel')}</label>
    <p class="hint">{t('newArticle.relatedHint')}</p>
    <div class="tag-input-wrap">
      {#each relatedTags as tag}
        <span class="tag-chip">
          {localTag(tag)}
          <button type="button" onclick={() => removeRelated(tag)}>&times;</button>
        </span>
      {/each}
      <input
        bind:value={relatedQuery}
        oninput={onRelatedInput}
        onkeydown={addRelatedOnEnter}
        placeholder={t('newArticle.tagInput')}
        class="tag-input"
      />
    </div>
    {#if relatedResults.length > 0}
      <div class="tag-dropdown">
        {#each relatedResults as tag}
          <button type="button" class="tag-option" onclick={() => addRelated(tag.tag_id)}>{tag.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Translation versions -->
  <div class="form-group">
    <div class="lang-header">
      <span class="form-label">{t('qa.translations')}</span>
      <button type="button" class="btn-add-lang" onclick={addLangVersion}>
        + {t('newArticle.addLangVersion')}
      </button>
    </div>
    {#each extraLangs as lv, idx}
      <div class="lang-row">
        <select bind:value={extraLangs[idx].lang}>
          {#each Object.entries(LANG_NAMES) as [code, name]}
            <option value={code} disabled={code === lang || extraLangs.some((l, i) => i !== idx && l.lang === code)}>{name}</option>
          {/each}
        </select>
        <input
          bind:value={extraLangs[idx].title}
          type="text"
          placeholder={t('qa.translationTitle')}
          class="lang-title-input"
        />
        <select bind:value={extraLangs[idx].contentFormat}>
          <option value="markdown">Markdown</option>
          <option value="typst">Typst</option>
          <option value="html">HTML</option>
        </select>
        <button type="button" class="lang-remove" onclick={() => removeLangVersion(idx)}>&times;</button>
      </div>
      <textarea
        bind:value={extraLangs[idx].content}
        rows="4"
        placeholder={t('qa.contentPlaceholder')}
        class="lang-textarea"
      ></textarea>
    {/each}
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

  /* Translation section */
  .lang-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .form-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .btn-add-lang {
    font-size: 12px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 8px;
    cursor: pointer;
    color: var(--text-secondary);
  }
  .btn-add-lang:hover { background: var(--bg-hover); }
  .lang-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 6px;
  }
  .lang-row select { width: auto; padding: 4px 8px; font-size: 12px; }
  .lang-title-input {
    flex: 1;
    padding: 4px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .lang-remove {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: var(--text-hint);
    padding: 0 4px;
  }
  .lang-textarea {
    width: 100%;
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
    resize: vertical;
    margin-bottom: 12px;
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
