<script lang="ts">
  import { login as apiLogin, register as apiRegister, startOAuthLogin } from '../lib/api';
  import { setAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';

  let { open = $bindable(false) } = $props();

  let isRegister = $state(false);
  let handle = $state('');
  let password = $state('');
  let displayName = $state('');
  let error = $state('');
  let loading = $state(false);

  // If the user typed a handle from a different PDS (contains a dot AND
  // doesn't end with .nightbo.at), we route them to OAuth instead of the
  // password form.
  let isExternalHandle = $derived(
    handle.includes('.') && !handle.toLowerCase().endsWith('.nightbo.at')
  );

  async function doSubmit() {
    error = '';
    if (!handle) return;

    if (isExternalHandle) {
      startOAuthLogin(handle);
      return;
    }

    if (!password) return;
    loading = true;
    try {
      const user = isRegister
        ? await apiRegister(handle, password, displayName || undefined)
        : await apiLogin(handle, password);
      setAuth(user);
      open = false;
      handle = '';
      password = '';
      displayName = '';
      isRegister = false;
    } catch (e: any) {
      error = e.message || (isRegister ? 'Registration failed' : 'Login failed');
    }
    loading = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
    if (e.key === 'Enter') doSubmit();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { open = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h2>{isRegister ? t('auth.register') : t('nav.login')}</h2>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <label>
        {t('auth.handle')}
        <div class="handle-row">
          <input
            type="text"
            bind:value={handle}
            onkeydown={onKeydown}
            placeholder={isRegister ? 'alice' : 'alice 或 alice.bsky.social'}
            disabled={loading}
            autocomplete="username"
          />
          {#if !isExternalHandle && !handle.includes('.')}
            <span class="handle-suffix">.nightbo.at</span>
          {/if}
        </div>
      </label>

      {#if isExternalHandle}
        <p class="hint small">{t('auth.oauthHint')}</p>
      {:else if !isRegister}
        <p class="hint small byo-hint">
          {t('auth.byoHint')}
        </p>
      {/if}

      {#if !isExternalHandle}
        {#if isRegister}
          <label>
            {t('auth.displayName')}
            <input
              type="text"
              bind:value={displayName}
              onkeydown={onKeydown}
              placeholder={t('auth.displayNamePlaceholder')}
              disabled={loading}
            />
          </label>
        {/if}
        <label>
          {t('auth.password')}
          <input
            type="password"
            bind:value={password}
            onkeydown={onKeydown}
            placeholder={isRegister ? t('auth.passwordMin') : ''}
            disabled={loading}
            autocomplete={isRegister ? 'new-password' : 'current-password'}
          />
        </label>
        <p class="hint small toggle-hint">
          {#if isRegister}
            {t('auth.hasAccount')}
            <button class="link-btn" onclick={() => { isRegister = false; error = ''; }}>{t('nav.login')}</button>
          {:else}
            {t('auth.noAccount')}
            <button class="link-btn" onclick={() => { isRegister = true; error = ''; }}>{t('auth.register')}</button>
          {/if}
        </p>
      {/if}

      <div class="actions">
        <button class="btn-cancel" onclick={() => { open = false; }} disabled={loading}>{t('common.cancel')}</button>
        <button
          class="btn-login"
          onclick={doSubmit}
          disabled={loading || !handle || (!isExternalHandle && !password)}
        >
          {#if loading}
            {t('common.loading')}
          {:else if isExternalHandle}
            {t('auth.submit')}
          {:else if isRegister}
            {t('auth.register')}
          {:else}
            {t('auth.submit')}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.3);
    z-index: 300;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .modal {
    width: 400px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 8px;
    padding: 24px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.15);
  }
  .modal h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 0 0 16px;
  }
  .hint.small {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 4px 0 16px;
  }
  .toggle-hint { margin-top: 0; }
  .link-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
    padding: 0;
    text-decoration: underline;
  }
  .error {
    background: #fef2f2;
    color: #dc2626;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 13px;
    margin-bottom: 12px;
  }
  label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }
  input {
    display: block;
    width: 100%;
    margin-top: 4px;
    padding: 8px 10px;
    font-size: 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
    background: var(--bg-white);
  }
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .handle-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .handle-row input { flex: 1; }
  .handle-suffix {
    font-size: 13px;
    color: var(--text-hint);
    white-space: nowrap;
    margin-top: 4px;
    opacity: 0.6;
  }
  .byo-hint { color: var(--text-hint); }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .btn-cancel {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .btn-login {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: var(--accent);
    color: white;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-login:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
