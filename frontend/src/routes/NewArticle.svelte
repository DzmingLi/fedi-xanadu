<script lang="ts">
  import { listTags, searchTags, lookupTag, createArticle, listArticles, getArticle, getArticleContent, convertContent, uploadImage, updateArticle, saveDraft, updateDraft as apiUpdateDraft, listDrafts, getBook, listArticleCollaborators, inviteArticleCollaborator, removeArticleCollaborator } from '../lib/api';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import { getLangPrefs } from '../lib/langPrefs.svelte';
  import MarkdownEditor from 'nbt-editor/MarkdownEditor.svelte';
  import TypstEditor from 'nbt-editor/TypstEditor.svelte';
  import type { Tag, Article, BookEdition, ContentFormat, PrereqType } from '../lib/types';

  let { editUri = '', draftId: initialDraftId = '', initialCategory = '', initialBookId = '' } = $props();
  let isEditing = $state(false);
  // svelte-ignore state_referenced_locally
  let currentDraftId = $state(initialDraftId);
  let savingDraft = $state(false);
  let draftSaved = $state(false);
  let autoSaveTimer: ReturnType<typeof setTimeout> | undefined;

  // Auto-save draft 2 s after any content change (new articles only, not edits of published ones)
  $effect(() => {
    // Touch reactive state to subscribe
    const _t = title, _d = summary, _c = content, _f = contentFormat, _l = lang;
    if (isEditing || !_t.trim()) return;
    clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      if (!savingDraft) handleSaveDraft();
    }, 2000);
    return () => clearTimeout(autoSaveTimer);
  });

  let tags = $state<Tag[]>([]);
  let allArticles = $state<Article[]>([]);
  let title = $state('');
  let summary = $state('');
  let content = $state('');
  let contentFormat = $state<ContentFormat>(getLangPrefs()?.default_format || 'markdown');
  let lang = $state(getLangPrefs()?.native_lang || getLocale());
  let license = $state('CC-BY-SA-4.0');
  let restricted = $state(false);
  let translationOf = $state('');
  let commitMessage = $state('');
  // Props seed the initial value only; after mount the user may edit freely,
  // so we intentionally don't wrap these in $derived.
  // svelte-ignore state_referenced_locally
  let category = $state(initialCategory || 'general');
  // svelte-ignore state_referenced_locally
  let bookId = $state(initialBookId || '');
  let editionId = $state('');
  let bookEditions = $state<BookEdition[]>([]);
  let selectedTags = $state<string[]>([]);
  let prereqs = $state<Array<{ tag_id: string; prereq_type: PrereqType }>>([]);
  let relatedTags = $state<string[]>([]);
  let submitting = $state(false);
  let error = $state('');
  let uploadingImage = $state(false);
  let savedArticleUri = $state(''); // Set after first save, needed for image upload
  let loadingFile = $state(false);
  let converting = $state(false);

  // --- UI state ---
  let sidebarOpen = $state(true);

  async function handleFormatChange(newFormat: ContentFormat) {
    contentFormat = newFormat;
  }

  // Multi-language versions
  interface LangVersion {
    lang: string;
    content: string;
    contentFormat: ContentFormat;
  }
  let extraLangs = $state<LangVersion[]>([]);

  function addLangVersion() {
    const usedLangs = new Set([lang, ...extraLangs.map(l => l.lang)]);
    const available = ['en', 'zh', 'ja', 'ko', 'fr', 'de'].filter(l => !usedLangs.has(l));
    if (available.length === 0) return;
    extraLangs = [...extraLangs, { lang: available[0], content: '', contentFormat }];
  }

  function removeLangVersion(idx: number) {
    extraLangs = extraLangs.filter((_, i) => i !== idx);
  }

  async function handleLangFileLoad(idx: number, e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const name = file.name.toLowerCase();
      let fmt = extraLangs[idx].contentFormat;
      if (name.endsWith('.md') || name.endsWith('.markdown')) fmt = 'markdown';
      else if (name.endsWith('.typ') || name.endsWith('.typst')) fmt = 'typst';
      else if (name.endsWith('.html') || name.endsWith('.htm')) fmt = 'html';
      extraLangs[idx] = { ...extraLangs[idx], content: text, contentFormat: fmt };
    } catch (err: any) {
      error = err.message;
    }
    input.value = '';
  }

  // Tag / related / prereq inputs — chip-row + search input with autocomplete,
  // mirroring the book editor. Lookup-only: unknown names surface an error
  // nudging the user to the hierarchy page to mint the concept first.
  let teachInput = $state('');
  let teachSuggestions = $state<Tag[]>([]);
  let teachTimeout: ReturnType<typeof setTimeout> | undefined;

  let relatedInput = $state('');
  let relatedSuggestions = $state<Tag[]>([]);
  let relatedTimeout: ReturnType<typeof setTimeout> | undefined;

  let prereqInput = $state('');
  let prereqSuggestions = $state<Tag[]>([]);
  let prereqTimeout: ReturnType<typeof setTimeout> | undefined;
  let tagError = $state('');

  $effect(() => {
    const q = teachInput.trim();
    clearTimeout(teachTimeout);
    if (!q) { teachSuggestions = []; return; }
    teachTimeout = setTimeout(() => {
      searchTags(q).then(results => { teachSuggestions = results; }).catch(() => { teachSuggestions = []; });
    }, 150);
  });

  $effect(() => {
    const q = relatedInput.trim();
    clearTimeout(relatedTimeout);
    if (!q) { relatedSuggestions = []; return; }
    relatedTimeout = setTimeout(() => {
      searchTags(q).then(results => { relatedSuggestions = results; }).catch(() => { relatedSuggestions = []; });
    }, 150);
  });

  $effect(() => {
    const q = prereqInput.trim();
    clearTimeout(prereqTimeout);
    if (!q) { prereqSuggestions = []; return; }
    prereqTimeout = setTimeout(() => {
      searchTags(q).then(results => { prereqSuggestions = results; }).catch(() => { prereqSuggestions = []; });
    }, 150);
  });

  async function appendArticleTag(slot: 'teaches' | 'related' | 'prereqs', input: string) {
    const s = input.trim();
    if (!s) return;
    let tagId: string;
    if (s.startsWith('tg-')) {
      tagId = s;
    } else {
      try {
        const res = await lookupTag(s);
        tagId = res.tag_id;
      } catch {
        tagError = t('newArticle.tagNotFound').replace('{name}', s);
        return;
      }
    }
    tagError = '';
    if (slot === 'teaches') {
      if (!selectedTags.includes(tagId)) selectedTags = [...selectedTags, tagId];
      teachInput = ''; teachSuggestions = [];
    } else if (slot === 'related') {
      if (!relatedTags.includes(tagId)) relatedTags = [...relatedTags, tagId];
      relatedInput = ''; relatedSuggestions = [];
    } else {
      if (!prereqs.some(p => p.tag_id === tagId)) {
        prereqs = [...prereqs, { tag_id: tagId, prereq_type: 'required' }];
      }
      prereqInput = ''; prereqSuggestions = [];
    }
  }

  function togglePrereqType(tagId: string) {
    prereqs = prereqs.map(p =>
      p.tag_id === tagId
        ? { ...p, prereq_type: p.prereq_type === 'required' ? 'recommended' : 'required' }
        : p,
    );
  }

  $effect(() => {
    listTags().then(data => { tags = data; });
    listArticles().then(data => { allArticles = data; });
    if (bookId) {
      getBook(bookId).then(d => { bookEditions = d.editions; }).catch(() => {});
    }
    if (editUri) {
      isEditing = true;
      savedArticleUri = editUri;
      Promise.all([getArticle(editUri), getArticleContent(editUri)]).then(([a, c]) => {
        title = a.title;
        summary = a.summary || '';
        content = c.source;
        contentFormat = a.content_format;
        lang = a.lang || 'zh';
        license = a.license || 'CC-BY-SA-4.0';
      }).catch(err => {
        console.error('Failed to load article for edit:', err);
        error = t('newArticle.loadFailed').replace('{err}', err?.message ?? String(err));
      });
    } else if (initialDraftId) {
      listDrafts().then(drafts => {
        const d = drafts.find(d => d.id === initialDraftId);
        if (!d) return;
        title = d.title;
        summary = d.summary || '';
        content = d.content;
        contentFormat = d.content_format;
        lang = d.lang || 'zh';
        license = d.license || 'CC-BY-SA-4.0';
        try { selectedTags = JSON.parse(d.tags); } catch { selectedTags = []; }
        try { prereqs = JSON.parse(d.prereqs); } catch { prereqs = []; }
      });
    }
  });

  function toggleTag(id: string) {
    if (selectedTags.includes(id)) {
      selectedTags = selectedTags.filter(t => t !== id);
    } else {
      selectedTags = [...selectedTags, id];
    }
  }

  function removePrereq(tagId: string) {
    prereqs = prereqs.filter(p => p.tag_id !== tagId);
  }

  function toggleRelated(id: string) {
    if (relatedTags.includes(id)) relatedTags = relatedTags.filter(t => t !== id);
    else relatedTags = [...relatedTags, id];
  }

  function getTagName(id: string): string {
    return tags.find(t => t.id === id)?.name ?? id;
  }

  async function handleImageUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    if (!savedArticleUri) {
      if (!title.trim() || !content.trim()) {
        error = t('newArticle.fillTitleContent');
        return;
      }
      uploadingImage = true;
      error = '';
      try {
        const article = await createArticle({
          title: title.trim(),
          summary: summary.trim() || undefined,
          content: content.trim(),
          content_format: contentFormat,
          lang: lang || getLocale(),
          license: restricted ? 'All-Rights-Reserved' : (license || undefined),
          translation_of: translationOf || undefined,
          restricted: restricted || undefined,
          category: category || undefined,
          book_id: bookId || undefined,
          edition_id: editionId || undefined,
          tags: selectedTags,
          prereqs,
          related: relatedTags,
        });
        savedArticleUri = article.at_uri;
        isEditing = true;
      } catch (err: any) {
        error = err.message;
        uploadingImage = false;
        return;
      }
    }

    uploadingImage = true;
    error = '';
    try {
      const result = await uploadImage(savedArticleUri, file);
      const ref = contentFormat === 'markdown'
        ? `![${result.filename}](${result.filename})`
        : `#image("${result.filename}")`;
      content += '\n' + ref + '\n';
    } catch (err: any) {
      error = err.message;
    } finally {
      uploadingImage = false;
      input.value = '';
    }
  }

  // Image upload callback for editor component (drag/paste/toolbar)
  async function editorImageUpload(file: File): Promise<{ src: string; alt?: string }> {
    if (!savedArticleUri) {
      if (!title.trim() || !content.trim()) throw new Error(t('newArticle.fillTitleContent'));
      const article = await createArticle({
        title: title.trim(),
        summary: summary.trim() || undefined,
        content: content.trim(),
        content_format: contentFormat,
        lang: lang || getLocale(),
        license: restricted ? 'All-Rights-Reserved' : (license || undefined),
        translation_of: translationOf || undefined,
        restricted: restricted || undefined,
        category: category || undefined,
        book_id: bookId || undefined,
        edition_id: editionId || undefined,
        tags: selectedTags,
        prereqs,
        related: relatedTags,
      });
      savedArticleUri = article.at_uri;
      isEditing = true;
    }
    const result = await uploadImage(savedArticleUri, file);
    return { src: result.filename, alt: result.filename };
  }

  async function handleFileLoad(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    loadingFile = true;
    error = '';
    try {
      const text = await file.text();
      content = text;
      const name = file.name.toLowerCase();
      if (name.endsWith('.md') || name.endsWith('.markdown')) {
        contentFormat = 'markdown';
      } else if (name.endsWith('.typ') || name.endsWith('.typst')) {
        contentFormat = 'typst';
      } else if (name.endsWith('.html') || name.endsWith('.htm')) {
        contentFormat = 'html';
      }
      if (!title.trim()) {
        title = file.name.replace(/\.(md|markdown|typ|typst|html|htm)$/i, '');
      }
    } catch (err: any) {
      error = err.message;
    } finally {
      loadingFile = false;
      input.value = '';
    }
  }

  async function handleSaveDraft() {
    savingDraft = true;
    draftSaved = false;
    error = '';
    try {
      const data = {
        title: title.trim() || t('newArticle.untitledDraft'),
        summary: summary.trim() || undefined,
        content: content,
        content_format: contentFormat,
        lang: lang || getLocale(),
        license: license || undefined,
        tags: selectedTags,
        prereqs,
        related: relatedTags,
      };
      if (currentDraftId) {
        await apiUpdateDraft(currentDraftId, data);
      } else {
        const draft = await saveDraft(data);
        currentDraftId = draft.id;
      }
      draftSaved = true;
      setTimeout(() => { draftSaved = false; }, 2000);
    } catch (e: any) {
      error = e.message;
    } finally {
      savingDraft = false;
    }
  }

  async function submit() {
    if (!title.trim() || !content.trim()) return;
    submitting = true;
    error = '';
    try {
      if (isEditing && editUri) {
        const article = await updateArticle(editUri, {
          title: title.trim(),
          summary: summary.trim(),
          content: content.trim(),
          commit_message: commitMessage.trim() || undefined,
        });
        window.location.href = `/article?uri=${encodeURIComponent(article.at_uri)}`;
      } else {
        const article = await createArticle({
          title: title.trim(),
          summary: summary.trim() || undefined,
          content: content.trim(),
          content_format: contentFormat,
          lang: lang || getLocale(),
          license: restricted ? 'All-Rights-Reserved' : (license || undefined),
          translation_of: translationOf || undefined,
          restricted: restricted || undefined,
          category: category || undefined,
          book_id: bookId || undefined,
          edition_id: editionId || undefined,
          tags: selectedTags,
          prereqs,
          related: relatedTags,
        });

        for (const lv of extraLangs) {
          if (!lv.content.trim()) continue;
          try {
            await createArticle({
              title: title.trim(),
              summary: summary.trim() || undefined,
              content: lv.content.trim(),
              content_format: lv.contentFormat,
              lang: lv.lang,
              license: license || undefined,
              translation_of: article.at_uri,
              category: category || undefined,
              book_id: bookId || undefined,
              tags: selectedTags,
              prereqs,
              related: relatedTags,
            });
          } catch (e: any) {
            console.warn(`Failed to create ${lv.lang} translation:`, e);
          }
        }

        window.location.href = `/article?uri=${encodeURIComponent(article.at_uri)}`;
      }
    } catch (e: any) {
      error = e.message;
    } finally {
      submitting = false;
    }
  }


