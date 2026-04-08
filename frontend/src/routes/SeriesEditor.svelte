<script lang="ts">
  import { getSeries, listSeriesFiles, readSeriesFile, writeSeriesFile, deleteSeriesFile, compileSeries, addSeriesPrereq, removeSeriesPrereq } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { SeriesDetail, SeriesArticle } from '../lib/types';
  import MarkdownEditor from '../lib/components/MarkdownEditor.svelte';
  import TypstEditor from '../lib/components/TypstEditor.svelte';

  let { id } = $props<{ id: string }>();

  let detail = $state<SeriesDetail | null>(null);
  let files = $state<{ path: string; size: number }[]>([]);
  let activeFile = $state<string | null>(null);
  let editorContent = $state('');
  let originalContent = $state('');
  let dirty = $state(false);
  let loading = $state(true);
  let saving = $state(false);
  let compiling = $state(false);
  let compileResult = $state<{ articles_created: number; articles_updated: number; total_headings: number } | null>(null);
  let compileError = $state('');
  let newFileName = $state('');
  let showNewFile = $state(false);
  let error = $state('');

  // Prereq editing
  let activeTab = $state<'editor' | 'prereqs'>('editor');
  let prereqFrom = $state('');
  let prereqTo = $state('');

  $effect(() => { loadSeries(); });

  async function loadSeries() {
    loading = true;
    try {
      const [d, f] = await Promise.all([getSeries(id), listSeriesFiles(id)]);
      detail = d;
      files = f;
      // Open first file by default
      if (f.length > 0 && !activeFile) {
        await openFile(f[0].path);
      }
    } catch (e: any) {
      error = e.message || 'Failed to load';
    }
    loading = false;
  }

  async function openFile(path: string) {
    if (dirty && activeFile) {
      if (!confirm(t('seriesEditor.unsavedChanges'))) return;
    }
    activeFile = path;
    try {
      const content = await readSeriesFile(id, path);
      editorContent = content;
      originalContent = content;
      dirty = false;
    } catch {
      editorContent = '';
      originalContent = '';
    }
  }

  $effect(() => {
    dirty = editorContent !== originalContent;
  });

  async function save() {
    if (!activeFile || !dirty) return;
    saving = true;
    try {
      await writeSeriesFile(id, activeFile, editorContent);
      originalContent = editorContent;
      dirty = false;
    } catch (e: any) {
      error = e.message;
    }
    saving = false;
  }

  async function compile() {
    if (dirty) await save();
    compiling = true;
    compileError = '';
    compileResult = null;
    try {
      compileResult = await compileSeries(id);
      // Reload series to reflect new articles
      detail = await getSeries(id);
    } catch (e: any) {
      compileError = e.message || 'Compile failed';
    }
    compiling = false;
  }

  async function createFile() {
    let name = newFileName.trim();
    if (!name) return;
    // Add default extension if missing
    if (!name.includes('.')) name += '.md';
    const path = name.includes('/') ? name : `chapters/${name}`;
    try {
      await writeSeriesFile(id, path, '', `Create ${path}`);
      files = await listSeriesFiles(id);
      newFileName = '';
      showNewFile = false;
      await openFile(path);
    } catch (e: any) {
      error = e.message;
    }
  }

  async function deleteFile(path: string) {
    if (!confirm(t('seriesEditor.confirmDelete', path))) return;
    try {
      await deleteSeriesFile(id, path);
      files = files.filter(f => f.path !== path);
      if (activeFile === path) {
        activeFile = null;
        editorContent = '';
        dirty = false;
        if (files.length > 0) await openFile(files[0].path);
      }
    } catch (e: any) {
      error = e.message;
    }
  }

  function ext(path: string) {
    return path.split('.').pop() ?? '';
  }

  function fileIcon(path: string) {
    const e = ext(path);
    if (e === 'md') return '📝';
    if (e === 'typ') return '🔤';
    if (e === 'bib') return '📚';
    return '📄';
  }

  // Prereq helpers
  function prereqsFor(articleUri: string): SeriesArticle[] {
    if (!detail) return [];
    return detail.prereqs
      .filter(p => p.article_uri === articleUri)
      .map(p => detail!.articles.find(a => a.article_uri === p.prereq_article_uri)!)
      .filter(Boolean);
  }

  async function addPrereq() {
    if (!prereqFrom || !prereqTo || prereqFrom === prereqTo) return;
    await addSeriesPrereq(id, prereqFrom, prereqTo);
    detail = await getSeries(id);
    prereqFrom = '';
    prereqTo = '';
  }

  async function delPrereq(articleUri: string, prereqUri: string) {
    await removeSeriesPrereq(id, articleUri, prereqUri);
    detail = await getSeries(id);
  }

  // Keyboard shortcut: Ctrl+S / Cmd+S
  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      save();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if error && !detail}
  <p class="error">{error}</p>
{:else if detail}
  <div class="editor-layout">
    <!-- Sidebar: file tree -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="series-name">{detail.series.title}</span>
        <a href="#/series?id={encodeURIComponent(id)}" class="view-link" title={t('seriesEditor.viewSeries')}>↗</a>
      </div>

      <div class="file-tree">
        {#each files as f}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="file-item"
            class:active={activeFile === f.path}
            onclick={() => openFile(f.path)}
            role="button"
            tabindex="0"
            onkeydown={(e) => { if (e.key === 'Enter') openFile(f.path); }}
          >
            <span class="file-icon">{fileIcon(f.path)}</span>
            <span class="file-name">{f.path.replace('chapters/', '')}</span>
            <button class="del-btn" onclick={(e) => { e.stopPropagation(); deleteFile(f.path); }} title={t('common.delete')}>×</button>
          </div>
        {/each}

        {#if showNewFile}
          <div class="new-file-row">
            <input
              type="text"
              bind:value={newFileName}
              placeholder="01-intro.md"
              onkeydown={(e) => { if (e.key === 'Enter') createFile(); if (e.key === 'Escape') { showNewFile = false; } }}
              autofocus
            />
            <button onclick={createFile}>{t('common.add')}</button>
          </div>
        {:else}
          <button class="add-file-btn" onclick={() => { showNewFile = true; }}>+ {t('seriesEditor.newFile')}</button>
        {/if}
      </div>

      <!-- Compile -->
      <div class="compile-section">
        <button class="compile-btn" onclick={compile} disabled={compiling}>
          {compiling ? t('seriesEditor.compiling') : t('seriesEditor.compile')}
        </button>
        {#if compileResult}
          <p class="compile-ok">
            ✓ {compileResult.total_headings} 节，{compileResult.articles_created} 篇新建，{compileResult.articles_updated} 篇更新
          </p>
        {/if}
        {#if compileError}
          <p class="compile-error">{compileError}</p>
        {/if}
      </div>
    </aside>

    <!-- Main editor -->
    <main class="editor-pane">
      <!-- Tab bar -->
      <div class="tab-bar">
        <button class="tab" class:active={activeTab === 'editor'} onclick={() => { activeTab = 'editor'; }}>
          {t('seriesEditor.tabEditor')}
        </button>
        <button class="tab" class:active={activeTab === 'prereqs'} onclick={() => { activeTab = 'prereqs'; }}>
          {t('seriesEditor.tabPrereqs')}
          {#if detail.prereqs.length > 0}<span class="tab-count">{detail.prereqs.length}</span>{/if}
        </button>
      </div>

      {#if activeTab === 'editor'}
        {#if activeFile}
          <div class="editor-toolbar">
            <span class="current-file">{activeFile}</span>
            {#if dirty}
              <span class="dirty-dot" title={t('seriesEditor.unsaved')}>●</span>
            {/if}
            <button class="save-btn" onclick={save} disabled={saving || !dirty}>
              {saving ? t('seriesEditor.saving') : t('seriesEditor.save')}
            </button>
          </div>
          <div class="wysiwyg-wrap">
            {#if ext(activeFile) === 'md'}
              <MarkdownEditor bind:value={editorContent} fillHeight={true} />
            {:else if ext(activeFile) === 'typ'}
              <TypstEditor bind:value={editorContent} fillHeight={true} />
            {:else}
              <textarea
                class="code-editor"
                value={editorContent}
                oninput={onInput}
                spellcheck="false"
                autocomplete="off"
              ></textarea>
            {/if}
          </div>
        {:else}
          <div class="no-file">
            <p>{t('seriesEditor.noFile')}</p>
          </div>
        {/if}
      {:else}
        <!-- Prereq editor -->
        <div class="prereq-pane">
          {#if detail.articles.length === 0}
            <p class="hint">{t('seriesEditor.prereqHint')}</p>
          {:else}
            <div class="prereq-add">
              <select bind:value={prereqFrom}>
                <option value="">{t('seriesEditor.prereqSelectArticle')}</option>
                {#each detail.articles as a}
                  <option value={a.article_uri}>{a.title}</option>
                {/each}
              </select>
              <span class="arrow">需要先读</span>
              <select bind:value={prereqTo}>
                <option value="">{t('seriesEditor.prereqSelectPrereq')}</option>
                {#each detail.articles as a}
                  <option value={a.article_uri}>{a.title}</option>
                {/each}
              </select>
              <button class="add-prereq-btn" onclick={addPrereq} disabled={!prereqFrom || !prereqTo || prereqFrom === prereqTo}>
                {t('common.add')}
              </button>
            </div>

            <div class="prereq-list">
              {#each detail.articles as article}
                {@const pqs = prereqsFor(article.article_uri)}
                {#if pqs.length > 0}
                  <div class="prereq-group">
                    <span class="prereq-article">{article.title}</span>
                    <span class="prereq-needs">需要先读：</span>
                    {#each pqs as pq}
                      <span class="prereq-tag">
                        {pq.title}
                        <button class="rm-prereq" onclick={() => delPrereq(article.article_uri, pq.article_uri)}>×</button>
                      </span>
                    {/each}
                  </div>
                {/if}
              {/each}
              {#if detail.prereqs.length === 0}
                <p class="hint">{t('seriesEditor.noPrereqs')}</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </main>
  </div>
{/if}

<style>
  .editor-layout {
    display: flex;
    height: calc(100vh - 120px);
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  /* Sidebar */
  .sidebar {
    width: 220px;
    flex-shrink: 0;
    background: var(--bg-white);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-header {
    padding: 12px 12px 8px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .series-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .view-link {
    font-size: 13px;
    color: var(--text-hint);
    text-decoration: none;
    flex-shrink: 0;
  }
  .view-link:hover { color: var(--accent); }

  .file-tree {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }
  .file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 10px 5px 12px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    transition: background 0.1s;
  }
  .file-item:hover { background: rgba(95,155,101,0.06); }
  .file-item.active {
    background: rgba(95,155,101,0.1);
    color: var(--accent);
  }
  .file-icon { flex-shrink: 0; font-size: 12px; }
  .file-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .del-btn {
    opacity: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 14px;
    padding: 0 2px;
    flex-shrink: 0;
  }
  .file-item:hover .del-btn { opacity: 1; }
  .del-btn:hover { color: #c33; }

  .new-file-row {
    display: flex;
    gap: 4px;
    padding: 4px 8px;
  }
  .new-file-row input {
    flex: 1;
    padding: 4px 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    min-width: 0;
  }
  .new-file-row button {
    padding: 4px 8px;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .add-file-btn {
    width: 100%;
    padding: 6px 12px;
    border: none;
    background: none;
    text-align: left;
    font-size: 12px;
    color: var(--text-hint);
    cursor: pointer;
  }
  .add-file-btn:hover { color: var(--accent); }

  .compile-section {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }
  .compile-btn {
    width: 100%;
    padding: 7px 0;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .compile-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .compile-ok {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--accent);
  }
  .compile-error {
    margin: 6px 0 0;
    font-size: 12px;
    color: #c33;
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tab {
    padding: 7px 16px;
    font-size: 13px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .tab-count {
    font-size: 11px;
    background: rgba(95,155,101,0.15);
    color: var(--accent);
    padding: 1px 5px;
    border-radius: 10px;
  }

  /* Prereq pane */
  .prereq-pane {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }
  .prereq-add {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 20px;
    flex-wrap: wrap;
  }
  .prereq-add select {
    padding: 6px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    max-width: 220px;
  }
  .arrow { font-size: 13px; color: var(--text-hint); }
  .add-prereq-btn {
    padding: 6px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .add-prereq-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .prereq-list { display: flex; flex-direction: column; gap: 10px; }
  .prereq-group {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .prereq-article { font-weight: 500; color: var(--text-primary); }
  .prereq-needs { color: var(--text-hint); }
  .prereq-tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: rgba(95,155,101,0.1);
    color: var(--accent);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 12px;
  }
  .rm-prereq {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 13px;
    padding: 0;
    line-height: 1;
  }
  .rm-prereq:hover { color: #c33; }
  .hint { font-size: 13px; color: var(--text-hint); }

  /* Editor pane */
  .editor-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-white);
  }
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    flex-shrink: 0;
  }
  .current-file {
    font-size: 12px;
    color: var(--text-hint);
    font-family: var(--font-mono, monospace);
  }
  .dirty-dot {
    color: var(--accent);
    font-size: 10px;
  }
  .save-btn {
    margin-left: auto;
    padding: 4px 12px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .save-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .save-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .wysiwyg-wrap {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .code-editor {
    flex: 1;
    width: 100%;
    padding: 16px;
    font-family: var(--font-mono, 'Fira Code', 'Consolas', monospace);
    font-size: 14px;
    line-height: 1.6;
    border: none;
    resize: none;
    outline: none;
    background: var(--bg-white);
    color: var(--text-primary);
    box-sizing: border-box;
    tab-size: 2;
  }

  .no-file {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-hint);
    font-size: 14px;
  }
</style>
