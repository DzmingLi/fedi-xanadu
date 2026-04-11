<script lang="ts">
  import { getDiscussion, updateDiscussionStatus, applyDiscussionChange, applyAllDiscussionChanges } from '../lib/api';
  import type { DiscussionDetail } from '../lib/api';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';

  let { id } = $props<{ id: string }>();

  let detail = $state<DiscussionDetail | null>(null);
  let loading = $state(true);
  let error = $state('');
  let applyingHash = $state('');
  let applyingAll = $state(false);

  let isTargetOwner = $state(false);

  $effect(() => { load(); });

  async function load() {
    loading = true;
    try {
      detail = await getDiscussion(id);
      // Check if current user owns the target article
      const auth = getAuth();
      if (auth && detail) {
        // Simple heuristic: target_uri contains the owner's DID
        isTargetOwner = detail.discussion.target_uri.includes(auth.did);
      }
    } catch (e: any) {
      error = e.message || 'Failed to load';
    }
    loading = false;
  }

  async function doApply(hash: string) {
    applyingHash = hash;
    try {
      const result = await applyDiscussionChange(id, hash);
      if (result.has_conflicts) {
        alert('应用成功但存在冲突，请在编辑器中解决');
      }
      await load();
    } catch { /* */ }
    applyingHash = '';
  }

  async function doApplyAll() {
    applyingAll = true;
    try {
      const result = await applyAllDiscussionChanges(id);
      if (result.has_conflicts) {
        alert('部分 changes 存在冲突，请在编辑器中解决');
      }
      await load();
    } catch { /* */ }
    applyingAll = false;
  }

  async function doClose() {
    await updateDiscussionStatus(id, 'closed');
    await load();
  }

  async function doReopen() {
    await updateDiscussionStatus(id, 'open');
    await load();
  }

  function statusLabel(s: string) {
    return s === 'open' ? '开放' : s === 'merged' ? '已合并' : '已关闭';
  }
  function statusClass(s: string) {
    return s === 'open' ? 'status-open' : s === 'merged' ? 'status-merged' : 'status-closed';
  }
</script>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  {@const disc = detail.discussion}
  {@const pendingCount = detail.changes.filter(c => !c.applied).length}

  <div class="disc-header">
    <h1>{disc.title}</h1>
    <span class="status-badge {statusClass(disc.status)}">{statusLabel(disc.status)}</span>
  </div>

  <div class="disc-meta">
    <span>由 {disc.author_did.slice(0, 20)}… 发起</span>
    <span>{disc.created_at.split('T')[0]}</span>
  </div>

  <div class="disc-links">
    <span>来源：</span>
    <a href="#/article?uri={encodeURIComponent(disc.source_uri)}">Fork 文章</a>
    <span>→</span>
    <a href="#/article?uri={encodeURIComponent(disc.target_uri)}">原文</a>
  </div>

  {#if disc.body}
    <div class="disc-body">{disc.body}</div>
  {/if}

  <!-- Changes -->
  <div class="changes-section">
    <div class="changes-header">
      <h2>Changes ({detail.changes.length})</h2>
      {#if isTargetOwner && pendingCount > 0 && disc.status === 'open'}
        <button class="apply-all-btn" onclick={doApplyAll} disabled={applyingAll}>
          {applyingAll ? '应用中...' : `全部应用 (${pendingCount})`}
        </button>
      {/if}
    </div>

    {#each detail.changes as change (change.id)}
      <div class="change-item" class:applied={change.applied}>
        <div class="change-info">
          <code class="change-hash">{change.change_hash.slice(0, 16)}…</code>
          {#if change.applied}
            <span class="applied-badge">已应用</span>
          {:else}
            <span class="pending-badge">待审</span>
          {/if}
        </div>
        {#if isTargetOwner && !change.applied && disc.status === 'open'}
          <button
            class="apply-btn"
            onclick={() => doApply(change.change_hash)}
            disabled={applyingHash === change.change_hash}
          >
            {applyingHash === change.change_hash ? '...' : '应用'}
          </button>
        {/if}
      </div>
    {/each}
  </div>

  <!-- Status actions -->
  {#if isTargetOwner || disc.author_did === getAuth()?.did}
    <div class="disc-actions">
      {#if disc.status === 'open'}
        <button class="close-btn" onclick={doClose}>关闭 Discussion</button>
      {:else if disc.status === 'closed'}
        <button class="reopen-btn" onclick={doReopen}>重新打开</button>
      {/if}
    </div>
  {/if}

  <!-- Comments -->
  <div class="disc-comments">
    <CommentThread contentUri={`discussion:${id}`} />
  </div>
{/if}

<style>
  .disc-header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 8px;
  }
  .disc-header h1 {
    margin: 0;
    font-family: var(--font-serif);
  }
  .status-badge {
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 10px;
    font-weight: 500;
  }
  .status-open { background: #e6f4ea; color: #1a7f37; }
  .status-merged { background: #ddf4ff; color: #0969da; }
  .status-closed { background: #f6f0f0; color: #8b6b6b; }

  .disc-meta {
    font-size: 13px;
    color: var(--text-hint);
    display: flex;
    gap: 12px;
    margin-bottom: 8px;
  }
  .disc-links {
    font-size: 13px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .disc-links a {
    color: var(--accent);
    text-decoration: none;
  }
  .disc-links a:hover { text-decoration: underline; }

  .disc-body {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin-bottom: 24px;
    padding: 12px 16px;
    background: var(--bg-gray, #f8f8f8);
    border-radius: 6px;
    white-space: pre-line;
  }

  .changes-section {
    margin: 24px 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .changes-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--bg-hover);
    border-bottom: 1px solid var(--border);
  }
  .changes-header h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .apply-all-btn {
    padding: 4px 12px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--accent);
    color: white;
    font-size: 12px;
    cursor: pointer;
  }
  .apply-all-btn:disabled { opacity: 0.5; cursor: wait; }

  .change-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
  }
  .change-item:last-child { border-bottom: none; }
  .change-item.applied { opacity: 0.6; }
  .change-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .change-hash {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .applied-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
    background: #ddf4ff;
    color: #0969da;
  }
  .pending-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
    background: #fff8c5;
    color: #9a6700;
  }
  .apply-btn {
    padding: 3px 10px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: none;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
  }
  .apply-btn:hover { background: var(--accent); color: white; }
  .apply-btn:disabled { opacity: 0.5; cursor: wait; }

  .disc-actions {
    margin: 16px 0;
    display: flex;
    gap: 8px;
  }
  .close-btn, .reopen-btn {
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    font-size: 13px;
    cursor: pointer;
    color: var(--text-secondary);
  }
  .close-btn:hover { border-color: #c44; color: #c44; }
  .reopen-btn:hover { border-color: var(--accent); color: var(--accent); }

  .disc-comments {
    margin-top: 24px;
  }
</style>
