<script lang="ts">
  import {
    ACTIONS, getAllBindings, setBinding, resetBindings, saveToServer,
    formatKeyDisplay, CATEGORY_LABELS,
  } from '../lib/keybindings.svelte';
  import { getToken } from '../lib/auth.svelte';

  let { onclose }: { onclose: () => void } = $props();

  let bindings = $state(getAllBindings());
  let recording: string | null = $state(null);
  let recordedKeys: string[] = $state([]);
  let dirty = $state(false);

  function startRecording(actionId: string) {
    recording = actionId;
    recordedKeys = [];
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      recording = null;
      recordedKeys = [];
      return;
    }

    // Build key representation
    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.shiftKey) parts.push('Shift');
    if (e.altKey) parts.push('Alt');
    if (e.metaKey) parts.push('Meta');

    const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
    if (!['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
      parts.push(key);
    } else {
      return; // Don't record modifier-only presses
    }

    const combo = parts.join('+');

    // If it's a simple key (no modifiers), could be part of a sequence
    if (!e.ctrlKey && !e.altKey && !e.metaKey && key.length === 1) {
      recordedKeys = [...recordedKeys, combo];

      // Wait briefly for potential second key in sequence
      if (recordedKeys.length === 1) {
        setTimeout(() => {
          if (recording && recordedKeys.length === 1) {
            // Single key, commit
            commitRecording();
          }
        }, 800);
      } else {
        // Second key in sequence, commit immediately
        commitRecording();
      }
    } else {
      // Modifier chord, commit immediately
      recordedKeys = [combo];
      commitRecording();
    }
  }

  function commitRecording() {
    if (!recording || recordedKeys.length === 0) return;
    const finalKey = recordedKeys.join(' ');
    setBinding(recording, finalKey);
    bindings = getAllBindings();
    recording = null;
    recordedKeys = [];
    dirty = true;
  }

  function resetAll() {
    resetBindings();
    bindings = getAllBindings();
    dirty = true;
  }

  async function save() {
    await saveToServer();
    dirty = false;
    onclose();
  }

  function groupedActions() {
    const groups: Record<string, typeof ACTIONS> = {};
    for (const action of ACTIONS) {
      if (!groups[action.category]) groups[action.category] = [];
      groups[action.category].push(action);
    }
    return groups;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor">
  {#each Object.entries(groupedActions()) as [cat, actions]}
    <div class="ed-category">
      <h3>{CATEGORY_LABELS[cat]?.en ?? cat}</h3>
      {#each actions as action}
        <div class="ed-row">
          <span class="ed-label">{action.label}</span>
          <div class="ed-key-area">
            {#if recording === action.id}
              <span class="ed-recording">
                {#if recordedKeys.length > 0}
                  {recordedKeys.join(' ')} ...
                {:else}
                  Press key...
                {/if}
              </span>
            {:else}
              <button class="ed-key-btn" onclick={() => startRecording(action.id)}>
                {formatKeyDisplay(bindings[action.id])}
              </button>
            {/if}
            {#if bindings[action.id] !== action.defaultKey}
              <button
                class="ed-reset-btn"
                onclick={() => { setBinding(action.id, action.defaultKey); bindings = getAllBindings(); dirty = true; }}
                title="Reset to default"
              >&times;</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/each}
</div>

<div class="ed-footer">
  <button class="ed-reset-all" onclick={resetAll}>Reset all</button>
  <div class="ed-footer-right">
    {#if !getToken()}
      <span class="ed-hint">Log in to sync shortcuts to PDS</span>
    {/if}
    <button class="ed-save-btn" onclick={save}>
      {dirty ? 'Save & Close' : 'Close'}
    </button>
  </div>
</div>

<style>
  .editor {
    min-height: 200px;
  }
  .ed-category {
    margin-bottom: 16px;
  }
  .ed-category h3 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-hint, #999);
    margin: 0 0 6px;
  }
  .ed-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
  }
  .ed-label {
    font-size: 14px;
    color: var(--text-primary, #333);
  }
  .ed-key-area {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .ed-key-btn {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    background: var(--bg-hover, #f5f5f5);
    border: 1px solid var(--border, #e5e5e5);
    border-radius: 3px;
    padding: 3px 10px;
    cursor: pointer;
    min-width: 40px;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: all 0.15s;
  }
  .ed-key-btn:hover {
    border-color: var(--accent, #4a7c59);
    color: var(--accent, #4a7c59);
  }
  .ed-recording {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    background: var(--accent, #4a7c59);
    color: white;
    border-radius: 3px;
    padding: 3px 10px;
    min-width: 40px;
    text-align: center;
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
  .ed-reset-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-hint, #999);
    padding: 0 2px;
    line-height: 1;
  }
  .ed-reset-btn:hover { color: #dc2626; }

  .ed-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 12px;
    border-top: 1px solid var(--border, #e5e5e5);
    margin-top: 8px;
  }
  .ed-footer-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .ed-hint {
    font-size: 12px;
    color: var(--text-hint, #999);
  }
  .ed-reset-all {
    font-size: 13px;
    padding: 4px 12px;
    border: 1px solid var(--border, #e5e5e5);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary, #666);
    transition: all 0.15s;
  }
  .ed-reset-all:hover {
    border-color: #dc2626;
    color: #dc2626;
  }
  .ed-save-btn {
    font-size: 13px;
    padding: 5px 16px;
    background: var(--accent, #4a7c59);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .ed-save-btn:hover { opacity: 0.9; }
</style>
