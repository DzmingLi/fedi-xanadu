<script lang="ts">
  import {
    getSeries, listSeriesFiles, readSeriesFile, writeSeriesFile, deleteSeriesFile,
    compileSeries, addSeriesPrereq, removeSeriesPrereq,
    listCollaborators, inviteCollaborator, removeCollaborator,
    listChannels, readChannelFile, writeChannelFile,
    channelDiff, applyChannelChange,
  } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { SeriesDetail, SeriesArticle } from '../lib/types';
  import MarkdownEditor from 'pijul-editor/MarkdownEditor.svelte';
  import TypstEditor from 'pijul-editor/TypstEditor.svelte';
  import JsonView from 'pijul-editor/JsonView.svelte';
  import ChannelPanel from 'pijul-editor/ChannelPanel.svelte';
  import FilePanel from 'pijul-editor/FilePanel.svelte';

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
  let error = $state('');

  // Channel state
  let channels = $state<string[]>(['main']);
  let currentChannel = $state('main');
  let channelReadOnly = $derived((() => {
    // Owner can write to main; others only to their own channel
    const auth = getAuth();
    if (!auth) return true;
    // Will be set after collaborators load
    return false;
  })());

  // Panel toggles (mirrors NewArticle)
  let filePanelOpen = $state(true);
  let settingsPanelOpen = $state(true);

  // Prereqs tab
  let activeTab = $state<'compile' | 'prereqs' | 'collab'>('compile');
  let prereqFrom = $state('');
  let prereqTo = $state('');

  $effect(() => { loadSeries(); });

  async function loadSeries() {
    loading = true;
    try {
      const [d, f, chs, collabs] = await Promise.all([
        getSeries(id),
        listSeriesFiles(id),
        listChannels(id).catch(() => ['main']),
        listCollaborators(id).catch(() => []),
      ]);
      detail = d;
      files = f;
      channels = chs;

      // Set current channel to user's own channel
      const auth = getAuth();
      if (auth) {
        const myCollab = collabs.find(c => c.user_did === auth.did);
        if (myCollab) currentChannel = myCollab.channel_name;
      }

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
      let content: string;
      if (currentChannel === 'main') {
        content = await readSeriesFile(id, path);
      } else {
        const res = await readChannelFile(id, currentChannel, path);
        content = res.content;
      }
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
      if (currentChannel === 'main') {
        await writeSeriesFile(id, activeFile, editorContent);
      } else {
        await writeChannelFile(id, currentChannel, activeFile, editorContent);
      }
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
      detail = await getSeries(id);
    } catch (e: any) {
      compileError = e.message || 'Compile failed';
    }
    compiling = false;
  }

  async function createFile(path: string) {
    if (currentChannel === 'main') {
      await writeSeriesFile(id, path, '', `Create ${path}`);
    } else {
      await writeChannelFile(id, currentChannel, path, '', `Create ${path}`);
    }
    files = await listSeriesFiles(id);
    await openFile(path);
  }

  function switchChannel(ch: string) {
    if (dirty && !confirm(t('seriesEditor.unsavedChanges'))) return;
    currentChannel = ch;
    dirty = false;
    if (activeFile) openFile(activeFile);
  }

  async function doDeleteFile(path: string) {
    await deleteSeriesFile(id, path);
    files = files.filter(f => f.path !== path);
    if (activeFile === path) {
      activeFile = null;
      editorContent = '';
      dirty = false;
      if (files.length > 0) await openFile(files[0].path);
    }
  }

  async function doReorderFiles(paths: string[]) {
    // Rename files with numeric prefixes to reflect new order
    for (let i = 0; i < paths.length; i++) {
      const oldPath = paths[i];
      const fileName = oldPath.replace(/^chapters\//, '');
      // Strip existing numeric prefix (e.g. "01-" or "1-")
      const baseName = fileName.replace(/^\d+-/, '');
      const prefix = String(i + 1).padStart(2, '0');
      const newPath = `chapters/${prefix}-${baseName}`;
      if (oldPath !== newPath) {
        // Read content, write to new path, delete old
        const content = currentChannel === 'main'
          ? await readSeriesFile(id, oldPath)
          : (await readChannelFile(id, currentChannel, oldPath)).content;
        if (currentChannel === 'main') {
          await writeSeriesFile(id, newPath, content);
        } else {
          await writeChannelFile(id, currentChannel, newPath, content);
        }
        await deleteSeriesFile(id, oldPath);
        if (activeFile === oldPath) activeFile = newPath;
      }
    }
    files = await listSeriesFiles(id);
  }

  function ext(path: string) {
    return path.split('.').pop() ?? '';
  }

  // Prereqs
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

  // Keyboard shortcut
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
  <div class="editor-page">

    <!-- Title area -->
    <div class="editor-title-area">
      <div class="title-row">
        <span class="series-title">{detail.series.title}</span>
        <a href="#/series?id={encodeURIComponent(id)}" class="view-link">↗ {t('seriesEditor.viewSeries')}</a>
      </div>
      {#if activeFile}
        <div class="active-file-row">
          <span class="active-file-name">{activeFile}</span>
          {#if currentChannel !== 'main'}
            <span class="channel-badge">{currentChannel}</span>
          {/if}
          {#if dirty}<span class="dirty-dot">●</span>{/if}
        </div>
      {/if}
    </div>

    <!-- Body: file panel + editor + settings panel -->
    <div class="editor-body">

      <!-- Left: File Tree Panel -->
      {#if filePanelOpen}
        <aside class="file-panel">
          <div class="panel-header">
            <span class="panel-label">{t('seriesEditor.files')}</span>
          </div>
          <FilePanel
            files={files.map(f => ({ path: f.path, is_dir: false }))}
            {activeFile}
            sortable={true}
            onSelect={openFile}
            onCreate={createFile}
            onDelete={doDeleteFile}
            onReorder={doReorderFiles}
          />
        </aside>
      {/if}

      <!-- Center: Editor -->
      <div class="editor-main">
        <!-- Left toggle -->
        <button
          class="panel-tab panel-tab-left"
          class:open={filePanelOpen}
          onclick={() => { filePanelOpen = !filePanelOpen; }}
          title={t('seriesEditor.files')}
        >
          <svg width="12" height="22" viewBox="0 0 10 18" fill="currentColor">
            {#if filePanelOpen}
              <polyline points="8,2 2,9 8,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {:else}
              <polyline points="2,2 8,9 2,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {/if}
          </svg>
        </button>

        <div class="editor-content">
          {#if activeFile}
            {#if ext(activeFile) === 'json'}
              <JsonView value={editorContent} />
            {:else if ext(activeFile) === 'md'}
              <MarkdownEditor bind:value={editorContent} fillHeight={true} />
            {:else if ext(activeFile) === 'typ'}
              <TypstEditor bind:value={editorContent} fillHeight={true} />
            {:else}
              <textarea class="editor-textarea" bind:value={editorContent} spellcheck="false"></textarea>
            {/if}
          {:else}
            <div class="no-file"><p>{t('seriesEditor.noFile')}</p></div>
          {/if}
        </div>

        <!-- Right toggle -->
        <button
          class="panel-tab panel-tab-right"
          class:open={settingsPanelOpen}
          onclick={() => { settingsPanelOpen = !settingsPanelOpen; }}
          title={t('editor.settings')}
        >
          <svg width="12" height="22" viewBox="0 0 10 18" fill="currentColor">
            {#if settingsPanelOpen}
              <polyline points="2,2 8,9 2,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {:else}
              <polyline points="8,2 2,9 8,16" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            {/if}
          </svg>
        </button>
      </div>

      <!-- Right: Settings Panel -->
      {#if settingsPanelOpen}
        <aside class="settings-panel">
          <!-- Tab bar -->
          <div class="sp-tabs">
            <button class="sp-tab" class:active={activeTab === 'compile'} onclick={() => { activeTab = 'compile'; }}>
              {t('seriesEditor.compile')}
            </button>
            <button class="sp-tab" class:active={activeTab === 'prereqs'} onclick={() => { activeTab = 'prereqs'; }}>
              {t('seriesEditor.tabPrereqs')}
            </button>
            <button class="sp-tab" class:active={activeTab === 'collab'} onclick={() => { activeTab = 'collab'; }}>
              协作
            </button>
          </div>

          {#if activeTab === 'compile'}
            <div class="sp-body">
              <p class="sp-hint">{t('seriesEditor.compileHint')}</p>
              <button class="compile-btn" onclick={compile} disabled={compiling}>
                {compiling ? t('seriesEditor.compiling') : t('seriesEditor.compile')}
              </button>
              {#if compileResult}
                <p class="compile-ok">✓ {compileResult.total_headings} {t('seriesEditor.compileHeadings')}，{compileResult.articles_created} {t('seriesEditor.compileCreated')}，{compileResult.articles_updated} {t('seriesEditor.compileUpdated')}</p>
              {/if}
              {#if compileError}
                <p class="compile-error">{compileError}</p>
              {/if}
            </div>
          {:else if activeTab === 'collab'}
            <div class="sp-body">
              <ChannelPanel
                {currentChannel}
                {channels}
                currentUserDid={getAuth()?.did || ''}
                onChannelChange={switchChannel}
                fetchCollaborators={() => listCollaborators(id)}
                doInvite={(did) => inviteCollaborator(id, did).then(() => {})}
                doRemove={(did) => removeCollaborator(id, did)}
                fetchDiff={(target, current) => channelDiff(id, target, current)}
                doApply={(target, source, hash) => applyChannelChange(id, target, source, hash)}
              />
            </div>
          {:else}
            <div class="sp-body">
              {#if detail.articles.length === 0}
                <p class="sp-hint">{t('seriesEditor.prereqHint')}</p>
              {:else}
                <div class="prereq-add">
                  <select bind:value={prereqFrom}>
                    <option value="">{t('seriesEditor.prereqSelectArticle')}</option>
                    {#each detail.articles as a}
                      <option value={a.article_uri}>{a.title}</option>
                    {/each}
                  </select>
                  <span class="prereq-arrow">需要先读</span>
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
                        <div class="prereq-tags">
                          {#each pqs as pq}
                            <span class="prereq-tag">
                              {pq.title}
                              <button class="rm-prereq" onclick={() => delPrereq(article.article_uri, pq.article_uri)}>×</button>
                            </span>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  {/each}
                  {#if detail.prereqs.length === 0}
                    <p class="sp-hint">{t('seriesEditor.noPrereqs')}</p>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </aside>
      {/if}
    </div>

    <!-- Footer -->
    <div class="editor-footer">
      <span class="footer-status">
        {#if dirty}
          <span class="status-unsaved">{t('seriesEditor.unsaved')}</span>
        {:else if activeFile}
          <span class="status-saved">{t('version.saved')}</span>
        {/if}
      </span>
      <div class="footer-spacer"></div>
      <button class="btn btn-outline" onclick={save} disabled={saving || !dirty}>
        {saving ? t('seriesEditor.saving') : t('common.save')}
      </button>
    </div>

  </div>
{/if}

<style>
  .editor-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  /* Title area */
  .editor-title-area {
    flex-shrink: 0;
    padding: 8px 16px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
  }
  .title-row {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .series-title {
    font-family: var(--font-serif);
    font-size: 1.3rem;
    color: var(--text-primary);
    font-weight: 400;
  }
  .view-link {
    font-size: 12px;
    color: var(--text-hint);
    text-decoration: none;
  }
  .view-link:hover { color: var(--accent); }
  .active-file-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  .active-file-name {
    font-size: 12px;
    color: var(--text-hint);
    font-family: var(--font-mono, monospace);
  }
  .dirty-dot { color: var(--accent); font-size: 10px; }
  .channel-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--accent);
    color: white;
    font-family: var(--font-mono, monospace);
  }

  /* Body */
  .editor-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* Left: File panel */
  .file-panel {
    width: 220px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-white);
    font-size: 13px;
  }
  .panel-header {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-hover, #fafafa);
  }
  .panel-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-hint);
  }
  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px 5px 14px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: background 0.1s;
    border-radius: 0;
  }
  .file-item:hover { background: rgba(95,155,101,0.06); }
  .file-item.active { background: rgba(95,155,101,0.1); color: var(--accent); }
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
    padding: 6px 14px;
    border: none;
    background: none;
    text-align: left;
    font-size: 12px;
    color: var(--text-hint);
    cursor: pointer;
  }
  .add-file-btn:hover { color: var(--accent); }

  /* Center: Editor */
  .editor-main {
    flex: 1;
    display: flex;
    min-width: 0;
    position: relative;
  }
  .panel-tab {
    width: 14px;
    flex-shrink: 0;
    background: var(--bg-hover, #fafafa);
    border: none;
    border-right: 1px solid var(--border);
    cursor: pointer;
    color: var(--text-hint);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: background 0.15s, color 0.15s;
  }
  .panel-tab-right {
    border-right: none;
    border-left: 1px solid var(--border);
  }
  .panel-tab:hover { background: var(--bg-gray, #f0f0f0); color: var(--accent); }
  .editor-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }
  .editor-textarea {
    flex: 1;
    width: 100%;
    padding: 16px;
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    line-height: 1.6;
    border: none;
    resize: none;
    outline: none;
    background: var(--bg-white);
    color: var(--text-primary);
    box-sizing: border-box;
  }
  .no-file {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-hint);
    font-size: 14px;
  }

  /* Right: Settings panel */
  .settings-panel {
    width: 240px;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-white);
    font-size: 13px;
  }
  .sp-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .sp-tab {
    flex: 1;
    padding: 8px 4px;
    font-size: 12px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
  }
  .sp-tab:hover { color: var(--text-primary); }
  .sp-tab.active { color: var(--accent); border-bottom-color: var(--accent); }
  .sp-body {
    flex: 1;
    overflow-y: auto;
    padding: 14px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .sp-hint {
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.5;
    margin: 0;
  }
  .compile-btn {
    width: 100%;
    padding: 8px 0;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .compile-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .compile-ok { font-size: 12px; color: var(--accent); margin: 0; }
  .compile-error { font-size: 12px; color: #c33; margin: 0; }

  /* Prereq editor */
  .prereq-add {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .prereq-add select {
    width: 100%;
    padding: 5px 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
  }
  .prereq-arrow { font-size: 12px; color: var(--text-hint); text-align: center; }
  .add-prereq-btn {
    padding: 5px 0;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    width: 100%;
  }
  .add-prereq-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .prereq-list { display: flex; flex-direction: column; gap: 8px; }
  .prereq-group { display: flex; flex-direction: column; gap: 3px; }
  .prereq-article { font-weight: 500; font-size: 12px; color: var(--text-primary); }
  .prereq-tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .prereq-tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: rgba(95,155,101,0.1);
    color: var(--accent);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
  }
  .rm-prereq {
    background: none; border: none; cursor: pointer;
    color: var(--text-hint); font-size: 12px; padding: 0; line-height: 1;
  }
  .rm-prereq:hover { color: #c33; }

  /* Footer */
  .editor-footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-white);
  }
  .footer-spacer { flex: 1; }
  .status-unsaved { font-size: 12px; color: var(--text-hint); }
  .status-saved { font-size: 12px; color: var(--accent); }
  .btn {
    padding: 6px 16px;
    font-size: 13px;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: none;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-outline { /* same as .btn */ }
</style>
