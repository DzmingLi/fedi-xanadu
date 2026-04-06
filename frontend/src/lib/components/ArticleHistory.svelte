<script lang="ts">
  import { getArticleHistory, getArticleDiff, unrecordArticleChange, applyChange } from '../api';
  import type { ArticleVersionInfo, VersionDiff } from '../types';
  import { t } from '../i18n/index.svelte';

  let { uri, isOwner = false, applyTargetUri = '' }: { uri: string; isOwner?: boolean; applyTargetUri?: string } = $props();
  let applying = $state<string | null>(null);
  let applyResult = $state<{ has_conflicts: boolean; content: string } | null>(null);

  let versions = $state<ArticleVersionInfo[]>([]);
  let selectedVersion = $state<ArticleVersionInfo | null>(null);
  let diff = $state<VersionDiff | null>(null);
  let loadingDiff = $state(false);
  let unrecording = $state<number | null>(null);
  let error = $state('');

  $effect(() => {
    if (uri) loadHistory();
  });

  async function loadHistory() {
    try {
      versions = await getArticleHistory(uri);
    } catch (e: any) {
      error = e.message;
    }
  }

  async function selectVersion(v: ArticleVersionInfo) {
    selectedVersion = v;
    diff = null;
    error = '';

    const idx = versions.indexOf(v);
    if (idx <= 0) return; // first version has no previous to diff against

    loadingDiff = true;
    try {
      diff = await getArticleDiff(uri, versions[idx - 1].id, v.id);
    } catch (e: any) {
      error = e.message;
    } finally {
      loadingDiff = false;
    }
  }

  async function doUnrecord(v: ArticleVersionInfo) {
    if (!confirm(t('history.unrecordConfirm', v.message))) return;
    unrecording = v.id;
    error = '';
    try {
      await unrecordArticleChange(uri, v.id);
      if (selectedVersion?.id === v.id) {
        selectedVersion = null;
        diff = null;
      }
      await loadHistory();
    } catch (e: any) {
      error = e.message;
    } finally {
      unrecording = null;
    }
  }

  async function doApply(v: ArticleVersionInfo) {
    if (!applyTargetUri) return;
    if (!confirm(t('history.applyConfirm', v.message))) return;
    applying = v.change_hash;
    error = '';
    applyResult = null;
    try {
      const result = await applyChange({
        source_uri: uri,
        target_uri: applyTargetUri,
        change_hash: v.change_hash,
      });
      applyResult = result;
      if (result.has_conflicts) {
        // Navigate to editor with conflict content for manual resolution
        window.location.hash = `#/new?edit_uri=${encodeURIComponent(applyTargetUri)}&resolve_conflicts=1`;
      }
    } catch (e: any) {
      error = e.message;
    } finally {
      applying = null;
    }
  }

  function formatDate(s: string) {
    return new Intl.DateTimeFormat('zh-CN', {
      month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    }).format(new Date(s));
  }

  function shortHash(h: string) {
    return h.slice(0, 8);
  }
</script>

