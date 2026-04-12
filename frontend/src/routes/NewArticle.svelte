<script lang="ts">
  import { listTags, searchTags, createArticle, listArticles, getArticle, getArticleContent, forkArticle, convertContent, uploadImage, updateArticle, saveArticle, recordArticle, saveDraft, updateDraft as apiUpdateDraft, listDrafts, getBook, getArticleHistory, getArticleDiff, unrecordArticleChange, listArticleCollaborators, inviteArticleCollaborator, removeArticleCollaborator, listArticleChannels, readArticleChannelFile, writeArticleChannelFile, articleChannelDiff, applyArticleChannelChange } from '../lib/api';
  import ChannelPanel from 'pijul-editor/ChannelPanel.svelte';
  import VersionPanel from 'pijul-editor/VersionPanel.svelte';
  import type { DiffLine } from 'pijul-editor/VersionPanel.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { getLangPrefs } from '../lib/langPrefs.svelte';
  import MarkdownEditor from 'pijul-editor/MarkdownEditor.svelte';
  import TypstEditor from 'pijul-editor/TypstEditor.svelte';
  import type { Tag, Article, BookEdition, ContentFormat, PrereqType, ArticleVersionInfo } from '../lib/types';

  let { forkOf = '', editUri = '', draftId: initialDraftId = '', initialCategory = '', initialBookId = '' } = $props();
  let isEditing = $state(false);
  // svelte-ignore state_referenced_locally
  let currentDraftId = $state(initialDraftId);
  let savingDraft = $state(false);
  let draftSaved = $state(false);
  let autoSaveTimer: ReturnType<typeof setTimeout> | undefined;

  // Auto-save draft 2 s after any content change (new articles only, not edits of published ones)
  $effect(() => {
    // Touch reactive state to subscribe
    const _t = title, _d = description, _c = content, _f = contentFormat, _l = lang;
    if (isEditing || forkSource || !_t.trim()) return;
    clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      if (!savingDraft) handleSaveDraft();
    }, 2000);
    return () => clearTimeout(autoSaveTimer);
  });

  let tags = $state<Tag[]>([]);
  let allArticles = $state<Article[]>([]);
  let title = $state('');
  let description = $state('');
  let content = $state('');
  let contentFormat = $state<ContentFormat>(getLangPrefs()?.default_format || 'markdown');
  let lang = $state(getLangPrefs()?.native_lang || getLocale());
  let license = $state('CC-BY-SA-4.0');
  let restricted = $state(false);
  let translationOf = $state('');
  let commitMessage = $state('');
  let category = $state(initialCategory || 'general');
  let bookId = $state(initialBookId || '');
  let editionId = $state('');
  let bookEditions = $state<BookEdition[]>([]);
  let selectedTags = $state<string[]>([]);
  let prereqs = $state<Array<{ tag_id: string; prereq_type: PrereqType }>>([]);
  let submitting = $state(false);
  let error = $state('');
  let forkSource = $state('');
  let uploadingImage = $state(false);
  let savedArticleUri = $state(''); // Set after first save, needed for image upload
  let showDiff = $state(false);
  let originalContent = $state(''); // Original content for fork diff
  let loadingFile = $state(false);
  let converting = $state(false);
  let originalFormat = $state<ContentFormat | ''>(''); // Track source format for fork conversion

  // --- Channel state ---
  let articleChannels = $state<string[]>(['main']);
  let currentArticleChannel = $state('main');

  // --- UI state ---
  let sidebarOpen = $state(true);
  let versionPanelOpen = $state(true);
  let lastSavedContent = $state(''); // For diff computation in version panel
  let saving = $state(false);

  // --- Version panel state ---
  let versionHistory = $state<ArticleVersionInfo[]>([]);
  let recording = $state(false);
  let loadingHistory = $state(false);

  // --- Sidebar quick-record popup ---
  let showRecordInput = $state(false);
  let recordMessage = $state('');

  async function loadHistory() {
    if (!savedArticleUri) return;
    loadingHistory = true;
    try {
      versionHistory = await getArticleHistory(savedArticleUri);
    } catch { /* ok */ }
    loadingHistory = false;
  }

  async function doUnrecord(v: ArticleVersionInfo) {
    if (!confirm(t('version.confirmUnrecord'))) return;
    try {
      await unrecordArticleChange(savedArticleUri, v.id);
      await loadHistory();
      // Reload content from server after unrecord
      const c = await getArticleContent(savedArticleUri);
      content = c.source;
      lastSavedContent = c.source;
    } catch (e: any) {
      error = e.message;
    }
  }

  async function doRecord(message: string) {
    if (!title.trim() || !content.trim()) {
      error = t('newArticle.fillRequired');
      return;
    }
    recording = true;
    error = '';
    try {
      const msg = message;

      if (!savedArticleUri) {
        // New article: publish first, which auto-records the initial change
        const article = await createArticle({
          title: title.trim(),
          description: description.trim() || undefined,
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
        });
        savedArticleUri = article.at_uri;
        lastSavedContent = content;
        isEditing = true;
        await loadHistory();
      } else {
        // Existing article: save + record
        if (content !== lastSavedContent) {
          await saveArticle(savedArticleUri, {
            title: title.trim(),
            description: description.trim(),
            content: content.trim(),
          });
          lastSavedContent = content;
        }
        versionHistory = await recordArticle(savedArticleUri, msg);
      }
    } catch (e: any) {
      error = e.message;
    } finally {
      recording = false;
    }
  }

  async function doSave() {
    if (!savedArticleUri || saving) return;
    saving = true;
    error = '';
    try {
      await saveArticle(savedArticleUri, {
        title: title.trim(),
        description: description.trim(),
        content: content.trim(),
      });
      lastSavedContent = content;
    } catch (e: any) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  // Simple line-based diff for the version panel (current changes)
  function computeSimpleDiff(oldText: string, newText: string): DiffLine[] {
    const oldLines = oldText.split('\n');
    const newLines = newText.split('\n');
    const result: DiffLine[] = [];
    const m = oldLines.length, n = newLines.length;
    const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
    for (let i = 1; i <= m; i++)
      for (let j = 1; j <= n; j++)
        dp[i][j] = oldLines[i-1] === newLines[j-1] ? dp[i-1][j-1] + 1 : Math.max(dp[i-1][j], dp[i][j-1]);
    let i = m, j = n;
    const ops: DiffLine[] = [];
    while (i > 0 || j > 0) {
      if (i > 0 && j > 0 && oldLines[i-1] === newLines[j-1]) {
        ops.push({ type: 'same', text: oldLines[i-1] }); i--; j--;
      } else if (j > 0 && (i === 0 || dp[i][j-1] >= dp[i-1][j])) {
        ops.push({ type: 'add', text: newLines[j-1] }); j--;
      } else {
        ops.push({ type: 'del', text: oldLines[i-1] }); i--;
      }
    }
    return ops.reverse();
  }

  let currentDiffLines = $derived(
    savedArticleUri && content !== lastSavedContent
      ? computeSimpleDiff(lastSavedContent, content)
      : (!savedArticleUri && content.trim())
        ? content.split('\n').map(l => ({ type: 'add' as const, text: l }))
        : []
  );
  let hasUnsavedChanges = $derived(savedArticleUri ? content !== lastSavedContent : false);

  async function handleFormatChange(newFormat: ContentFormat) {
    const oldFormat = contentFormat;
    contentFormat = newFormat;
    if (!forkSource || !content.trim() || oldFormat === newFormat) return;
    converting = true;
    error = '';
    try {
      const result = await convertContent(content, oldFormat, newFormat);
      content = result.content;
    } catch (e: any) {
      error = `${t('newArticle.convertError')}: ${e.message}`;
      contentFormat = oldFormat;
    } finally {
      converting = false;
    }
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

  // For tag input with server-side autocomplete
  let newTagInput = $state('');
  let showTagSuggestions = $state(false);
  let tagSuggestionList = $state<Tag[]>([]);
  let searchTimeout: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const q = newTagInput.trim();
    clearTimeout(searchTimeout);
    if (!q) { tagSuggestionList = []; return; }
    searchTimeout = setTimeout(() => {
      searchTags(q).then(results => { tagSuggestionList = results; });
    }, 150);
  });

  function addNewTag() {
    const val = newTagInput.trim();
    if (!val) return;
    const existing = tags.find(t => t.id === val || t.name.toLowerCase() === val.toLowerCase());
    const tagId = existing ? existing.id : val;
    if (!selectedTags.includes(tagId)) {
      selectedTags = [...selectedTags, tagId];
    }
    newTagInput = '';
    showTagSuggestions = false;
  }

  // For prereq adding
  let prereqTagId = $state('');
  let prereqType = $state<PrereqType>('required');

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
        description = a.description || '';
        content = c.source;
        lastSavedContent = c.source;
        contentFormat = a.content_format;
        lang = a.lang || 'zh';
        license = a.license || 'CC-BY-SA-4.0';
      });
      loadHistory();
      // Load channels for collaboration
      listArticleChannels(editUri).then(chs => { articleChannels = chs; }).catch(() => {});
    } else if (initialDraftId) {
      listDrafts().then(drafts => {
        const d = drafts.find(d => d.id === initialDraftId);
        if (!d) return;
        title = d.title;
        description = d.description || '';
        content = d.content;
        contentFormat = d.content_format;
        lang = d.lang || 'zh';
        license = d.license || 'CC-BY-SA-4.0';
        try { selectedTags = JSON.parse(d.tags); } catch { selectedTags = []; }
        try { prereqs = JSON.parse(d.prereqs); } catch { prereqs = []; }
      });
    } else if (forkOf) {
      Promise.all([getArticle(forkOf), getArticleContent(forkOf)]).then(([a, c]) => {
        title = `Fork: ${a.title}`;
        description = a.description || '';
        content = c.source;
        originalContent = c.source;
        contentFormat = a.content_format;
        originalFormat = a.content_format;
        lang = a.lang || 'zh';
        license = a.license || 'CC-BY-SA-4.0';
        forkSource = forkOf;
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

  function addPrereq() {
    if (!prereqTagId) return;
    if (prereqs.some(p => p.tag_id === prereqTagId)) return;
    prereqs = [...prereqs, { tag_id: prereqTagId, prereq_type: prereqType }];
    prereqTagId = '';
  }

  function removePrereq(tagId: string) {
    prereqs = prereqs.filter(p => p.tag_id !== tagId);
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
          description: description.trim() || undefined,
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
        });
        savedArticleUri = article.at_uri;
        lastSavedContent = content;
        isEditing = true;
        loadHistory();
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
        description: description.trim() || undefined,
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
      });
      savedArticleUri = article.at_uri;
      lastSavedContent = content;
      isEditing = true;
      loadHistory();
    }
    const result = await uploadImage(savedArticleUri, file);
    return { src: result.filename, alt: result.filename };
  }

  // Fork diff (reused from original)
  let diffLines = $derived(
    forkSource && showDiff ? computeSimpleDiff(originalContent, content) : []
  );

  interface DiffHunk { lines: DiffLine[]; collapsed?: number; }
  let diffHunks = $derived.by((): DiffHunk[] => {
    if (diffLines.length === 0) return [];
    const CONTEXT = 3;
    const show = new Uint8Array(diffLines.length);
    for (let i = 0; i < diffLines.length; i++) {
      if (diffLines[i].type !== 'same') {
        for (let j = Math.max(0, i - CONTEXT); j <= Math.min(diffLines.length - 1, i + CONTEXT); j++) {
          show[j] = 1;
        }
      }
    }
    const hunks: DiffHunk[] = [];
    let i = 0;
    while (i < diffLines.length) {
      if (show[i]) {
        const lines: DiffLine[] = [];
        while (i < diffLines.length && show[i]) {
          lines.push(diffLines[i]);
          i++;
        }
        hunks.push({ lines });
      } else {
        let skip = 0;
        while (i < diffLines.length && !show[i]) { skip++; i++; }
        hunks.push({ lines: [], collapsed: skip });
      }
    }
    return hunks;
  });
  let hasForkChanges = $derived(forkSource ? content !== originalContent : true);

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

  function previewDiff() {
    if (!title.trim() || !content.trim()) {
      error = t('newArticle.fillRequired');
      return;
    }
    error = '';
    showDiff = true;
  }

  async function handleSaveDraft() {
    savingDraft = true;
    draftSaved = false;
    error = '';
    try {
      const data = {
        title: title.trim() || t('newArticle.untitledDraft'),
        description: description.trim() || undefined,
        content: content,
        content_format: contentFormat,
        lang: lang || getLocale(),
        license: license || undefined,
        tags: selectedTags,
        prereqs,
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
          description: description.trim(),
          content: content.trim(),
          commit_message: commitMessage.trim() || undefined,
        });
        window.location.href = `/article?uri=${encodeURIComponent(article.at_uri)}`;
      } else if (forkSource) {
        const targetFormat = contentFormat !== originalFormat ? contentFormat : undefined;
        const forked = await forkArticle(forkSource, targetFormat);

        if (hasForkChanges) {
          await updateArticle(forked.at_uri, {
            title: title.trim(),
            description: description.trim() || undefined,
            content: content.trim(),
            commit_message: 'Initial fork edits',
          });
        }
        window.location.href = `/article?uri=${encodeURIComponent(forked.at_uri)}`;
      } else {
        const article = await createArticle({
          title: title.trim(),
          description: description.trim() || undefined,
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
        });

        for (const lv of extraLangs) {
          if (!lv.content.trim()) continue;
          try {
            await createArticle({
              title: title.trim(),
              description: description.trim() || undefined,
              content: lv.content.trim(),
              content_format: lv.contentFormat,
              lang: lv.lang,
              license: license || undefined,
              translation_of: article.at_uri,
              category: category || undefined,
              book_id: bookId || undefined,
              tags: selectedTags,
              prereqs,
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

  {#if showDiff && forkSource}
    <!-- Fork diff overlay -->
    <div class="diff-overlay">
      <div class="diff-header">
        <h3>{t('newArticle.diffPreview')}</h3>
        <button class="btn-outline" onclick={() => showDiff = false}>{t('newArticle.backToEdit')}</button>
      </div>
      {#if !hasForkChanges}
        <p class="diff-empty">{t('newArticle.noChanges')}</p>
      {:else}
        <div class="diff-stats">
          <span class="diff-add-count">+{diffLines.filter(l => l.type === 'add').length}</span>
          <span class="diff-del-count">-{diffLines.filter(l => l.type === 'del').length}</span>
        </div>
        <pre class="diff-content">{#each diffHunks as hunk}{#if hunk.collapsed}<span class="line-collapse">... {t('newArticle.linesUnchanged', hunk.collapsed)} ...</span>
{:else}{#each hunk.lines as line}{#if line.type === 'add'}<span class="line-add">+{line.text}</span>
{:else if line.type === 'del'}<span class="line-del">-{line.text}</span>
{:else}<span class="line-same"> {line.text}</span>
{/if}{/each}{/if}{/each}</pre>
      {/if}
      <div class="diff-actions">
        <button class="btn btn-primary" onclick={submit} disabled={submitting || !hasForkChanges}>
          {submitting ? t('newArticle.publishing') : t('newArticle.confirmFork')}
        </button>
      </div>
    </div>
  {:else}
    <!-- Title area -->
    <div class="editor-title-area">
      {#if forkSource}
        <div class="fork-hint">{t('newArticle.forkHint')}</div>
      {/if}
      <input
        class="title-input"
        bind:value={title}
        placeholder={t('newArticle.titleLabel')}
      />
      <input
        class="desc-input"
        bind:value={description}
        placeholder={t('newArticle.descPlaceholder')}
      />
    </div>

    <!-- Main body: version panel + editor + settings sidebar -->
    <div class="editor-body">
      <!-- Left: Version Panel -->
      {#if versionPanelOpen}
        <aside class="version-panel">
          <VersionPanel
            {currentDiffLines}
            versions={versionHistory}
            {loadingHistory}
            {recording}
            onRecord={doRecord}
            onUnrecord={doUnrecord}
            onFetchDiff={async (v) => {
              const idx = versionHistory.findIndex(h => h.id === v.id);
              const prev = idx + 1 < versionHistory.length ? versionHistory[idx + 1] : null;
              if (!prev) return [];
              const diff = await getArticleDiff(savedArticleUri, prev.id, v.id);
              return diff.hunks.flatMap(h => h.lines.map(l => ({
                type: l.kind === 'add' ? 'add' as const : l.kind === 'remove' ? 'del' as const : 'same' as const,
                text: l.content,
              })));
            }}
            labels={{
              diff: t('version.diff'),
              noChanges: t('version.noChanges'),
              history: t('version.history'),
              noHistory: t('version.noHistory'),
              recordPlaceholder: t('version.recordPlaceholder'),
              record: t('version.record'),
            }}
          />
        </aside>
      {/if}

      <!-- Center: Editor -->
      <div class="editor-main">
        <!-- Version panel toggle tab on left edge -->
        <button
          class="version-tab"
          class:open={versionPanelOpen}
          onclick={() => versionPanelOpen = !versionPanelOpen}
          title={t('version.togglePanel')}
          aria-label={t('version.togglePanel')}
        >
          <svg width="12" height="22" viewBox="0 0 10 18" fill="currentColor">
            {#if versionPanelOpen}
              <polyline points="8,2 2,9 8,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {:else}
              <polyline points="2,2 8,9 2,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {/if}
          </svg>
        </button>

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
            <label>{t('newArticle.formatLabel')}</label>
            <select value={contentFormat} onchange={(e) => handleFormatChange((e.target as HTMLSelectElement).value as ContentFormat)} disabled={converting}>
              <option value="markdown">Markdown + KaTeX</option>
              <option value="typst">Typst</option>
              {#if contentFormat === 'html'}<option value="html">HTML (只读)</option>{/if}
            </select>
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
              <label>{t('newArticle.langLabel')}</label>
              <select bind:value={lang}>
                <option value="zh">中文</option>
                <option value="en">English</option>
                <option value="ja">日本語</option>
                <option value="ko">한국어</option>
                <option value="fr">Français</option>
                <option value="de">Deutsch</option>
              </select>
            </div>
            <div class="sb-field">
              <label class="check-label">
                <input type="checkbox" bind:checked={restricted} />
                {t('newArticle.restricted')}
              </label>
            </div>
            {#if !restricted}
            <div class="sb-field">
              <label>{t('newArticle.licenseLabel')}</label>
              <select bind:value={license}>
                <option value="CC-BY-NC-SA-4.0">CC BY-NC-SA 4.0</option>
                <option value="CC-BY-SA-4.0">CC BY-SA 4.0</option>
                <option value="CC-BY-4.0">CC BY 4.0</option>
                <option value="CC-BY-NC-4.0">CC BY-NC 4.0</option>
                <option value="CC-BY-NC-ND-4.0">CC BY-NC-ND 4.0</option>
                <option value="CC0-1.0">CC0</option>
                <option value="MIT">MIT</option>
                <option value="Apache-2.0">Apache 2.0</option>
                <option value="GFDL-1.3">GFDL 1.3</option>
                <option value="All-Rights-Reserved">All Rights Reserved</option>
              </select>
            </div>
            {/if}
            <div class="sb-field">
              <label>{t('newArticle.categoryLabel')}</label>
              <input
                bind:value={category}
                list="category-suggestions"
                placeholder={t('category.general')}
                class="category-input"
              />
              <datalist id="category-suggestions">
                <option value="general">{t('category.general')}</option>
                <option value="lecture">{t('category.lecture')}</option>
                <option value="paper">{t('category.paper')}</option>
                <option value="review">{t('category.review')}</option>
              </datalist>
            </div>
            {#if category === 'review' && bookEditions.length > 0}
              <div class="sb-field">
                <label>{t('newArticle.editionLabel')}</label>
                <select bind:value={editionId}>
                  <option value="">{t('newArticle.noEdition')}</option>
                  {#each bookEditions as ed}
                    <option value={ed.id}>{ed.title} ({ed.lang}{ed.year ? `, ${ed.year}` : ''})</option>
                  {/each}
                </select>
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

          <details>
            <summary>{t('newArticle.tagsLabel')}</summary>
            <div class="sb-field">
              <div class="tag-input-row">
                <input
                  type="text"
                  bind:value={newTagInput}
                  placeholder={t('newArticle.tagInput')}
                  onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addNewTag(); } }}
                  onfocus={() => showTagSuggestions = true}
                  onblur={() => setTimeout(() => showTagSuggestions = false, 200)}
                  oninput={() => showTagSuggestions = true}
                />
                <button class="tag-add-btn" onclick={addNewTag}>{t('common.add')}</button>
                {#if showTagSuggestions && tagSuggestionList.length > 0}
                  <div class="tag-suggestions">
                    {#each tagSuggestionList as s}
                      <button type="button" onmousedown={() => { toggleTag(s.id); newTagInput = ''; showTagSuggestions = false; }}>
                        {s.name} {#if s.name !== s.id}<span class="sg-id">({s.id})</span>{/if}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
              {#if selectedTags.length > 0}
                <div class="selected-tags">
                  {#each selectedTags as tagId}
                    <span class="tag lit">{getTagName(tagId)} <button class="tag-remove" onclick={() => toggleTag(tagId)}>&times;</button></span>
                  {/each}
                </div>
              {/if}
              <div class="tag-picker">
                {#each tags.filter(t => !selectedTags.includes(t.id)).slice(0, 15) as t}
                  <button class="tag" onclick={() => toggleTag(t.id)}>{t.name}</button>
                {/each}
              </div>
            </div>
          </details>

          <details>
            <summary>{t('newArticle.prereqsLabel')}</summary>
            <p class="sb-hint">{t('newArticle.prereqsHint')}</p>
            {#if prereqs.length > 0}
              <div class="prereq-list">
                {#each prereqs as p}
                  <div class="prereq-item">
                    <span class="tag {p.prereq_type}">{getTagName(p.tag_id)}</span>
                    <span class="prereq-type-label">{p.prereq_type}</span>
                    <button class="prereq-remove" onclick={() => removePrereq(p.tag_id)}>&times;</button>
                  </div>
                {/each}
              </div>
            {/if}
            <div class="prereq-add">
              <select bind:value={prereqTagId}>
                <option value="">{t('newArticle.selectTag')}</option>
                {#each tags.filter(t => !prereqs.some(p => p.tag_id === t.id)) as t}
                  <option value={t.id}>{t.name}</option>
                {/each}
              </select>
              <select bind:value={prereqType}>
                <option value="required">{t('newArticle.required')}</option>
                <option value="recommended">{t('newArticle.recommended')}</option>
                <option value="suggested">{t('newArticle.suggested')}</option>
              </select>
              <button class="prereq-add-btn" onclick={addPrereq} disabled={!prereqTagId}>{t('newArticle.addPrereq')}</button>
            </div>
          </details>

          {#if !isEditing && !forkSource}
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
          {#if isEditing && savedArticleUri && articleChannels.length > 0}
            <details>
              <summary>协作</summary>
              <ChannelPanel
                currentChannel={currentArticleChannel}
                channels={articleChannels}
                currentUserDid={getAuth()?.did || ''}
                onChannelChange={(ch) => { currentArticleChannel = ch; }}
                fetchCollaborators={() => listArticleCollaborators(savedArticleUri)}
                doInvite={(did) => inviteArticleCollaborator(savedArticleUri, did).then(() => {})}
                doRemove={(did) => removeArticleCollaborator(savedArticleUri, did)}
                fetchDiff={(target, current) => articleChannelDiff(savedArticleUri, target, current)}
                doApply={(target, _source, hash) => applyArticleChannelChange(savedArticleUri, target, hash)}
              />
            </details>
          {/if}
        </aside>
      {/if}
    </div>

    <!-- Footer -->
    <div class="editor-footer">
      {#if isEditing}
        <span class="footer-status">
          {#if hasUnsavedChanges}
            <span class="status-unsaved">{t('version.unsaved')}</span>
          {:else}
            <span class="status-saved">{t('version.saved')}</span>
          {/if}
        </span>
        <button class="btn btn-outline" onclick={doSave} disabled={saving || !hasUnsavedChanges}>
          {saving ? t('newArticle.saving') : t('common.save')}
        </button>
      {:else if !forkSource}
        <button class="btn btn-draft" onclick={handleSaveDraft} disabled={savingDraft}>
          {savingDraft
            ? t('newArticle.saving')
            : draftSaved
              ? t('newArticle.saved')
              : currentDraftId
                ? t('newArticle.updateDraft')
                : t('newArticle.saveDraft')}
        </button>
      {/if}

      <div class="footer-spacer"></div>

      {#if forkSource}
        <button class="btn btn-outline" onclick={previewDiff}>
          {t('newArticle.previewDiff')}
        </button>
        <button class="btn btn-primary" onclick={previewDiff} disabled={submitting}>
          {submitting ? t('newArticle.publishing') : t('newArticle.publish')}
        </button>
      {:else}
        <!-- Record button with popup input -->
        <div class="record-group">
          {#if showRecordInput}
            <div class="record-popup">
              <input
                class="record-msg-input"
                bind:value={recordMessage}
                placeholder={t('version.recordPlaceholder')}
                maxlength={200}
                onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); doRecord(recordMessage.trim() || 'Update'); recordMessage = ''; showRecordInput = false; } if (e.key === 'Escape') { showRecordInput = false; } }}
              />
              <button class="btn btn-primary btn-sm" onclick={() => { doRecord(recordMessage.trim() || 'Update'); recordMessage = ''; showRecordInput = false; }} disabled={recording || (!title.trim() || !content.trim())}>
                {recording ? '...' : t('version.record')}
              </button>
            </div>
          {/if}
          <button class="btn btn-record" onclick={() => { showRecordInput = !showRecordInput; }} disabled={recording}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
            {t('version.record')}
          </button>
        </div>
      {/if}
    </div>
  {/if}
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
  .fork-hint {
    font-size: 13px;
    color: var(--accent);
    margin-bottom: 4px;
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

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .toolbar-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 4px 6px;
    border: none;
    background: none;
    border-radius: 3px;
    cursor: pointer;
    color: var(--text-hint);
    transition: all 0.15s;
  }
  .toolbar-icon:hover { background: var(--bg-hover); color: var(--text-primary); }
  .toolbar-icon.active { color: var(--accent); background: rgba(95,155,101,0.1); }
  .toolbar-select {
    padding: 3px 8px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .converting-hint {
    font-size: 11px;
    color: var(--accent);
  }
  .toolbar-spacer { flex: 1; }
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
  .toolbar-btn.disabled { opacity: 0.5; pointer-events: none; }

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

  /* Tags inside sidebar */
  .tag-input-row {
    position: relative;
    display: flex;
    gap: 4px;
    margin-bottom: 6px;
  }
  .tag-input-row input { flex: 1; }
  .tag-add-btn {
    padding: 4px 10px;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    white-space: nowrap;
  }
  .tag-suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 40px;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10;
    max-height: 150px;
    overflow-y: auto;
  }
  .tag-suggestions button {
    display: block;
    width: 100%;
    padding: 4px 8px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }
  .tag-suggestions button:hover { background: var(--bg-gray, #f5f5f5); }
  .sg-id { color: var(--text-hint); font-size: 10px; }
  .selected-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 6px;
  }
  .selected-tags .tag { display: inline-flex; align-items: center; gap: 3px; font-size: 12px; }
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
  .tag-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }
  .tag-picker .tag { cursor: pointer; font-size: 11px; }

  /* Prereqs inside sidebar */
  .prereq-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 12px;
  }
  .prereq-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px;
    background: var(--bg-hover);
    border-radius: 3px;
    font-size: 12px;
  }
  .prereq-type-label {
    font-size: 11px;
    color: var(--text-hint);
    margin-left: auto;
  }
  .prereq-remove {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 14px;
    padding: 0 2px;
    line-height: 1;
  }
  .prereq-remove:hover { color: #dc2626; }
  .prereq-add {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 6px 12px;
  }
  .prereq-add select {
    padding: 3px 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
  }
  .prereq-add-btn {
    padding: 3px 10px;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .prereq-add-btn:disabled { opacity: 0.4; cursor: not-allowed; }

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

  /* === Fork diff overlay === */
  .diff-overlay {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    max-width: 800px;
    margin: 0 auto;
  }
  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .diff-header h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 16px;
    margin: 0;
  }
  .diff-stats { font-size: 13px; margin-bottom: 8px; display: flex; gap: 12px; }
  .diff-add-count { color: #22863a; }
  .diff-del-count { color: #cb2431; }
  .diff-empty {
    color: var(--text-hint);
    text-align: center;
    padding: 2rem 0;
  }
  .diff-content {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
    overflow-x: auto;
    max-height: 60vh;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 0;
    margin: 0 0 12px;
    background: #fafafa;
  }
  .diff-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  /* === Responsive === */
  @media (max-width: 900px) {
    .version-panel { display: none; }
    .settings-sidebar { display: none; }
  }
</style>
