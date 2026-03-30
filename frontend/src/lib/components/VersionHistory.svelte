<script lang="ts">
  import { getArticleHistory, getArticleDiff } from '$lib/api';
  import { t } from '$lib/i18n/index.svelte';
  import type { ArticleVersion, VersionDiff } from '$lib/types';

  let { uri, open = $bindable(false) }: { uri: string; open?: boolean } = $props();

  let versions = $state<ArticleVersion[]>([]);
  let loading = $state(false);
  let diff = $state<VersionDiff | null>(null);
  let diffLoading = $state(false);
  let selectedFrom = $state<number | null>(null);
  let selectedTo = $state<number | null>(null);

  $effect(() => {
    if (!open || !uri) return;
    loading = true;
    getArticleHistory(uri).then(v => {
      versions = v;
      loading = false;
    }).catch(() => { loading = false; });
  });

  async function showDiff(fromId: number, toId: number) {
    selectedFrom = fromId;
    selectedTo = toId;
    diffLoading = true;
    diff = null;
    try {
      diff = await getArticleDiff(uri, fromId, toId);
    } catch { /* */ }
    diffLoading = false;
  }

  function formatDate(d: string): string {
    return new Date(d).toLocaleString();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { open = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="panel" onclick={(e) => e.stopPropagation()}>
      <div class="panel-header">
        <h2>{t('article.versionHistory') ?? '版本历史'}</h2>
        <button class="close-btn" onclick={() => { open = false; }}>&times;</button>
      </div>

      {#if loading}
        <p class="meta">{t('common.loading')}</p>
      {:else if versions.length === 0}
        <p class="meta">暂无版本记录</p>
      {:else}
        <div class="version-list">
          {#each versions as v, i}
            <div class="version-item" class:active={selectedTo === v.id}>
              <div class="version-meta">
                <span class="version-num">v{i + 1}</span>
                <span class="version-date">{formatDate(v.created_at)}</span>
              </div>
              <div class="version-msg">{v.message}</div>
              <div class="version-hash">{v.change_hash.slice(0, 12)}</div>
              {#if i > 0}
                <button
                  class="diff-btn"
                  onclick={() => showDiff(versions[i - 1].id, v.id)}
                  disabled={diffLoading}
                >
                  diff
                </button>
              {/if}
            </div>
          {/each}
        </div>

        {#if diffLoading}
          <p class="meta">{t('common.loading')}</p>
        {:else if diff}
          <div class="diff-view">
            <div class="diff-header">
              v{versions.findIndex(v => v.id === diff?.from_version) + 1} → v{versions.findIndex(v => v.id === diff?.to_version) + 1}
            </div>
            {#each diff.hunks as hunk}
              <div class="diff-hunk">
                <div class="hunk-header">@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@</div>
                {#each hunk.lines as line}
                  <pre class="diff-line {line.kind}">{#if line.kind === 'add'}+{:else if line.kind === 'remove'}-{:else} {/if}{line.content}</pre>
                {/each}
              </div>
            {/each}
            {#if diff.hunks.length === 0}
              <p class="meta">无差异</p>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.35);
    z-index: 300;
    display: flex;
    justify-content: flex-end;
  }
  .panel {
    width: 560px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-white);
    overflow-y: auto;
    padding: 20px;
    box-shadow: -4px 0 16px rgba(0, 0, 0, 0.1);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }
  .panel-header h2 {
    margin: 0;
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.2rem;
  }
  .close-btn {
    background: none;
    border: none;
    font-size: 22px;
    cursor: pointer;
    color: var(--text-hint);
    padding: 0 4px;
    line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); }

  .version-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 16px;
  }
  .version-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 4px;
    border: 1px solid var(--border);
    transition: background 0.1s;
  }
  .version-item:hover { background: var(--bg-hover); }
  .version-item.active {
    border-color: var(--accent);
    background: rgba(95, 155, 101, 0.06);
  }
  .version-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .version-num {
    font-weight: 600;
    font-size: 13px;
    color: var(--accent);
  }
  .version-date {
    font-size: 11px;
    color: var(--text-hint);
  }
  .version-msg {
    flex: 1;
    font-size: 13px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .version-hash {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-hint);
    flex-shrink: 0;
  }
  .diff-btn {
    font-size: 11px;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    font-family: monospace;
    transition: all 0.15s;
  }
  .diff-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .diff-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  /* Diff view */
  .diff-view {
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }
  .diff-header {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-hover);
    border-bottom: 1px solid var(--border);
  }
  .diff-hunk {
    border-bottom: 1px solid var(--border);
  }
  .diff-hunk:last-child { border-bottom: none; }
  .hunk-header {
    padding: 4px 12px;
    font-family: monospace;
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-hover);
  }
  .diff-line {
    margin: 0;
    padding: 1px 12px;
    font-family: monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .diff-line.add {
    background: rgba(46, 160, 67, 0.1);
    color: #1a7f37;
  }
  .diff-line.remove {
    background: rgba(248, 81, 73, 0.1);
    color: #cf222e;
  }
  .diff-line.context {
    color: var(--text-secondary);
  }

  :global([data-theme="dark"]) .diff-line.add {
    background: rgba(46, 160, 67, 0.15);
    color: #3fb950;
  }
  :global([data-theme="dark"]) .diff-line.remove {
    background: rgba(248, 81, 73, 0.15);
    color: #f85149;
  }
</style>
