<script lang="ts">
  import {
    ACTIONS, getAllBindings, parseKeyCombo, matchesKey, formatKeyDisplay,
    CATEGORY_LABELS, onBindingsChange, loadFromServer,
  } from '../lib/keybindings';
  import { onAuthChange } from '../lib/auth';

  let helpOpen = $state(false);
  let settingsOpen = $state(false);
  let pendingKeys: string[] = $state([]);
  let pendingTimeout: ReturnType<typeof setTimeout> | null = null;
  let bindings = $state(getAllBindings());

  // Reload bindings when they change
  $effect(() => {
    return onBindingsChange(() => { bindings = getAllBindings(); });
  });

  // Load from server on auth change
  $effect(() => {
    return onAuthChange(() => { loadFromServer(); });
  });

  // Load from server on mount
  $effect(() => { loadFromServer(); });

  // Expose open functions for external use
  export function openHelp() { helpOpen = true; }
  export function openSettings() { settingsOpen = true; }

  // Build action-to-parsed-key map
  function getActionMap(): Map<string, { sequence: string[][]; actionId: string }> {
    const map = new Map();
    for (const [actionId, combo] of Object.entries(bindings)) {
      map.set(actionId, { sequence: parseKeyCombo(combo), actionId });
    }
    return map;
  }

  function isInputFocused(): boolean {
    const el = document.activeElement;
    if (!el) return false;
    const tag = el.tagName.toLowerCase();
    if (tag === 'input' || tag === 'textarea' || tag === 'select') return true;
    if ((el as HTMLElement).isContentEditable) return true;
    return false;
  }

  function executeAction(actionId: string) {
    switch (actionId) {
      case 'goto.home': window.location.hash = '#/'; break;
      case 'goto.skills': window.location.hash = '#/skills'; break;
      case 'goto.library': window.location.hash = '#/library'; break;
      case 'goto.about': window.location.hash = '#/about'; break;
      case 'goto.newArticle': window.location.hash = '#/new'; break;
      case 'goto.newSeries': window.location.hash = '#/new-series'; break;
      case 'search':
        // Dispatch custom event for NavBar to pick up
        window.dispatchEvent(new CustomEvent('fx:search'));
        break;
      case 'help':
        helpOpen = !helpOpen;
        settingsOpen = false;
        break;
      case 'settings':
        settingsOpen = !settingsOpen;
        helpOpen = false;
        break;
      case 'article.upvote':
      case 'article.downvote':
      case 'article.bookmark':
      case 'list.next':
      case 'list.prev':
      case 'list.open':
        // Dispatch custom events for page components
        window.dispatchEvent(new CustomEvent(`fx:${actionId}`));
        break;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // Don't intercept when typing in inputs
    if (isInputFocused()) return;
    // Don't intercept when modals are open (except Escape)
    if ((helpOpen || settingsOpen) && e.key === 'Escape') {
      helpOpen = false;
      settingsOpen = false;
      e.preventDefault();
      return;
    }
    if (helpOpen || settingsOpen) return;

    const actionMap = getActionMap();

    // Try matching with pending keys first (for sequences like "g h")
    const testKeys = [...pendingKeys, 'current'];

    for (const [, entry] of actionMap) {
      const seq = entry.sequence;

      // Check if current pending + this key matches
      if (pendingKeys.length + 1 === seq.length) {
        // Verify all pending keys match
        let allMatch = true;
        for (let i = 0; i < pendingKeys.length; i++) {
          if (pendingKeys[i] !== seq[i].join('+')) {
            allMatch = false;
            break;
          }
        }
        if (allMatch && matchesKey(e, seq[seq.length - 1])) {
          e.preventDefault();
          pendingKeys = [];
          if (pendingTimeout) clearTimeout(pendingTimeout);
          executeAction(entry.actionId);
          return;
        }
      }

      // Check if this could be the start of a sequence
      if (seq.length > 1 && pendingKeys.length === 0 && matchesKey(e, seq[0])) {
        e.preventDefault();
        pendingKeys = [seq[0].join('+')];
        if (pendingTimeout) clearTimeout(pendingTimeout);
        pendingTimeout = setTimeout(() => { pendingKeys = []; }, 1000);
        return;
      }
    }

    // Single-key shortcuts (if no pending sequence was started for this key)
    if (pendingKeys.length === 0) {
      for (const [, entry] of actionMap) {
        if (entry.sequence.length === 1 && matchesKey(e, entry.sequence[0])) {
          e.preventDefault();
          executeAction(entry.actionId);
          return;
        }
      }
    }

    // Reset pending if no match
    pendingKeys = [];
  }

  $effect(() => {
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  // Group actions by category
  function groupedActions() {
    const groups: Record<string, typeof ACTIONS> = {};
    for (const action of ACTIONS) {
      if (!groups[action.category]) groups[action.category] = [];
      groups[action.category].push(action);
    }
    return groups;
  }
</script>

<!-- Pending key indicator -->
{#if pendingKeys.length > 0}
  <div class="key-pending">
    {pendingKeys.join(' ')} ...
  </div>
{/if}

<!-- Help modal -->
{#if helpOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="kb-overlay" onclick={() => { helpOpen = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="kb-modal" onclick={(e) => e.stopPropagation()}>
      <div class="kb-header">
        <h2>Keyboard Shortcuts</h2>
        <button class="kb-close" onclick={() => { helpOpen = false; }}>&times;</button>
      </div>
      <div class="kb-body">
        {#each Object.entries(groupedActions()) as [cat, actions]}
          <div class="kb-category">
            <h3>{CATEGORY_LABELS[cat]?.en ?? cat}</h3>
            {#each actions as action}
              <div class="kb-row">
                <span class="kb-label">{action.label}</span>
                <kbd class="kb-key">{formatKeyDisplay(bindings[action.id] ?? action.defaultKey)}</kbd>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      <div class="kb-footer">
        <span class="kb-hint">Press <kbd>?</kbd> to toggle · <kbd>Esc</kbd> to close</span>
        <button class="kb-settings-btn" onclick={() => { helpOpen = false; settingsOpen = true; }}>
          Customize
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Settings modal (imported separately) -->
{#if settingsOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="kb-overlay" onclick={() => { settingsOpen = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="kb-modal kb-settings-modal" onclick={(e) => e.stopPropagation()}>
      <div class="kb-header">
        <h2>Customize Shortcuts</h2>
        <button class="kb-close" onclick={() => { settingsOpen = false; }}>&times;</button>
      </div>
      <div class="kb-body">
        {#await import('./KeybindingsEditor.svelte') then mod}
          <mod.default onclose={() => { settingsOpen = false; }} />
        {/await}
      </div>
    </div>
  </div>
{/if}

<style>
  .key-pending {
    position: fixed;
    bottom: 24px;
    right: 24px;
    background: var(--text-primary, #1a1a1a);
    color: white;
    padding: 8px 16px;
    border-radius: 6px;
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    z-index: 999;
    opacity: 0.9;
    pointer-events: none;
  }

  .kb-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 500;
    display: flex;
    justify-content: center;
    padding-top: 8vh;
  }

  .kb-modal {
    width: 520px;
    max-width: 90vw;
    max-height: 80vh;
    background: var(--bg-white, #fff);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    align-self: flex-start;
  }
  .kb-settings-modal {
    width: 600px;
  }

  .kb-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border, #e5e5e5);
  }
  .kb-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .kb-close {
    background: none;
    border: none;
    font-size: 22px;
    cursor: pointer;
    color: var(--text-hint, #999);
    padding: 0 4px;
    line-height: 1;
  }
  .kb-close:hover { color: var(--text-primary, #333); }

  .kb-body {
    padding: 12px 20px;
    overflow-y: auto;
    flex: 1;
  }

  .kb-category {
    margin-bottom: 16px;
  }
  .kb-category h3 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-hint, #999);
    margin: 0 0 6px;
  }

  .kb-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 0;
  }
  .kb-label {
    font-size: 14px;
    color: var(--text-primary, #333);
  }
  .kb-key {
    display: inline-flex;
    gap: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    background: var(--bg-hover, #f5f5f5);
    border: 1px solid var(--border, #e5e5e5);
    border-radius: 3px;
    padding: 2px 8px;
    color: var(--text-secondary, #666);
    min-width: 24px;
    text-align: center;
  }

  .kb-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    border-top: 1px solid var(--border, #e5e5e5);
  }
  .kb-hint {
    font-size: 12px;
    color: var(--text-hint, #999);
  }
  .kb-hint kbd {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    background: var(--bg-hover, #f5f5f5);
    border: 1px solid var(--border, #e5e5e5);
    border-radius: 2px;
    padding: 0 4px;
  }
  .kb-settings-btn {
    font-size: 13px;
    padding: 4px 14px;
    border: 1px solid var(--accent, #4a7c59);
    border-radius: 3px;
    color: var(--accent, #4a7c59);
    background: none;
    cursor: pointer;
    transition: all 0.15s;
  }
  .kb-settings-btn:hover {
    background: var(--accent, #4a7c59);
    color: white;
  }
</style>
