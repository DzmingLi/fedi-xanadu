<script lang="ts">
  import type { ChannelDiffResult } from '../api';
  import { getAuth } from '../auth.svelte';

  interface CollabItem {
    user_did: string;
    channel_name: string;
    role: string;
  }

  let { currentChannel, channels, onChannelChange, fetchCollaborators, doInvite: doInviteApi, doRemove: doRemoveApi, fetchDiff, doApply: doApplyApi }: {
    currentChannel: string;
    channels: string[];
    onChannelChange: (ch: string) => void;
    fetchCollaborators: () => Promise<CollabItem[]>;
    doInvite: (did: string) => Promise<void>;
    doRemove: (did: string) => Promise<void>;
    fetchDiff: (target: string, current: string) => Promise<ChannelDiffResult>;
    doApply: (targetChannel: string, sourceChannel: string, hash: string) => Promise<void>;
  } = $props();

  let collaborators = $state<CollabItem[]>([]);
  let inviteDid = $state('');
  let inviting = $state(false);
  let diffTarget = $state('');
  let diffResult = $state<ChannelDiffResult | null>(null);
  let diffLoading = $state(false);
  let applyingHash = $state('');

  let isOwner = $derived(collaborators.some(c => c.user_did === getAuth()?.did && c.role === 'owner'));

  $effect(() => {
    loadCollaborators();
  });

  async function loadCollaborators() {
    try {
      collaborators = await fetchCollaborators();
    } catch { /* */ }
  }

  async function doInvite() {
    if (!inviteDid.trim() || inviting) return;
    inviting = true;
    try {
      await doInviteApi(inviteDid.trim());
      inviteDid = '';
      await loadCollaborators();
    } catch { /* */ }
    inviting = false;
  }

  async function doRemove(did: string) {
    if (!confirm('确定移除此协作者？')) return;
    try {
      await doRemoveApi(did);
      await loadCollaborators();
    } catch { /* */ }
  }

  async function loadDiff(target: string) {
    if (!target || target === currentChannel) {
      diffResult = null;
      return;
    }
    diffTarget = target;
    diffLoading = true;
    try {
      diffResult = await fetchDiff(target, currentChannel);
    } catch { diffResult = null; }
    diffLoading = false;
  }

  async function doApply(hash: string) {
    applyingHash = hash;
    try {
      await doApplyApi(currentChannel, diffTarget, hash);
      await loadDiff(diffTarget);
    } catch { /* */ }
    applyingHash = '';
  }

  async function doApplyAll() {
    if (!diffResult) return;
    for (const hash of diffResult.only_in_a) {
      await doApply(hash);
    }
  }
</script>

<div class="channel-panel">
  <h3>协作</h3>

  <!-- Channel selector -->
  <div class="section">
    <label class="section-label">当前 Channel</label>
    <select
      class="channel-select"
      value={currentChannel}
      onchange={(e) => onChannelChange((e.target as HTMLSelectElement).value)}
    >
      {#each channels as ch}
        <option value={ch}>{ch}</option>
      {/each}
    </select>
  </div>

  <!-- Collaborators -->
  <div class="section">
    <label class="section-label">协作者</label>
    <div class="collab-list">
      {#each collaborators as c (c.user_did)}
        <div class="collab-item">
          <span class="collab-did" title={c.user_did}>
            {c.user_did.length > 24 ? c.user_did.slice(0, 12) + '…' + c.user_did.slice(-8) : c.user_did}
          </span>
          <span class="collab-role">{c.role}</span>
          <span class="collab-channel">{c.channel_name}</span>
          {#if isOwner && c.role !== 'owner'}
            <button class="remove-btn" onclick={() => doRemove(c.user_did)} title="移除">×</button>
          {/if}
        </div>
      {/each}
    </div>
    {#if isOwner}
      <div class="invite-row">
        <input
          type="text"
          class="invite-input"
          placeholder="did:plc:..."
          bind:value={inviteDid}
          onkeydown={(e) => { if (e.key === 'Enter') doInvite(); }}
        />
        <button class="invite-btn" onclick={doInvite} disabled={inviting || !inviteDid.trim()}>
          {inviting ? '...' : '邀请'}
        </button>
      </div>
    {/if}
  </div>

  <!-- Channel diff -->
  <div class="section">
    <label class="section-label">比较 Channel</label>
    <select class="channel-select" value={diffTarget} onchange={(e) => loadDiff((e.target as HTMLSelectElement).value)}>
      <option value="">选择...</option>
      {#each channels.filter(ch => ch !== currentChannel) as ch}
        <option value={ch}>{ch}</option>
      {/each}
    </select>

    {#if diffLoading}
      <p class="meta">加载中...</p>
    {:else if diffResult}
      {#if diffResult.only_in_a.length === 0 && diffResult.only_in_b.length === 0}
        <p class="meta">两个 channel 完全同步</p>
      {:else}
        {#if diffResult.only_in_a.length > 0}
          <div class="diff-section">
            <div class="diff-header">
              <span>仅在 {diffTarget} ({diffResult.only_in_a.length})</span>
              <button class="apply-all-btn" onclick={doApplyAll}>全部应用</button>
            </div>
            {#each diffResult.only_in_a as hash}
              <div class="diff-change">
                <code class="change-hash">{hash.slice(0, 12)}…</code>
                <button
                  class="apply-btn"
                  onclick={() => doApply(hash)}
                  disabled={applyingHash === hash}
                >
                  {applyingHash === hash ? '...' : '应用'}
                </button>
              </div>
            {/each}
          </div>
        {/if}
        {#if diffResult.only_in_b.length > 0}
          <div class="diff-section">
            <span class="diff-label">仅在 {currentChannel} ({diffResult.only_in_b.length})</span>
          </div>
        {/if}
      {/if}
    {/if}
  </div>
</div>

<style>
  .channel-panel {
    padding: 12px;
    font-size: 13px;
  }
  .channel-panel h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 12px;
  }
  .section {
    margin-bottom: 16px;
  }
  .section-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-hint);
    text-transform: uppercase;
    margin-bottom: 6px;
  }
  .channel-select {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 13px;
    background: var(--bg-white);
  }
  .collab-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  .collab-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    font-size: 12px;
  }
  .collab-did {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .collab-role {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--bg-gray, #f0f0f0);
    color: var(--text-hint);
  }
  .collab-channel {
    font-size: 10px;
    color: var(--accent);
    font-family: monospace;
  }
  .remove-btn {
    background: none;
    border: none;
    color: var(--text-hint);
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
  }
  .remove-btn:hover {
    color: #c44;
  }
  .invite-row {
    display: flex;
    gap: 6px;
  }
  .invite-input {
    flex: 1;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
  }
  .invite-btn {
    padding: 4px 10px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--accent);
    color: white;
    font-size: 12px;
    cursor: pointer;
  }
  .invite-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .diff-section {
    margin-top: 8px;
  }
  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    margin-bottom: 4px;
  }
  .diff-label {
    font-size: 12px;
    color: var(--text-hint);
  }
  .diff-change {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 0;
    border-bottom: 1px solid var(--border);
  }
  .change-hash {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .apply-btn, .apply-all-btn {
    padding: 2px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: none;
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
  }
  .apply-btn:hover, .apply-all-btn:hover {
    background: var(--accent);
    color: white;
  }
  .apply-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .meta {
    font-size: 12px;
    color: var(--text-hint);
    margin: 4px 0;
  }
</style>
