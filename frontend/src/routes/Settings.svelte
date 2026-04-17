<script lang="ts">
  import { getSettings, setSettings, getKeybindings, listBlockedUsers, unblockUser as apiUnblockUser, listBookmarkFolders, listMembers, addMember, removeMember, getProfile, updateBio } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale, setLocale, LOCALES } from '../lib/i18n/index.svelte';
  import type { Locale } from '../lib/i18n/index.svelte';
  import { LANG_NAMES } from '../lib/i18n/index.svelte';
  import { setLangPrefs } from '../lib/langPrefs.svelte';
  import {
    ACTIONS, getAllBindings, formatKeyDisplay, CATEGORY_LABELS,
    setBinding, resetBindings, saveToServer,
  } from '../lib/keybindings.svelte';
  import { toast } from '../lib/components/Toast.svelte';
  import { removeBlocked } from '../lib/blocklist.svelte';
  import type { UserSettings, BlockedUser, ContentFormat } from '../lib/types';

  let locale = $derived(getLocale());

  let loading = $state(true);
  let saving = $state(false);

  // Settings form
  let nativeLang = $state('zh');
  let knownLangs = $state<string[]>(['zh']);
  let preferNative = $state(true);
  let hideUnknown = $state(false);
  let defaultFormat = $state<ContentFormat>('typst');
  let email = $state('');
  let bookmarksPublic = $state(false);
  let publicFolders = $state<string[]>([]);
  let allFolders = $state<string[]>([]);
  let bio = $state('');

  // Blocked users
  let blockedUsers = $state<BlockedUser[]>([]);

  // Members
  let members = $state<{ author_did: string; member_did: string; created_at: string }[]>([]);
  let newMemberDid = $state('');

  // Keybindings
  let bindings = $state<Record<string, string>>({});
  let editingAction = $state<string | null>(null);
  let capturedKeys = $state<string[]>([]);

  const ALL_LANGS = ['zh', 'en', 'ja', 'ko', 'fr', 'de', 'es', 'pt'];
  const FORMATS = ['typst', 'markdown'];

  $effect(() => { load(); });

  async function load() {
    if (!getAuth()) {
      loading = false;
      return;
    }
    loading = true;
    try {
      const s = await getSettings();
      nativeLang = s.native_lang;
      knownLangs = [...s.known_langs];
      preferNative = s.prefer_native;
      hideUnknown = s.hide_unknown;
      defaultFormat = s.default_format;
      email = s.email || '';
      bookmarksPublic = s.bookmarks_public ?? false;
      publicFolders = [...(s.public_folders ?? [])];
    } catch { /* use defaults */ }

    try {
      allFolders = await listBookmarkFolders();
    } catch { /* */ }

    try {
      const kb = await getKeybindings();
      bindings = { ...kb.bindings };
    } catch {
      bindings = getAllBindings();
    }

    try {
      blockedUsers = await listBlockedUsers();
    } catch { /* */ }

    try {
      members = await listMembers();
    } catch { /* */ }

    try {
      const auth = getAuth();
      if (auth) {
        const profile = await getProfile(auth.did);
        bio = profile.bio || '';
      }
    } catch { /* */ }

    loading = false;
  }

  async function doAddMember() {
    if (!newMemberDid.trim()) return;
    try {
      await addMember(newMemberDid.trim());
      members = await listMembers();
      newMemberDid = '';
    } catch (err: any) {
      toast(err.message || 'Failed to add member');
    }
  }

  async function doRemoveMember(did: string) {
    try {
      await removeMember(did);
      members = members.filter(m => m.member_did !== did);
    } catch { /* */ }
  }

  async function save() {
    saving = true;
    try {
      // Ensure native_lang is in known_langs
      const langs = knownLangs.includes(nativeLang)
        ? knownLangs
        : [nativeLang, ...knownLangs];

      const settings: UserSettings = {
        native_lang: nativeLang,
        known_langs: langs,
        prefer_native: preferNative,
        hide_unknown: hideUnknown,
        default_format: defaultFormat,
        email: email.trim() || null,
        bookmarks_public: bookmarksPublic,
        public_folders: publicFolders,
      };
      const saved = await setSettings(settings);
      setLangPrefs(saved);

      // Sync UI locale if native_lang is a supported locale
      const supported = LOCALES.map(l => l.code);
      if (supported.includes(nativeLang as Locale)) {
        setLocale(nativeLang as Locale);
      }

      toast(t('settings.saved'), 'success');
    } catch {
      toast(t('settings.saveFailed'), 'error');
    }
    saving = false;
  }

  function toggleKnownLang(lang: string) {
    if (knownLangs.includes(lang)) {
      if (lang === nativeLang) return; // can't remove native
      knownLangs = knownLangs.filter(l => l !== lang);
    } else {
      knownLangs = [...knownLangs, lang];
    }
  }

  function onNativeChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    nativeLang = val;
    if (!knownLangs.includes(val)) {
      knownLangs = [val, ...knownLangs];
    }
  }

  async function doUnblock(did: string) {
    try {
      await apiUnblockUser(did);
      removeBlocked(did);
      blockedUsers = blockedUsers.filter(b => b.blocked_did !== did);
    } catch { /* */ }
  }

  // Keybinding editing
  function startCapture(actionId: string) {
    editingAction = actionId;
    capturedKeys = [];
  }

  function onKeyCapture(e: KeyboardEvent) {
    if (!editingAction) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      editingAction = null;
      capturedKeys = [];
      return;
    }

    const parts: string[] = [];
    if (e.ctrlKey) parts.push('ctrl');
    if (e.altKey) parts.push('alt');
    if (e.shiftKey) parts.push('shift');
    if (e.metaKey) parts.push('meta');

    const key = e.key.toLowerCase();
    if (!['control', 'alt', 'shift', 'meta'].includes(key)) {
      parts.push(key);
      const combo = parts.join('+');
      setBinding(editingAction, combo);
      bindings = getAllBindings();
      editingAction = null;
      capturedKeys = [];
    }
  }

  function resetKey(actionId: string) {
    const action = ACTIONS.find(a => a.id === actionId);
    if (action) setBinding(actionId, action.defaultKey);
    bindings = getAllBindings();
  }

  function resetAll() {
    resetBindings();
    bindings = getAllBindings();
  }

  async function saveKeybindings() {
    try {
      await saveToServer();
      toast(t('settings.saved'), 'success');
    } catch {
      toast(t('settings.saveFailed'), 'error');
    }
  }

  // Group actions by category
  let actionsByCategory = $derived.by(() => {
    const map = new Map<string, typeof ACTIONS>();
    for (const action of ACTIONS) {
      const cat = action.category;
      const arr = map.get(cat) || [];
      arr.push(action);
      map.set(cat, arr);
    }
    return map;
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="settings-page" onkeydown={onKeyCapture}>
  <h1>{t('settings.title')}</h1>

  {#if !getAuth()}
    <p class="login-hint">{t('nav.login')}</p>
  {:else if loading}
    <p class="meta">{t('common.loading')}</p>
  {:else}
    <div class="settings-section">
      <h2>{t('settings.nativeLang')}</h2>
      <p class="hint">{t('settings.nativeLangHint')}</p>
      <select value={nativeLang} onchange={onNativeChange} class="select-input">
        {#each ALL_LANGS as lang}
          <option value={lang}>{LANG_NAMES[lang] || lang}</option>
        {/each}
      </select>
    </div>

    <div class="settings-section">
      <h2>{t('settings.knownLangs')}</h2>
      <p class="hint">{t('settings.knownLangsHint')}</p>
      <div class="lang-chips">
        {#each ALL_LANGS as lang}
          <button
            class="lang-chip"
            class:active={knownLangs.includes(lang)}
            class:native={lang === nativeLang}
            onclick={() => toggleKnownLang(lang)}
            disabled={lang === nativeLang}
          >
            {LANG_NAMES[lang] || lang}
          </button>
        {/each}
      </div>
    </div>

    <div class="settings-section">
      <label class="toggle-row">
        <input type="checkbox" bind:checked={preferNative} />
        <span class="toggle-label">{t('settings.preferNative')}</span>
      </label>
      <p class="hint">{t('settings.preferNativeHint')}</p>
    </div>

    <div class="settings-section">
      <label class="toggle-row">
        <input type="checkbox" bind:checked={hideUnknown} />
        <span class="toggle-label">{t('settings.hideUnknown')}</span>
      </label>
      <p class="hint">{t('settings.hideUnknownHint')}</p>
    </div>

    <div class="settings-section">
      <h2>{t('settings.defaultFormat')}</h2>
      <div class="format-options">
        {#each FORMATS as fmt}
          <label class="radio-row">
            <input type="radio" name="format" value={fmt} bind:group={defaultFormat} />
            <span>{fmt === 'typst' ? 'Typst' : 'Markdown'}</span>
          </label>
        {/each}
      </div>
    </div>

    <div class="settings-section">
      <h2>{t('settings.bio')}</h2>
      <p class="hint">{t('settings.bioHint')}</p>
      <textarea bind:value={bio} class="text-input bio-input" rows="3" placeholder={t('settings.bioPlaceholder')}></textarea>
      <button class="save-btn small" onclick={async () => { await updateBio(bio); toast(t('settings.saved'), 'success'); }}>{t('common.save')}</button>
    </div>

    <div class="settings-section">
      <h2>{t('settings.email')}</h2>
      <p class="hint">{t('settings.emailHint')}</p>
      <input type="email" bind:value={email} placeholder="user@example.com" class="text-input" />
    </div>

    <div class="settings-section">
      <label class="toggle-row">
        <input type="checkbox" bind:checked={bookmarksPublic} />
        <span class="toggle-label">{t('settings.bookmarksPublic')}</span>
      </label>
      <p class="hint">{t('settings.bookmarksPublicHint')}</p>
      {#if bookmarksPublic && allFolders.length > 0}
        <p class="hint" style="margin-top:8px">{t('settings.publicFoldersHint')}</p>
        <div class="lang-chips">
          {#each allFolders as folder}
            <button
              class="lang-chip"
              class:active={publicFolders.includes(folder)}
              onclick={() => {
                if (publicFolders.includes(folder)) {
                  publicFolders = publicFolders.filter(f => f !== folder);
                } else {
                  publicFolders = [...publicFolders, folder];
                }
              }}
            >
              {folder === '/' ? t('library.seriesFolder') : folder.replace(/^\//, '')}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="settings-actions">
      <button class="save-btn" onclick={save} disabled={saving}>
        {saving ? t('common.loading') : t('common.save')}
      </button>
    </div>

    <!-- Blocked users -->
    <div class="settings-section">
      <h2>{t('block.blockedUsers')}</h2>
      {#if blockedUsers.length === 0}
        <p class="hint">{t('block.empty')}</p>
      {:else}
        {#each blockedUsers as b}
          <div class="blocked-row">
            <a href="/profile?did={encodeURIComponent(b.blocked_did)}" class="blocked-name">
              {b.display_name || b.handle || b.blocked_did.slice(0, 20)}
            </a>
            <button class="unblock-btn" onclick={() => doUnblock(b.blocked_did)}>{t('block.unblock')}</button>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Members (restricted content access) -->
    <div class="settings-section">
      <h2>{t('books.members')}</h2>
      <p class="hint">{t('settings.membersHint')}</p>
      <div class="member-add-row">
        <input type="text" bind:value={newMemberDid} placeholder="DID or handle" class="member-input" />
        <button class="btn" onclick={doAddMember}>{t('books.addMember')}</button>
      </div>
      {#if members.length === 0}
        <p class="hint">{t('settings.noMembers')}</p>
      {:else}
        {#each members as m}
          <div class="blocked-row">
            <a href="/profile?did={encodeURIComponent(m.member_did)}" class="blocked-name">{m.member_did.slice(0, 24)}…</a>
            <button class="unblock-btn" onclick={() => doRemoveMember(m.member_did)}>{t('books.removeMember')}</button>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Keybindings section -->
    <div class="settings-section keybindings-section">
      <h2>{t('settings.keybindings')}</h2>
      {#each [...actionsByCategory] as [category, actions]}
        <h3 class="kb-category">{CATEGORY_LABELS[category]?.[locale] || category}</h3>
        {#each actions as action}
          <div class="kb-row">
            <span class="kb-action">{action.labels[locale]}</span>
            {#if editingAction === action.id}
              <span class="kb-key capturing">...</span>
            {:else}
              <button class="kb-key" onclick={() => startCapture(action.id)}>
                {formatKeyDisplay(bindings[action.id] || '')}
              </button>
            {/if}
            <button class="kb-reset" onclick={() => resetKey(action.id)} title={t('kb.resetDefault')}>
              &times;
            </button>
          </div>
        {/each}
      {/each}
      <div class="kb-actions">
        <button class="kb-reset-all" onclick={resetAll}>{t('kb.resetAll')}</button>
        <button class="save-btn" onclick={saveKeybindings}>{t('kb.save')}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 600px;
    margin: 0 auto;
  }
  .settings-page h1 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin-bottom: 24px;
  }
  .settings-section {
    margin-bottom: 28px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border);
  }
  .settings-section h2 {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 6px;
  }
  .hint {
    font-size: 13px;
    color: var(--text-hint);
    margin: 0 0 10px;
  }
  .select-input {
    padding: 6px 10px;
    font-size: 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
    font-family: var(--font-sans);
  }
  .text-input {
    padding: 6px 10px;
    font-size: 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
    font-family: var(--font-sans);
    width: 100%;
    max-width: 360px;
  }
  .lang-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .lang-chip {
    padding: 4px 12px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .lang-chip.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .lang-chip.native {
    opacity: 0.8;
    cursor: default;
  }
  .lang-chip:hover:not(.native):not(.active) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 14px;
  }
  .toggle-label {
    font-weight: 500;
  }
  .radio-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    cursor: pointer;
    margin-bottom: 4px;
  }
  .format-options {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .settings-actions {
    margin-bottom: 32px;
  }
  .save-btn {
    padding: 8px 24px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .save-btn:hover { opacity: 0.9; }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .login-hint {
    color: var(--text-hint);
    font-size: 14px;
  }

  /* Blocked users */
  .member-add-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }
  .member-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-size: 13px;
  }
  .blocked-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }
  .blocked-row:last-child { border-bottom: none; }
  .blocked-name {
    font-size: 14px;
    color: var(--text-primary);
    text-decoration: none;
  }
  .blocked-name:hover { color: var(--accent); }
  .unblock-btn {
    font-size: 12px;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .unblock-btn:hover {
    border-color: #dc2626;
    color: #dc2626;
  }

  /* Keybindings */
  .keybindings-section {
    border-bottom: none;
  }
  .kb-category {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 16px 0 8px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .kb-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 0;
  }
  .kb-action {
    flex: 1;
    font-size: 13px;
    color: var(--text-primary);
  }
  .kb-key {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-page);
    color: var(--text-secondary);
    cursor: pointer;
    min-width: 60px;
    text-align: center;
    transition: all 0.15s;
  }
  .kb-key:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .kb-key.capturing {
    border-color: var(--accent);
    color: var(--accent);
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  .kb-reset {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-hint);
    padding: 0 4px;
  }
  .kb-reset:hover { color: #dc2626; }
  .kb-actions {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }
  .kb-reset-all {
    padding: 8px 16px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .kb-reset-all:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