<div class="history-panel">
  <div class="history-list">
    <div class="history-title">{t('history.title')}</div>
    {#if error}
      <p class="history-error">{error}</p>
    {/if}
    {#if versions.length === 0}
      <p class="history-empty">{t('history.empty')}</p>
    {:else}
      {#each [...versions].reverse() as v}
        <button
          class="version-row"
          class:selected={selectedVersion?.id === v.id}
          onclick={() => selectVersion(v)}
        >
          <div class="version-meta">
            <span class="version-msg">{v.message}</span>
            <span class="version-time">{formatDate(v.created_at)}</span>
          </div>
          <code class="version-hash">{shortHash(v.change_hash)}</code>
          {#if isOwner}
            <button
              class="unrecord-btn"
              disabled={!v.unrecordable || unrecording !== null}
              title={v.unrecordable ? t('history.unrecord') : t('history.unrecordBlocked')}
              onclick={(e) => { e.stopPropagation(); doUnrecord(v); }}
            >
              {unrecording === v.id ? '…' : t('history.unrecord')}
            </button>
          {/if}
          {#if applyTargetUri && v.change_hash !== 'fork-initial'}
            <button
              class="apply-btn"
              disabled={applying !== null}
              title={t('history.applyHint')}
              onclick={(e) => { e.stopPropagation(); doApply(v); }}
            >
              {applying === v.change_hash ? '…' : t('history.apply')}
            </button>
          {/if}
        </button>
      {/each}
    {/if}
  </div>

  <div class="diff-panel">
    {#if !selectedVersion}
      <p class="diff-empty">{t('history.selectHint')}</p>
    {:else if loadingDiff}
      <p class="diff-empty">{t('history.loading')}</p>
    {:else if !diff || diff.hunks.length === 0}
      <p class="diff-empty">
        {versions.indexOf(selectedVersion) === 0 ? t('history.initialVersion') : t('history.noChanges')}
      </p>
    {:else}
      <div class="diff-stats">
        <span class="add-count">+{diff.hunks.flatMap(h => h.lines).filter(l => l.kind === 'add').length}</span>
        <span class="del-count">-{diff.hunks.flatMap(h => h.lines).filter(l => l.kind === 'remove').length}</span>
        <span class="diff-msg">{selectedVersion.message}</span>
      </div>
      <pre class="diff-content">{#each diff.hunks as hunk}<span class="hunk-header">@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@</span>
{#each hunk.lines as line}{#if line.kind === 'add'}<span class="line-add">+{line.content}</span>
{:else if line.kind === 'remove'}<span class="line-del">-{line.content}</span>
{:else}<span class="line-ctx"> {line.content}</span>
{/if}{/each}{/each}</pre>
    {/if}

    {#if applyResult && !applyResult.has_conflicts}
      <p class="apply-success">{t('history.applySuccess')}</p>
    {:else if applyResult && applyResult.has_conflicts}
      <p class="apply-conflict">{t('history.applyConflict')}</p>
    {/if}
  </div>
</div>

<style>
  .history-panel {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    background: var(--bg-white);
    min-height: 300px;
    max-height: 60vh;
  }

  .history-list {
    width: 220px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .history-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-hover);
    flex-shrink: 0;
  }

  .history-empty, .history-error {
    font-size: 13px;
    color: var(--text-hint);
    padding: 16px 12px;
    text-align: center;
  }
  .history-error { color: #dc2626; }

  .version-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: none;
    text-align: left;
    cursor: pointer;
    width: 100%;
    transition: background 0.1s;
    position: relative;
  }
  .version-row:hover { background: var(--bg-hover); }
  .version-row.selected { background: rgba(95,155,101,0.08); }

  .version-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .version-msg {
    font-size: 12px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }

  .version-time {
    font-size: 11px;
    color: var(--text-hint);
  }

  .version-hash {
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    color: var(--text-hint);
  }

  .unrecord-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    padding: 2px 6px;
    border: 1px solid #dc2626;
    color: #dc2626;
    background: none;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .unrecord-btn:hover:not(:disabled) {
    background: #dc2626;
    color: white;
  }
  .unrecord-btn:disabled {
    border-color: var(--border);
    color: var(--text-hint);
    cursor: not-allowed;
  }

  .apply-btn {
    position: absolute;
    right: 60px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    padding: 2px 6px;
    border: 1px solid #2563eb;
    color: #2563eb;
    background: none;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .apply-btn:hover:not(:disabled) {
    background: #2563eb;
    color: white;
  }
  .apply-btn:disabled {
    border-color: var(--border);
    color: var(--text-hint);
    cursor: not-allowed;
  }

  .apply-success {
    padding: 12px;
    color: #16a34a;
    font-size: 13px;
  }
  .apply-conflict {
    padding: 12px;
    color: #d97706;
    font-size: 13px;
  }

  .diff-panel {
    flex: 1;
    overflow: auto;
    min-width: 0;
  }

  .diff-empty {
    font-size: 13px;
    color: var(--text-hint);
    padding: 24px;
    text-align: center;
  }

  .diff-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    background: var(--bg-hover);
  }
  .add-count { color: #22863a; font-weight: 500; }
  .del-count { color: #cb2431; font-weight: 500; }
  .diff-msg { color: var(--text-secondary); font-size: 12px; }

  .diff-content {
    font-family: var(--font-mono, 'SF Mono', monospace);
    font-size: 12px;
    line-height: 1.5;
    margin: 0;
    padding: 0;
    overflow-x: auto;
  }

  .diff-content :global(.hunk-header) {
    display: block;
    background: #f1f8ff;
    color: #1d75ab;
    padding: 2px 10px;
    font-style: italic;
  }
  .diff-content :global(.line-add) {
    display: block;
    background: #e6ffec;
    color: #22863a;
    padding: 0 10px;
    white-space: pre;
  }
  .diff-content :global(.line-del) {
    display: block;
    background: #ffebe9;
    color: #cb2431;
    padding: 0 10px;
    white-space: pre;
  }
  .diff-content :global(.line-ctx) {
    display: block;
    padding: 0 10px;
    color: var(--text-secondary);
    white-space: pre;
  }
</style>
