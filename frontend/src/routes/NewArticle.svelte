<script lang="ts">
  import { listTags, searchTags, createArticle, listArticles, getArticle, getArticleContent, forkArticle, uploadImage, updateArticle, saveDraft, updateDraft as apiUpdateDraft, listDrafts } from '../lib/api';
  import { t } from '../lib/i18n';
  import type { Tag, Article } from '../lib/types';

  let { forkOf = '', editUri = '', draftId: initialDraftId = '' } = $props();
  let isEditing = $state(false);
  // svelte-ignore state_referenced_locally
  let currentDraftId = $state(initialDraftId);
  let savingDraft = $state(false);
  let draftSaved = $state(false);

  let tags = $state<Tag[]>([]);
  let allArticles = $state<Article[]>([]);
  let title = $state('');
  let description = $state('');
  let content = $state('');
  let contentFormat = $state('typst');
  let lang = $state('zh');
  let license = $state('CC-BY-NC-SA-4.0');
  let translationOf = $state('');
  let selectedTags = $state<string[]>([]);
  let prereqs = $state<Array<{ tag_id: string; prereq_type: string }>>([]);
  let submitting = $state(false);
  let error = $state('');
  let forkSource = $state('');
  let uploadingImage = $state(false);
  let savedArticleUri = $state(''); // Set after first save, needed for image upload
  let showDiff = $state(false);
  let originalContent = $state(''); // Original content for fork diff
  let loadingFile = $state(false);

  // Multi-language versions
  interface LangVersion {
    lang: string;
    content: string;
    contentFormat: string;
  }
  let extraLangs = $state<LangVersion[]>([]);
  let showAddLang = $state(false);

  function addLangVersion() {
    // Pick first unused language
    const usedLangs = new Set([lang, ...extraLangs.map(l => l.lang)]);
    const available = ['en', 'zh', 'ja', 'ko', 'fr', 'de'].filter(l => !usedLangs.has(l));
    if (available.length === 0) return;
    extraLangs = [...extraLangs, { lang: available[0], content: '', contentFormat }];
    showAddLang = false;
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
    // If matches an existing tag, use that
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
  let prereqType = $state('required');

  $effect(() => {
    listTags().then(data => { tags = data; });
    listArticles().then(data => { allArticles = data; });
    if (editUri) {
      isEditing = true;
      savedArticleUri = editUri;
      Promise.all([getArticle(editUri), getArticleContent(editUri)]).then(([a, c]) => {
        title = a.title;
        description = a.description || '';
        content = c.source;
        contentFormat = a.content_format;
        lang = a.lang || 'zh';
        license = a.license || 'CC-BY-NC-SA-4.0';
      });
    } else if (initialDraftId) {
      listDrafts().then(drafts => {
        const d = drafts.find(d => d.id === initialDraftId);
        if (!d) return;
        title = d.title;
        description = d.description || '';
        content = d.content;
        contentFormat = d.content_format;
        lang = d.lang || 'zh';
        license = d.license || 'CC-BY-NC-SA-4.0';
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
        lang = a.lang || 'zh';
        license = a.license || 'CC-BY-NC-SA-4.0';
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

    // Need a saved article to upload to. If no savedArticleUri, create article first.
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
          lang: lang || 'zh',
          license: license || undefined,
          translation_of: translationOf || undefined,
          tags: selectedTags,
          prereqs,
        });
        savedArticleUri = article.at_uri;
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
      // Insert image reference at cursor position
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

  // Simple line-based diff for preview
  interface DiffLine { type: 'same' | 'add' | 'del'; text: string; }
  function computeDiff(oldText: string, newText: string): DiffLine[] {
    const oldLines = oldText.split('\n');
    const newLines = newText.split('\n');
    const result: DiffLine[] = [];

    // Simple LCS-based diff
    const m = oldLines.length, n = newLines.length;
    // Use O(n) space DP for LCS length, then backtrack
    const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
    for (let i = 1; i <= m; i++)
      for (let j = 1; j <= n; j++)
        dp[i][j] = oldLines[i-1] === newLines[j-1] ? dp[i-1][j-1] + 1 : Math.max(dp[i-1][j], dp[i][j-1]);

    let i = m, j = n;
    const ops: DiffLine[] = [];
    while (i > 0 || j > 0) {
      if (i > 0 && j > 0 && oldLines[i-1] === newLines[j-1]) {
        ops.push({ type: 'same', text: oldLines[i-1] });
        i--; j--;
      } else if (j > 0 && (i === 0 || dp[i][j-1] >= dp[i-1][j])) {
        ops.push({ type: 'add', text: newLines[j-1] });
        j--;
      } else {
        ops.push({ type: 'del', text: oldLines[i-1] });
        i--;
      }
    }
    return ops.reverse();
  }

  let diffLines = $derived(
    forkSource && showDiff ? computeDiff(originalContent, content) : []
  );

  // Collapse unchanged lines, showing only 3 lines of context around changes
  interface DiffHunk { lines: DiffLine[]; collapsed?: number; }
  let diffHunks = $derived.by((): DiffHunk[] => {
    if (diffLines.length === 0) return [];
    const CONTEXT = 3;
    // Mark which lines are "near" a change
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
  let hasChanges = $derived(forkSource ? content !== originalContent : true);

  async function handleFileLoad(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    loadingFile = true;
    error = '';
    try {
      const text = await file.text();
      content = text;
      // Auto-detect format from extension
      const name = file.name.toLowerCase();
      if (name.endsWith('.md') || name.endsWith('.markdown')) {
        contentFormat = 'markdown';
      } else if (name.endsWith('.typ') || name.endsWith('.typst')) {
        contentFormat = 'typst';
      } else if (name.endsWith('.html') || name.endsWith('.htm')) {
        contentFormat = 'html';
      }
      // Use filename (without extension) as title if title is empty
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
        lang: lang || 'zh',
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
        });
        window.location.hash = `#/article?uri=${encodeURIComponent(article.at_uri)}`;
      } else {
        const article = await createArticle({
          title: title.trim(),
          description: description.trim() || undefined,
          content: content.trim(),
          content_format: contentFormat,
          lang: lang || 'zh',
          license: license || undefined,
          translation_of: translationOf || undefined,
          tags: selectedTags,
          prereqs,
        });

        // Create extra language versions as translations
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
              tags: selectedTags,
              prereqs,
            });
          } catch (e: any) {
            console.warn(`Failed to create ${lv.lang} translation:`, e);
          }
        }

        window.location.hash = `#/article?uri=${encodeURIComponent(article.at_uri)}`;
      }
    } catch (e: any) {
      error = e.message;
    } finally {
      submitting = false;
    }
  }
