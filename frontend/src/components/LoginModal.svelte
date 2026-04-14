<script lang="ts">
  import { login as apiLogin, register as apiRegister, startOAuthLogin } from '../lib/api';
  import { setAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';

  let { open = $bindable(false) } = $props();

  let mode = $state<'platform' | 'atproto'>('platform');
  let isRegister = $state(false);
  let handle = $state('');
  let password = $state('');
  let displayName = $state('');
  let error = $state('');
  let loading = $state(false);

  async function doSubmit() {
    error = '';
    if (!handle) return;

    if (mode === 'atproto') {
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
    if (e.key === 'Enter') {
      if (mode === 'atproto' && handle) doSubmit();
      else if (mode === 'platform' && handle && password) doSubmit();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { open = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h2>{isRegister ? (t('auth.register') || 'Register') : t('nav.login')}</h2>

      <div class="tabs">
        <button class="tab" class:active={mode === 'platform'} onclick={() => { mode = 'platform'; error = ''; }}>
          {t('auth.platformLogin') || 'Platform'}
        </button>
        <button class="tab" class:active={mode === 'atproto'} onclick={() => { mode = 'atproto'; error = ''; isRegister = false; }}>
          AT Protocol
        </button>
      </div>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      {#if mode === 'platform'}
        <label>
          {t('auth.handle') || 'Username'}
          <input
            type="text"
            bind:value={handle}
            onkeydown={onKeydown}
            placeholder="alice"
            disabled={loading}
          />
        </label>

        {#if isRegister}
          <label>
            {t('auth.displayName') || 'Display Name'}
            <input
              type="text"
              bind:value={displayName}
              onkeydown={onKeydown}
              placeholder={t('auth.displayNamePlaceholder') || 'Optional'}
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
            placeholder={isRegister ? (t('auth.passwordMin') || '8+ characters') : ''}
            disabled={loading}
          />
        </label>

        <p class="hint small toggle-hint">
          {#if isRegister}
            {t('auth.hasAccount') || 'Already have an account?'}
            <button class="link-btn" onclick={() => { isRegister = false; error = ''; }}>{t('nav.login')}</button>
          {:else}
            {t('auth.noAccountLocal') || "Don't have an account?"}
            <button class="link-btn" onclick={() => { isRegister = true; error = ''; }}>{t('auth.register') || 'Register'}</button>
          {/if}
        </p>
      {:else}
        <label>
          {t('auth.handle')}
          <input
            type="text"
            bind:value={handle}
            onkeydown={onKeydown}
            placeholder="alice.bsky.social"
            disabled={loading}
          />
        </label>
        <p class="hint small">{t('auth.oauthHint') || 'You will be redirected to your PDS to authorize.'}</p>
      {/if}

      <div class="actions">
        <button class="btn-cancel" onclick={() => { open = false; }} disabled={loading}>{t('common.cancel')}</button>
        <button class="btn-login" onclick={doSubmit} disabled={loading || !handle || (mode === 'platform' && !password)}>
          {#if loading}
            {t('common.loading')}
          {:else if mode === 'atproto'}
            {t('auth.submit') || 'Login'}
          {:else if isRegister}
            {t('auth.register') || 'Register'}
          {:else}
            {t('auth.submit') || 'Login'}
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
    margin: 0 0 12px;
  }
  .tabs {
    display: flex;
    gap: 0;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    flex: 1;
    padding: 8px 12px;
    font-size: 13px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .hint.small {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 4px 0 16px;
  }
  .toggle-hint {
    margin-top: 0;
  }
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