</script>

<div class="editor-page">
  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  <!-- Title area -->
    <div class="editor-title-area">
      <input
        class="title-input"
        bind:value={title}
        placeholder={t('newArticle.titleLabel')}
      />
      <input
        class="desc-input"
        bind:value={summary}
        placeholder={t('newArticle.summaryPlaceholder')}
      />
    </div>

    <!-- Main body: editor + settings sidebar -->
    <div class="editor-body">
      <!-- Center: Editor -->
      <div class="editor-main">
        <!-- Editor content area -->
        <div class="editor-content">
          {#if contentFormat === 'markdown'}
            <MarkdownEditor bind:value={content} placeholder="# 我的文章&#10;&#10;正文..." fillHeight={true} onImageUpload={editorImageUpload} />
          {:else if contentFormat === 'typst'}
            <TypstEditor bind:value={content} placeholder="= 我的文章&#10;&#10;正文..." fillHeight={true} onImageUpload={editorImageUpload} />
          {:else}
            <textarea class="editor-textarea" bind:value={content} placeholder="<!DOCTYPE html>..."></textarea>
          {/if}
        </div>

        <!-- Sidebar toggle tab on right edge -->
        <button
          class="sidebar-tab"
          class:open={sidebarOpen}
          onclick={() => sidebarOpen = !sidebarOpen}
          title={t('editor.settings')}
          aria-label={t('editor.settings')}
        >
          <svg width="12" height="22" viewBox="0 0 10 18" fill="currentColor">
            {#if sidebarOpen}
              <polyline points="2,2 8,9 2,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {:else}
              <polyline points="8,2 2,9 8,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {/if}
          </svg>
        </button>
      </div>

      <!-- Right: Settings Sidebar -->
      {#if sidebarOpen}
        <aside class="settings-sidebar">
          <div class="sb-field" style="padding: 8px 12px; border-bottom: 1px solid var(--border);">
            <label>
              {t('newArticle.formatLabel')}
              <select value={contentFormat} onchange={(e) => handleFormatChange((e.target as HTMLSelectElement).value as ContentFormat)} disabled={converting}>
                <option value="markdown">Markdown</option>
                <option value="typst">Typst</option>
                {#if contentFormat === 'html'}<option value="html">HTML (只读)</option>{/if}
              </select>
            </label>
            {#if converting}<span class="converting-hint">{t('newArticle.converting')}</span>{/if}
          </div>
          <div class="sb-uploads">
            <label class="sb-upload-btn" class:disabled={loadingFile}>
              <input type="file" accept=".md,.markdown,.typ,.typst" onchange={handleFileLoad} hidden />
              {loadingFile ? t('newArticle.readingFile') : t('newArticle.uploadFile')}
            </label>
            <label class="sb-upload-btn" class:disabled={uploadingImage}>
              <input type="file" accept="image/*" onchange={handleImageUpload} hidden />
              {uploadingImage ? t('newArticle.uploading') : t('newArticle.uploadImage')}
            </label>
          </div>
          <details open>
            <summary>{t('editor.basicInfo')}</summary>
            <div class="sb-field">
              <label>
                {t('newArticle.langLabel')}
                <select bind:value={lang}>
                  <option value="zh">中文</option>
                  <option value="en">English</option>
                  <option value="ja">日本語</option>
                  <option value="ko">한국어</option>
                  <option value="fr">Français</option>
                  <option value="de">Deutsch</option>
                </select>
              </label>
            </div>
            <div class="sb-field">
              <label class="check-label">
                <input type="checkbox" bind:checked={restricted} />
                {t('newArticle.restricted')}
              </label>
            </div>
            {#if !restricted}
            <div class="sb-field">
              <label>
                {t('newArticle.licenseLabel')}
                <select bind:value={license}>
                  <option value="CC-BY-SA-4.0">CC BY-SA 4.0</option>
                  <option value="CC-BY-NC-SA-4.0">CC BY-NC-SA 4.0</option>
                  <option value="CC-BY-4.0">CC BY 4.0</option>
                  <option value="CC-BY-NC-4.0">CC BY-NC 4.0</option>
                  <option value="CC-BY-NC-ND-4.0">CC BY-NC-ND 4.0</option>
                  <option value="CC0-1.0">CC0</option>
                  <option value="MIT">MIT</option>
                  <option value="Apache-2.0">Apache 2.0</option>
                  <option value="GFDL-1.3">GFDL 1.3</option>
                  <option value="All-Rights-Reserved">All Rights Reserved</option>
                </select>
              </label>
            </div>
            {/if}
            <div class="sb-field">
              <label>
                {t('newArticle.categoryLabel')}
                <input
                  bind:value={category}
                  list="category-suggestions"
                  placeholder={t('category.general')}
                  class="category-input"
                />
              </label>
              <datalist id="category-suggestions">
                <option value="general">{t('category.general')}</option>
                <option value="lecture">{t('category.lecture')}</option>
                <option value="paper">{t('category.paper')}</option>
                <option value="review">{t('category.review')}</option>
              </datalist>
            </div>
            {#if category === 'review' && bookEditions.length > 0}
              <div class="sb-field">
                <label>
                  {t('newArticle.editionLabel')}
                  <select bind:value={editionId}>
                    <option value="">{t('newArticle.noEdition')}</option>
                    {#each bookEditions as ed}
                      <option value={ed.id}>{ed.title} ({ed.lang}{ed.year ? `, ${ed.year}` : ''})</option>
                    {/each}
                  </select>
                </label>
              </div>
            {/if}
          </details>

          <details>
            <summary>{t('newArticle.translationOf')}</summary>
            <div class="sb-field">
              <select bind:value={translationOf}>
                <option value="">{t('newArticle.originalArticle')}</option>
                {#each allArticles as a}
                  <option value={a.at_uri}>[{a.lang}] {a.title}</option>
                {/each}
              </select>
            </div>
          </details>

          <details open>
            <summary>{t('newArticle.tagsLabel')}</summary>
            <div class="sb-field">
              {#if selectedTags.length > 0}
                <div class="selected-tags">
                  {#each selectedTags as tagId}
                    <span class="tag lit">{getTagName(tagId)} <button class="tag-remove" onclick={() => toggleTag(tagId)}>&times;</button></span>
                  {/each}
                </div>
              {/if}
              <div class="tag-input-wrap">
                <input
                  class="tag-input"
                  type="text"
                  bind:value={teachInput}
                  placeholder={t('newArticle.tagInput')}
                  onkeydown={(e) => { if (e.key === 'Enter' && teachInput.trim()) { e.preventDefault(); appendArticleTag('teaches', teachInput); } }}
                />
                {#if teachSuggestions.length > 0}
                  <ul class="tag-suggestions">
                    {#each teachSuggestions as s}
                      <li><button type="button" onclick={() => appendArticleTag('teaches', s.id)}>{s.name || s.id}</button></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
          </details>

          <details>
            <summary>{t('newArticle.relatedLabel')}</summary>
            <div class="sb-field">
              <p class="sb-hint">{t('newArticle.relatedHint')}</p>
              {#if relatedTags.length > 0}
                <div class="selected-tags">
                  {#each relatedTags as tagId}
                    <span class="tag lit related">{getTagName(tagId)} <button class="tag-remove" onclick={() => toggleRelated(tagId)}>&times;</button></span>
                  {/each}
                </div>
              {/if}
              <div class="tag-input-wrap">
                <input
                  class="tag-input"
                  type="text"
                  bind:value={relatedInput}
                  placeholder={t('newArticle.relatedPlaceholder')}
                  onkeydown={(e) => { if (e.key === 'Enter' && relatedInput.trim()) { e.preventDefault(); appendArticleTag('related', relatedInput); } }}
                />
                {#if relatedSuggestions.length > 0}
                  <ul class="tag-suggestions">
                    {#each relatedSuggestions as s}
                      <li><button type="button" onclick={() => appendArticleTag('related', s.id)}>{s.name || s.id}</button></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
          </details>

          <details>
            <summary>{t('newArticle.prereqsLabel')}</summary>
            <div class="sb-field">
              <p class="sb-hint">{t('newArticle.prereqsHint')}</p>
              {#if prereqs.length > 0}
                <div class="selected-tags">
                  {#each prereqs as p (p.tag_id)}
                    <span class="tag lit prereq {p.prereq_type === 'recommended' ? 'recommended' : ''}">
                      {getTagName(p.tag_id)}
                      <button
                        type="button"
                        class="prereq-toggle"
                        title={p.prereq_type === 'required' ? t('books.prereqClickRecommended') : t('books.prereqClickRequired')}
                        onclick={() => togglePrereqType(p.tag_id)}
                      >{p.prereq_type === 'required' ? t('books.prereqRequired') : t('books.prereqRecommended')}</button>
                      <button class="tag-remove" onclick={() => removePrereq(p.tag_id)}>&times;</button>
                    </span>
                  {/each}
                </div>
              {/if}
              <div class="tag-input-wrap">
                <input
                  class="tag-input"
                  type="text"
                  bind:value={prereqInput}
                  placeholder={t('newArticle.prereqsPlaceholder')}
                  onkeydown={(e) => { if (e.key === 'Enter' && prereqInput.trim()) { e.preventDefault(); appendArticleTag('prereqs', prereqInput); } }}
                />
                {#if prereqSuggestions.length > 0}
                  <ul class="tag-suggestions">
                    {#each prereqSuggestions as s}
                      <li><button type="button" onclick={() => appendArticleTag('prereqs', s.id)}>{s.name || s.id}</button></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
          </details>

          {#if tagError}
            <p class="sb-error">{tagError}</p>
          {/if}

          {#if !isEditing}
            <details>
              <summary>{t('newArticle.langVersions')}</summary>
              <button class="btn-add-lang" onclick={addLangVersion}>+ {t('newArticle.addLangVersion')}</button>
              {#each extraLangs as lv, idx}
                <div class="lang-version-block">
                  <div class="lang-version-header">
                    <select bind:value={extraLangs[idx].lang}>
                      {#each [['zh', '中文'], ['en', 'English'], ['ja', '日本語'], ['ko', '한국어'], ['fr', 'Français'], ['de', 'Deutsch']] as [code, name]}
                        <option value={code} disabled={code === lang || extraLangs.some((l, i) => i !== idx && l.lang === code)}>{name}</option>
                      {/each}
                    </select>
                    <select bind:value={extraLangs[idx].contentFormat}>
                      <option value="typst">Typst</option>
                      <option value="markdown">Markdown</option>
                      <option value="html">HTML</option>
                    </select>
                    <label class="toolbar-btn">
                      <input type="file" accept=".md,.markdown,.typ,.typst,.html,.htm," onchange={(e) => handleLangFileLoad(idx, e)} hidden />
                      {t('newArticle.uploadFile')}
                    </label>
                    <button class="lang-remove" onclick={() => removeLangVersion(idx)}>&times;</button>
                  </div>
                  <textarea
                    bind:value={extraLangs[idx].content}
                    placeholder={t('newArticle.versionContent', lv.lang)}
                    class="lang-textarea"
                  ></textarea>
                </div>
              {/each}
            </details>
          {/if}
        </aside>
      {/if}
    </div>

    <!-- Footer -->
    <div class="editor-footer">
      <button class="btn btn-draft" onclick={handleSaveDraft} disabled={savingDraft}>
        {savingDraft
          ? t('newArticle.saving')
          : draftSaved
            ? t('newArticle.saved')
            : currentDraftId
              ? t('newArticle.updateDraft')
              : t('newArticle.saveDraft')}
      </button>

      <div class="footer-spacer"></div>

      <button class="btn btn-primary" onclick={submit} disabled={submitting || !title.trim() || !content.trim()}>
        {submitting ? t('newArticle.publishing') : t('newArticle.publish')}
      </button>
    </div>
</div>

<style>
  /* === Page layout === */
  .editor-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .error-banner {
    background: #fef2f2;
    color: #dc2626;
    padding: 8px 16px;
    font-size: 13px;
    border-bottom: 1px solid #fecaca;
  }

  /* === Title area === */
  .editor-title-area {
    padding: 6px 0 0;
    flex-shrink: 0;
    max-width: 760px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    padding-left: 1rem;
    padding-right: 1rem;
  }
  .title-input {
    display: block;
    width: 100%;
    border: none;
    font-family: var(--font-serif);
    font-size: 1.8rem;
    font-weight: 400;
    outline: none;
    padding: 0;
    margin-bottom: 4px;
    color: var(--text-primary);
    background: transparent;
  }
  .title-input::placeholder { color: var(--text-hint); }
  .desc-input {
    display: block;
    width: 100%;
    border: none;
    font-size: 14px;
    outline: none;
    padding: 0 0 8px;
    color: var(--text-secondary);
    background: transparent;
    border-bottom: 1px solid var(--border);
  }
  .desc-input::placeholder { color: var(--text-hint); }

  /* === Main body === */
  .editor-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* === Version Panel (left) === */
  .version-panel {
    width: 260px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    background: var(--bg-white);
  }

  /* === Editor main === */
  .editor-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    position: relative;
  }

  .sidebar-tab, .version-tab {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 10;
    width: 20px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-white);
    border: 1px solid var(--border);
    cursor: pointer;
    color: var(--text-secondary);
    padding: 0;
    transition: background 0.15s, color 0.15s;
  }
  .sidebar-tab:hover, .version-tab:hover {
    background: var(--accent-light, #e8f2e8);
    color: var(--accent, #4a7);
  }

  .sidebar-tab {
    right: 0;
    border-right: none;
    border-radius: 6px 0 0 6px;
    box-shadow: -2px 0 6px rgba(0,0,0,0.08);
  }

  .version-tab {
    left: 0;
    border-left: none;
    border-radius: 0 6px 6px 0;
    box-shadow: 2px 0 6px rgba(0,0,0,0.08);
  }

  .converting-hint {
    font-size: 11px;
    color: var(--accent);
  }
  .toolbar-btn {
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    padding: 3px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    transition: all 0.15s;
  }
  .toolbar-btn:hover { background: rgba(95,155,101,0.08); }

  .editor-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-page);
  }
  .editor-textarea {
    flex: 1;
    width: 100%;
    max-width: 760px;
    margin: 0 auto;
    border: none;
    outline: none;
    resize: none;
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    line-height: 1.5;
    padding: 2rem 1rem;
    color: var(--text-primary);
    background: var(--bg-page);
    box-sizing: border-box;
  }

  /* === Settings Sidebar (right) === */
  .settings-sidebar {
    width: 300px;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    overflow-y: auto;
    font-size: 13px;
    background: var(--bg-white);
  }
  .sb-uploads {
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .sb-upload-btn {
    flex: 1;
    text-align: center;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    padding: 5px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    transition: all 0.15s;
  }
  .sb-upload-btn:hover { background: rgba(95,155,101,0.08); }
  .sb-upload-btn.disabled { opacity: 0.5; pointer-events: none; }
  .settings-sidebar details {
    border-bottom: 1px solid var(--border);
  }
  .settings-sidebar summary {
    padding: 8px 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
    background: var(--bg-hover, #fafafa);
  }
  .settings-sidebar summary:hover { color: var(--text-primary); }
  .sb-field {
    padding: 6px 12px;
  }
  .sb-field label {
    display: block;
    font-size: 11px;
    color: var(--text-hint);
    margin-bottom: 3px;
  }
  .sb-field select, .sb-field input[type="text"] {
    width: 100%;
    padding: 4px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .sb-hint {
    font-size: 11px;
    color: var(--text-hint);
    padding: 4px 12px 0;
    margin: 0;
  }
  .check-label {
    display: flex !important;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    font-size: 12px !important;
    cursor: pointer;
    color: var(--text-secondary) !important;
  }

  /* Tag inputs inside sidebar (mirrors BookDetail) */
  .selected-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 6px;
  }
  .selected-tags .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }
  .selected-tags .tag.related {
    background: rgba(107, 114, 128, 0.1);
    color: var(--text-secondary);
  }
  .selected-tags .tag.prereq {
    background: rgba(217, 119, 6, 0.12);
    color: #b45309;
  }
  .selected-tags .tag.prereq.recommended {
    background: rgba(22, 163, 74, 0.1);
    color: #15803d;
  }
  .tag-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    color: inherit;
    padding: 0;
    line-height: 1;
    opacity: 0.6;
  }
  .tag-remove:hover { opacity: 1; }
  .prereq-toggle {
    font-size: 10px;
    padding: 0 4px;
    border: 1px solid currentColor;
    border-radius: 2px;
    background: none;
    color: inherit;
    cursor: pointer;
    line-height: 1.4;
    opacity: 0.85;
  }
  .prereq-toggle:hover { opacity: 1; background: rgba(0, 0, 0, 0.05); }
  .tag-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .tag-input {
    flex: 1;
    padding: 4px 8px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .tag-suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 10;
    list-style: none;
    margin: 2px 0 0;
    padding: 4px 0;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    max-height: 180px;
    overflow-y: auto;
  }
  .tag-suggestions li { margin: 0; }
  .tag-suggestions li button {
    display: block;
    width: 100%;
    padding: 4px 10px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-primary);
  }
  .tag-suggestions li button:hover { background: var(--bg-hover, #f5f5f5); }
  .sb-error {
    padding: 4px 12px;
    margin: 0;
    font-size: 11px;
    color: #c53030;
  }

  /* Language versions inside sidebar */
  .btn-add-lang {
    font-size: 12px;
    color: var(--accent);
    background: none;
    border: 1px dashed var(--accent);
    border-radius: 3px;
    padding: 3px 10px;
    cursor: pointer;
    margin: 6px 12px;
    display: block;
  }
  .btn-add-lang:hover { background: rgba(95,155,101,0.08); }
  .lang-version-block {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px;
    margin: 6px 12px;
    background: var(--bg-hover, #fafafa);
  }
  .lang-version-header {
    display: flex;
    gap: 4px;
    align-items: center;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }
  .lang-version-header select {
    padding: 3px 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
  }
  .lang-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-hint);
    padding: 0 4px;
    line-height: 1;
    margin-left: auto;
  }
  .lang-remove:hover { color: #dc2626; }
  .lang-textarea {
    width: 100%;
    min-height: 100px;
    padding: 6px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    resize: vertical;
  }

  /* === Footer === */
  .editor-footer {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .footer-spacer { flex: 1; }
  .footer-status { font-size: 12px; }
  .status-unsaved { color: #d97706; }
  .status-saved { color: var(--text-hint); }

  .btn {
    padding: 6px 16px;
    font-size: 13px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary {
    background: var(--accent);
    color: white;
    border: none;
  }
  .btn-primary:hover:not(:disabled) { opacity: 0.9; }
  .btn-outline {
    background: var(--bg-white);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }
  .btn-outline:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-draft {
    padding: 6px 16px;
    font-size: 13px;
    border: 1px dashed var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .btn-draft:hover { border-color: var(--accent); color: var(--accent); }
  .btn-draft:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Record button group */
  .record-group {
    position: relative;
  }
  .record-popup {
    position: absolute;
    bottom: 100%;
    right: 0;
    margin-bottom: 8px;
    display: flex;
    gap: 6px;
    padding: 10px;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.12);
    min-width: 320px;
    z-index: 20;
  }
  .record-msg-input {
    flex: 1;
    padding: 6px 10px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
    outline: none;
  }
  .record-msg-input:focus { border-color: var(--accent); }
  .record-msg-input::placeholder { color: var(--text-hint); }
  .btn-sm {
    padding: 6px 12px !important;
    font-size: 12px !important;
  }
  .btn-record {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 18px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .btn-record:hover:not(:disabled) { opacity: 0.9; }
  .btn-record:disabled { opacity: 0.5; cursor: not-allowed; }

  /* === Responsive === */
  @media (max-width: 900px) {
    .version-panel { display: none; }
    .settings-sidebar { display: none; }
  }
</style>