</script>

<h1>{isEditing ? t('newArticle.editTitle') : forkSource ? t('newArticle.forkTitle') : t('newArticle.title')}</h1>

{#if forkSource}
  <p class="fork-hint">{t('newArticle.forkHint')}</p>
{/if}

{#if error}
  <p class="error-msg">{error}</p>
{/if}

<div class="form-group">
  <label for="title">{t('newArticle.titleLabel')}</label>
  <input id="title" bind:value={title} placeholder={t('newArticle.titleLabel')} />
</div>

<div class="form-group">
  <label for="description">{t('newArticle.descLabel')}</label>
  <input id="description" bind:value={description} placeholder={t('newArticle.descPlaceholder')} />
</div>

<div class="form-row">
  <div class="form-group" style="flex:1">
    <label for="lang">{t('newArticle.langLabel')}</label>
    <select id="lang" bind:value={lang}>
      <option value="zh">中文</option>
      <option value="en">English</option>
      <option value="ja">日本語</option>
      <option value="ko">한국어</option>
      <option value="fr">Français</option>
      <option value="de">Deutsch</option>
    </select>
  </div>
  <div class="form-group" style="flex:1">
    <label for="license">{t('newArticle.licenseLabel')}</label>
    <select id="license" bind:value={license}>
      <option value="CC-BY-NC-SA-4.0">CC BY-NC-SA 4.0</option>
      <option value="CC-BY-SA-4.0">CC BY-SA 4.0</option>
      <option value="CC-BY-4.0">CC BY 4.0</option>
      <option value="CC-BY-NC-4.0">CC BY-NC 4.0</option>
      <option value="CC-BY-NC-ND-4.0">CC BY-NC-ND 4.0</option>
      <option value="CC0-1.0">CC0 (Public Domain)</option>
      <option value="MIT">MIT</option>
      <option value="Apache-2.0">Apache 2.0</option>
      <option value="GFDL-1.3">GFDL 1.3</option>
      <option value="All-Rights-Reserved">All Rights Reserved</option>
    </select>
  </div>
  <div class="form-group" style="flex:2">
    <label for="translation-of">{t('newArticle.translationOf')}</label>
    <select id="translation-of" bind:value={translationOf}>
      <option value="">{t('newArticle.originalArticle')}</option>
      {#each allArticles as a}
        <option value={a.at_uri}>[{a.lang}] {a.title}</option>
      {/each}
    </select>
  </div>
</div>

<div class="form-row">
  <div class="form-group" style="flex:1">
    <label for="format">{t('newArticle.formatLabel')}</label>
    <select id="format" bind:value={contentFormat}>
      <option value="typst">Typst</option>
      <option value="markdown">Markdown + KaTeX</option>
      <option value="html">HTML</option>
    </select>
  </div>
</div>

<div class="form-group">
  <div class="content-label-row">
    <label for="content">{t('newArticle.contentLabel')} ({contentFormat === 'markdown' ? 'Markdown' : contentFormat === 'html' ? 'HTML' : 'Typst'})</label>
    <div class="upload-btns">
      <label class="upload-btn" class:disabled={loadingFile}>
        <input type="file" accept=".md,.markdown,.typ,.typst,.html,.htm" onchange={handleFileLoad} hidden />
        {loadingFile ? t('newArticle.readingFile') : t('newArticle.uploadFile')}
      </label>
      <label class="upload-btn" class:disabled={uploadingImage}>
        <input type="file" accept="image/*" onchange={handleImageUpload} hidden />
        {uploadingImage ? t('newArticle.uploading') : t('newArticle.uploadImage')}
      </label>
    </div>
  </div>
  <textarea id="content" bind:value={content} placeholder={contentFormat === 'markdown' ? '# My Article\n\nSome text with $x^2$ math' : contentFormat === 'html' ? '<!DOCTYPE html>\n<html>\n<body>\n  <h1>My Article</h1>\n</body>\n</html>' : '= My Article'}></textarea>
</div>

{#if !isEditing && !forkSource}
<div class="form-group">
  <div class="lang-versions-header">
    <span class="form-label">{t('newArticle.langVersions')}</span>
    <button type="button" class="btn-add-lang" onclick={addLangVersion}>
      + {t('newArticle.addLangVersion')}
    </button>
  </div>
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
        <label class="upload-btn">
          <input type="file" accept=".md,.markdown,.typ,.typst,.html,.htm" onchange={(e) => handleLangFileLoad(idx, e)} hidden />
          {t('newArticle.uploadFile')}
        </label>
        <button type="button" class="lang-remove" onclick={() => removeLangVersion(idx)}>&times;</button>
      </div>
      <textarea
        bind:value={extraLangs[idx].content}
        placeholder={t('newArticle.versionContent', lv.lang)}
        class="lang-textarea"
      ></textarea>
    </div>
  {/each}
</div>
{/if}

<div class="form-group">
  <label for="tag-input">{t('newArticle.tagsLabel')}</label>
  <div class="tag-input-row">
    <input
      id="tag-input"
      type="text"
      bind:value={newTagInput}
      placeholder={t('newArticle.tagInput')}
      onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addNewTag(); } }}
      onfocus={() => showTagSuggestions = true}
      onblur={() => setTimeout(() => showTagSuggestions = false, 200)}
      oninput={() => showTagSuggestions = true}
    />
    <button type="button" class="tag-add-btn" onclick={addNewTag}>{t('common.add')}</button>
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
        <span class="tag lit">{getTagName(tagId)} <button type="button" class="tag-remove" onclick={() => toggleTag(tagId)}>&times;</button></span>
      {/each}
    </div>
  {/if}
  <div class="tag-picker">
    {#each tags.filter(t => !selectedTags.includes(t.id)).slice(0, 20) as t}
      <button
        type="button"
        class="tag"
        onclick={() => toggleTag(t.id)}
      >{t.name}</button>
    {/each}
  </div>
</div>

<div class="form-group">
  <label for="prereq-select">{t('newArticle.prereqsLabel')}</label>
  <p class="form-hint">{t('newArticle.prereqsHint')}</p>

  {#if prereqs.length > 0}
    <div class="prereq-list">
      {#each prereqs as p}
        <div class="prereq-item">
          <span class="tag {p.prereq_type}">{getTagName(p.tag_id)}</span>
          <span class="prereq-type-label">{p.prereq_type}</span>
          <button class="prereq-remove" onclick={() => removePrereq(p.tag_id)} title={t('common.remove')}>&times;</button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="prereq-add">
    <select id="prereq-select" bind:value={prereqTagId}>
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
</div>

{#if showDiff && forkSource}
  <div class="diff-preview">
    <div class="diff-header">
      <h3>{t('newArticle.diffPreview')}</h3>
      <button class="diff-close" onclick={() => showDiff = false}>{t('newArticle.backToEdit')}</button>
    </div>
    {#if !hasChanges}
      <p class="diff-empty">{t('newArticle.noChanges')}</p>
    {:else}
      <div class="diff-stats">
        <span class="diff-add">+{diffLines.filter(l => l.type === 'add').length}</span>
        <span class="diff-del">-{diffLines.filter(l => l.type === 'del').length}</span>
      </div>
      <pre class="diff-content">{#each diffHunks as hunk}{#if hunk.collapsed}<span class="line-collapse">⋯ {t('newArticle.linesUnchanged', hunk.collapsed)} ⋯</span>
{:else}{#each hunk.lines as line}{#if line.type === 'add'}<span class="line-add">+{line.text}</span>
{:else if line.type === 'del'}<span class="line-del">-{line.text}</span>
{:else}<span class="line-same"> {line.text}</span>
{/if}{/each}{/if}{/each}</pre>
    {/if}
    <button class="btn btn-primary" onclick={submit} disabled={submitting || !hasChanges}>
      {submitting ? t('newArticle.publishing') : t('newArticle.confirmFork')}
    </button>
  </div>
{:else}
  <div class="submit-row">
    {#if !isEditing}
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
    {#if forkSource}
      <button class="btn btn-secondary" onclick={previewDiff}>
        {t('newArticle.previewDiff')}
      </button>
    {/if}
    <button class="btn btn-primary" onclick={forkSource ? previewDiff : submit} disabled={submitting}>
      {submitting ? t('newArticle.publishing') : t('newArticle.publish')}
    </button>
  </div>
{/if}

<style>
  .content-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }
  .content-label-row label:first-child { margin-bottom: 0; }
  .upload-btns {
    display: flex;
    gap: 6px;
  }
  .upload-btn {
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    padding: 3px 10px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    transition: all 0.15s;
  }
  .upload-btn:hover { background: rgba(95,155,101,0.08); }
  .upload-btn.disabled { opacity: 0.5; pointer-events: none; }
  .fork-hint {
    font-size: 14px;
    color: var(--accent);
    margin: 0 0 16px;
  }
  .form-row {
    display: flex;
    gap: 12px;
  }
  .error-msg {
    color: #dc2626;
    margin-bottom: 1rem;
  }
  .form-group {
    margin-bottom: 1.25rem;
  }
  .form-hint {
    font-size: 12px;
    color: var(--text-hint);
    margin: 2px 0 8px;
  }
  .tag-input-row {
    position: relative;
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
  }
  .tag-input-row input {
    flex: 1;
    padding: 6px 10px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
  }
  .tag-add-btn {
    padding: 6px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }
  .tag-suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 60px;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10;
    max-height: 200px;
    overflow-y: auto;
  }
  .tag-suggestions button {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }
  .tag-suggestions button:hover { background: var(--bg-gray, #f5f5f5); }
  .sg-id { color: var(--text-hint); font-size: 11px; }
  .selected-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .selected-tags .tag { display: inline-flex; align-items: center; gap: 4px; }
  .tag-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: inherit;
    padding: 0;
    line-height: 1;
    opacity: 0.6;
  }
  .tag-remove:hover { opacity: 1; }
  .tag-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .tag-picker .tag {
    cursor: pointer;
  }

  .prereq-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  .prereq-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-hover);
    border-radius: 3px;
  }
  .prereq-type-label {
    font-size: 12px;
    color: var(--text-hint);
    margin-left: auto;
  }
  .prereq-remove {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 16px;
    padding: 0 4px;
    line-height: 1;
    transition: color 0.15s;
  }
  .prereq-remove:hover {
    color: #dc2626;
  }

  .prereq-add {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .prereq-add select {
    padding: 5px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
    font-family: var(--font-sans);
  }
  .prereq-add-btn {
    padding: 5px 12px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .prereq-add-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Language versions */
  .lang-versions-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .lang-versions-header .form-label { margin-bottom: 0; font-weight: 500; }
  .btn-add-lang {
    font-size: 12px;
    color: var(--accent);
    background: none;
    border: 1px dashed var(--accent);
    border-radius: 3px;
    padding: 3px 10px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-add-lang:hover { background: rgba(95,155,101,0.08); }
  .lang-version-block {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    margin-bottom: 10px;
    background: var(--bg-hover, #fafafa);
  }
  .lang-version-header {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: 8px;
  }
  .lang-version-header select {
    padding: 4px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
  }
  .lang-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 18px;
    color: var(--text-hint);
    padding: 0 4px;
    line-height: 1;
    margin-left: auto;
  }
  .lang-remove:hover { color: #dc2626; }
  .lang-textarea {
    width: 100%;
    min-height: 150px;
    padding: 8px;
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    resize: vertical;
  }

  /* Submit row */
  .submit-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .btn-draft {
    padding: 8px 20px;
    font-size: 14px;
    border: 1px dashed var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-draft:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-draft:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary {
    padding: 8px 20px;
    font-size: 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-secondary:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  /* Diff preview */
  .diff-preview {
    margin-top: 1rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 16px;
    background: var(--bg-white);
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
  .diff-close {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 12px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .diff-close:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .diff-stats {
    font-size: 13px;
    margin-bottom: 8px;
    display: flex;
    gap: 12px;
  }
  .diff-add { color: #22863a; }
  .diff-del { color: #cb2431; }
  .diff-empty {
    color: var(--text-hint);
    text-align: center;
    padding: 2rem 0;
  }
  .diff-content {
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
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
  .diff-content :global(.line-add) {
    display: block;
    background: #e6ffec;
    color: #22863a;
    padding: 0 8px;
  }
  .diff-content :global(.line-del) {
    display: block;
    background: #ffebe9;
    color: #cb2431;
    padding: 0 8px;
  }
  .diff-content :global(.line-same) {
    display: block;
    padding: 0 8px;
    color: var(--text-secondary);
  }
  .diff-content :global(.line-collapse) {
    display: block;
    padding: 4px 8px;
    color: var(--text-hint);
    background: var(--bg-hover);
    text-align: center;
    font-style: italic;
    font-size: 12px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    margin: 2px 0;
  }
</style>
