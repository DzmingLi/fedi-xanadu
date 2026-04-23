<script lang="ts">
  import {
    getSeries, listSeriesFiles, readSeriesFile, writeSeriesFile, deleteSeriesFile,
    compileSeries, addSeriesPrereq, removeSeriesPrereq,
    listCollaborators, inviteCollaborator, removeCollaborator,
    listChannels, readChannelFile, writeChannelFile,
    channelDiff, applyChannelChange,
    channelLogDetails, getSeriesChangeDetail, unrecordSeriesChange,
  } from '../lib/api';
  import type { ChangeInfo } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { SeriesDetail, SeriesArticle } from '../lib/types';
  import MarkdownEditor from 'pijul-editor/MarkdownEditor.svelte';
  import TypstEditor from 'pijul-editor/TypstEditor.svelte';
  import JsonView from 'pijul-editor/JsonView.svelte';
  import ChannelPanel from 'pijul-editor/ChannelPanel.svelte';
  import VersionPanel from 'pijul-editor/VersionPanel.svelte';
  import FilePanel from 'pijul-editor/FilePanel.svelte';
  import type { DiffLine, VersionInfo } from 'pijul-editor/VersionPanel.svelte';

  let { id } = $props<{ id: string }>();

  let detail = $state<SeriesDetail | null>(null);
  let files = $state<{ path: string; size: number }[]>([]);
  let activeFile = $state<string | null>(null);
  let editorContent = $state('');
  let originalContent = $state('');
  let loading = $state(true);
  let error = $state('');

  // Channel state
  let channels = $state<string[]>(['main']);
  let currentChannel = $state('main');

  // Version panel state
  let recording = $state(false);
  let versions = $state<VersionInfo[]>([]);
  let loadingHistory = $state(false);

  let currentDiffLines = $derived.by((): DiffLine[] => {
    if (editorContent === originalContent) return [];
    const oldLines = originalContent.split('\n');
    const newLines = editorContent.split('\n');
    const lines: DiffLine[] = [];
    for (const l of oldLines) { if (!newLines.includes(l)) lines.push({ type: 'del', text: l }); }
    for (const l of newLines) { if (!oldLines.includes(l)) lines.push({ type: 'add', text: l }); }
    return lines;
  });

  // Compile state
  let compiling = $state(false);
  let compileResult = $state<{ articles_created: number; articles_updated: number; total_headings: number } | null>(null);
  let compileError = $state('');

  // Right panel tab
  let activeTab = $state<'compile' | 'prereqs'>('compile');
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

      const auth = getAuth();
      if (auth) {
        const myCollab = collabs.find(c => c.user_did === auth.did);
        if (myCollab) currentChannel = myCollab.channel_name;
      }

      const firstFile = f.find(item => !(item as any).is_dir);
      if (firstFile && !activeFile) {
        await openFile(firstFile.path);
      }
      await loadHistory();
    } catch (e: any) {
      error = e.message || 'Failed to load';
    }
    loading = false;
  }

  async function openFile(path: string) {
    if (editorContent !== originalContent && activeFile && !confirm(t('seriesEditor.unsavedChanges'))) return;
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
    } catch {
      editorContent = '';
      originalContent = '';
    }
  }

  async function doRecord(message: string) {
    if (!activeFile) return;
    recording = true;
    try {
      if (currentChannel === 'main') {
        await writeSeriesFile(id, activeFile, editorContent, message);
      } else {
        await writeChannelFile(id, currentChannel, activeFile, editorContent, message);
      }
      originalContent = editorContent;
      await loadHistory();
    } catch (e: any) {
      alert('Record failed: ' + (e?.message || e));
    }
    recording = false;
  }

  async function loadHistory() {
    loadingHistory = true;
    try {
      const infos = await channelLogDetails(id, currentChannel);
      versions = infos.map((info: ChangeInfo, idx: number) => ({
        id: idx,
        change_hash: info.hash,
        message: info.message || `Change ${idx + 1}`,
        created_at: '',
        unrecordable: idx === infos.length - 1,
      }));
    } catch { versions = []; }
    loadingHistory = false;
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

  async function doDeleteFile(path: string) {
    await deleteSeriesFile(id, path);
    files = files.filter(f => f.path !== path);
    if (activeFile === path) {
      activeFile = null;
      editorContent = '';
      originalContent = '';
      const next = files.find(f => !(f as any).is_dir);
      if (next) await openFile(next.path);
    }
  }

  function switchChannel(ch: string) {
    if (editorContent !== originalContent && !confirm(t('seriesEditor.unsavedChanges'))) return;
    currentChannel = ch;
    if (activeFile) openFile(activeFile);
    loadHistory();
  }

  async function compile() {
    if (editorContent !== originalContent && activeFile) {
      await writeSeriesFile(id, activeFile, editorContent);
      originalContent = editorContent;
    }
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

  function ext(path: string) { return path.split('.').pop() ?? ''; }

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

  async function reorderChapters(_parentDir: string, paths: string[]) {
    // Update meta.json chapter_order so compile uses the new order
    let meta: Record<string, unknown> = {};
    try {
      const raw = await readSeriesFile(id, 'meta.json');
      meta = JSON.parse(raw);
    } catch { /* no meta.json yet, create one */ }
    meta.chapter_order = paths;
    await writeSeriesFile(id, 'meta.json', JSON.stringify(meta, null, 2), 'reorder chapters');
    // Reorder local file list to match
    const order = new Map(paths.map((p, i) => [p, i]));
    files = [...files].sort((a, b) => (order.get(a.path) ?? 999) - (order.get(b.path) ?? 999));
  }

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      doRecord('update');
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
    <!-- Title -->
    <div class="title-area">
      <span class="series-title">{detail.series.title}</span>
      <a href="/series?id={encodeURIComponent(id)}" class="view-link">↗ {t('seriesEditor.viewSeries')}</a>
      {#if currentChannel !== 'main'}
        <span class="channel-badge">{currentChannel}</span>
      {/if}
      {#if editorContent !== originalContent}<span class="dirty-dot">●</span>{/if}
    </div>

    <div class="editor-body">
      <!-- Left: Version panel -->
      <aside class="version-panel">
        <VersionPanel
          {versions}
          {loadingHistory}
          {recording}
          onRecord={doRecord}
          onUnrecord={async (v) => {
            await unrecordSeriesChange(id, v.change_hash);
            if (activeFile) await openFile(activeFile);
            await loadHistory();
          }}
          onFetchDiff={async (v) => {
            const detail = await getSeriesChangeDetail(id, v.change_hash);
            return detail.lines.map(l => ({ type: l.kind === 'add' ? 'add' as const : l.kind === 'del' ? 'del' as const : 'same' as const, text: l.content }));
          }}
        />
      </aside>

      <!-- File panel -->
      <aside class="file-panel">
        <FilePanel
          files={files.map(f => ({ path: f.path, is_dir: false }))}
          {activeFile}
          sortable={true}
          onSelect={openFile}
          onCreate={createFile}
          onDelete={doDeleteFile}
          onReorder={reorderChapters}
        />
      </aside>

      <!-- Editor -->
      <div class="editor-main">
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

      <!-- Right panel: tabs for compile/prereqs + channel panel below -->
      <aside class="right-panel">
        <div class="sp-tabs">
          <button class="sp-tab" class:active={activeTab === 'compile'} onclick={() => { activeTab = 'compile'; }}>
            {t('seriesEditor.compile')}
          </button>
          <button class="sp-tab" class:active={activeTab === 'prereqs'} onclick={() => { activeTab = 'prereqs'; }}>
            {t('seriesEditor.tabPrereqs')}
          </button>
        </div>

        {#if activeTab === 'compile'}
          <div class="sp-body">
            <p class="sp-hint">{t('seriesEditor.compileHint')}</p>
            <button class="compile-btn" onclick={compile} disabled={compiling}>
              {compiling ? t('seriesEditor.compiling') : t('seriesEditor.compile')}
            </button>
            {#if compileResult}
              <p class="compile-ok">{compileResult.total_headings} {t('seriesEditor.compileHeadings')}, {compileResult.articles_created} {t('seriesEditor.compileCreated')}, {compileResult.articles_updated} {t('seriesEditor.compileUpdated')}</p>
            {/if}
            {#if compileError}
              <p class="compile-error">{compileError}</p>
            {/if}
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
                <span class="prereq-arrow">→</span>
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

        <!-- Channel panel at bottom of right panel -->
        <div class="channel-section">
          <ChannelPanel
            {currentChannel}
            {channels}
            currentUserDid={getAuth()?.did || ''}
            onChannelChange={switchChannel}
            fetchCollaborators={() => listCollaborators(id)}
            doInvite={(identifier) => inviteCollaborator(id, identifier).then(() => {})}
            doRemove={(did) => removeCollaborator(id, did)}
            fetchDiff={(target, current) => channelDiff(id, target, current)}
            doApply={(target, source, hash) => applyChannelChange(id, target, source, hash)}
          />
        </div>
      </aside>
    </div>
  </div>
{/if}

<style>
  .editor-page {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 45px);
  }
  .title-area {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 16px;
    border-bottom: 1px solid #e5e5e5;
    background: white;
  }
  .series-title { font-size: 16px; font-weight: 600; }
  .view-link { font-size: 12px; color: #999; text-decoration: none; }
  .view-link:hover { color: var(--accent, #5f9b65); }
  .channel-badge { font-size: 10px; padding: 1px 6px; border-radius: 3px; background: var(--accent, #5f9b65); color: white; font-family: monospace; }
  .dirty-dot { color: var(--accent, #5f9b65); font-size: 10px; }

  .editor-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .version-panel {
    width: 220px;
    border-right: 1px solid #e5e5e5;
    overflow-y: auto;
    background: #fafafa;
    flex-shrink: 0;
  }

  .file-panel {
    width: 160px;
    border-right: 1px solid #e5e5e5;
    overflow-y: auto;
    background: #fafafa;
    flex-shrink: 0;
  }

  .editor-main { flex: 1; min-width: 0; overflow: hidden; display: flex; flex-direction: column; }
  .editor-textarea {
    flex: 1; width: 100%; padding: 16px;
    font-family: monospace; font-size: 14px; line-height: 1.6;
    border: none; resize: none; outline: none;
    background: white; box-sizing: border-box;
  }
  .no-file { flex: 1; display: flex; align-items: center; justify-content: center; color: #aaa; font-size: 14px; }

  .right-panel {
    width: 240px;
    border-left: 1px solid #e5e5e5;
    overflow-y: auto;
    background: #fafafa;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
  }

  .sp-tabs {
    display: flex;
    border-bottom: 1px solid #e5e5e5;
    flex-shrink: 0;
  }
  .sp-tab {
    flex: 1; padding: 8px 4px; font-size: 12px;
    border: none; background: none; cursor: pointer;
    color: #666; border-bottom: 2px solid transparent;
  }
  .sp-tab:hover { color: #333; }
  .sp-tab.active { color: var(--accent, #5f9b65); border-bottom-color: var(--accent, #5f9b65); }
  .sp-body { flex: 1; overflow-y: auto; padding: 14px 12px; display: flex; flex-direction: column; gap: 10px; }
  .sp-hint { font-size: 12px; color: #999; line-height: 1.5; margin: 0; }
  .compile-btn {
    width: 100%; padding: 8px 0; font-size: 13px;
    background: var(--accent, #5f9b65); color: white;
    border: none; border-radius: 4px; cursor: pointer;
  }
  .compile-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .compile-ok { font-size: 12px; color: var(--accent, #5f9b65); margin: 0; }
  .compile-error { font-size: 12px; color: #c33; margin: 0; }

  .prereq-add { display: flex; flex-direction: column; gap: 6px; }
  .prereq-add select { width: 100%; padding: 5px 6px; font-size: 12px; border: 1px solid #ddd; border-radius: 3px; }
  .prereq-arrow { font-size: 12px; color: #999; text-align: center; }
  .add-prereq-btn {
    padding: 5px 0; font-size: 12px; width: 100%;
    background: var(--accent, #5f9b65); color: white;
    border: none; border-radius: 3px; cursor: pointer;
  }
  .add-prereq-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .prereq-list { display: flex; flex-direction: column; gap: 8px; }
  .prereq-group { display: flex; flex-direction: column; gap: 3px; }
  .prereq-article { font-weight: 500; font-size: 12px; }
  .prereq-tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .prereq-tag {
    display: inline-flex; align-items: center; gap: 3px;
    background: rgba(95,155,101,0.1); color: var(--accent, #5f9b65);
    padding: 2px 6px; border-radius: 3px; font-size: 11px;
  }
  .rm-prereq { background: none; border: none; cursor: pointer; color: #999; font-size: 12px; padding: 0; }
  .rm-prereq:hover { color: #c33; }

  .channel-section {
    border-top: 1px solid #e5e5e5;
    flex-shrink: 0;
  }
</style>
