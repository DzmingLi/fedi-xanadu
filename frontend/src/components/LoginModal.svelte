<script lang="ts">
  import { login as apiLogin } from '../lib/api';
  import { setAuth } from '../lib/auth';

  let { open = $bindable(false) } = $props();

  let handle = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function doLogin() {
    error = '';
    loading = true;
    try {
      const user = await apiLogin(handle, password);
      setAuth(user);
      open = false;
      handle = '';
      password = '';
    } catch (e: any) {
      error = e.message || 'Login failed';
    }
    loading = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
    if (e.key === 'Enter' && handle && password) doLogin();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => { open = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h2>Login with AT Protocol</h2>
      <p class="hint">使用你的 Bluesky handle 和 App Password 登录</p>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <label>
        Handle
        <input
          type="text"
          bind:value={handle}
          onkeydown={onKeydown}
          placeholder="alice.bsky.social"
          disabled={loading}
        />
      </label>

      <label>
        App Password
        <input
          type="password"
          bind:value={password}
          onkeydown={onKeydown}
          placeholder="xxxx-xxxx-xxxx-xxxx"
          disabled={loading}
        />
      </label>

      <p class="hint small">
        在 <a href="https://bsky.app/settings/app-passwords" target="_blank" rel="noopener">bsky.app/settings/app-passwords</a> 创建 App Password
      </p>
      <p class="hint small">
        还没有帐号？前往 <a href="https://bsky.app" target="_blank" rel="noopener">bsky.app</a> 注册 Bluesky
      </p>

      <div class="actions">
        <button class="btn-cancel" onclick={() => { open = false; }} disabled={loading}>取消</button>
        <button class="btn-login" onclick={doLogin} disabled={loading || !handle || !password}>
          {loading ? '登录中...' : '登录'}
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
    margin: 0 0 4px;
  }
  .hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 16px;
  }
  .hint.small {
    font-size: 12px;
    margin: 8px 0 16px;
  }
  .hint a {
    color: var(--accent);
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
