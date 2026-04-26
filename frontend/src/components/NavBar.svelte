<script lang="ts">
  import { logout as apiLogout, getUnreadCount } from '../lib/api';
  import { getAuth, setAuth } from '../lib/auth.svelte';
  import { t, getLocale, setLocale, LOCALES } from '../lib/i18n/index.svelte';
  import { loadLangPrefs, clearLangPrefs } from '../lib/langPrefs.svelte';
  import { loadBlocklist, clearBlocklist } from '../lib/blocklist.svelte';
  import type { Locale } from '../lib/i18n/index.svelte';
  import type { AuthUser } from '../lib/types';
  import LoginModal from './LoginModal.svelte';

  let locale = $derived(getLocale());
  let currentLocaleLabel = $derived(
    LOCALES.find(l => l.code === locale)?.label ?? locale,
  );

  let localeMenuOpen = $state(false);
  let localeMenuEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!localeMenuOpen) return;
    function onDocClick(e: MouseEvent) {
      if (localeMenuEl && !localeMenuEl.contains(e.target as Node)) {
        localeMenuOpen = false;
      }
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') localeMenuOpen = false;
    }
    document.addEventListener('click', onDocClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('click', onDocClick);
      document.removeEventListener('keydown', onKey);
    };
  });

  function pickLocale(code: Locale) {
    setLocale(code);
    localeMenuOpen = false;
  }

  let isDark = $state(
    localStorage.getItem('theme') === 'dark' ||
    (!localStorage.getItem('theme') && window.matchMedia('(prefers-color-scheme: dark)').matches)
  );

  $effect(() => {
    document.documentElement.setAttribute('data-theme', isDark ? 'dark' : 'light');
    localStorage.setItem('theme', isDark ? 'dark' : 'light');
  });

  function toggleTheme() {
    isDark = !isDark;
  }

  let query = $state('');
  let searchEl: HTMLInputElement | undefined = $state();

  let loginOpen = $state(false);
  let user = $derived(getAuth());

  // Load/clear settings + blocklist when auth state changes
  $effect(() => {
    if (user) { loadLangPrefs(); loadBlocklist(); }
    else { clearLangPrefs(); clearBlocklist(); }
  });

  // Keyboard shortcut (`/`) focuses the navbar search input.
  $effect(() => {
    const handler = () => searchEl?.focus();
    window.addEventListener('fx:search', handler);
    return () => window.removeEventListener('fx:search', handler);
  });

  function submitSearch(e: Event) {
    e.preventDefault();
    const q = query.trim();
    if (!q) return;
    window.location.href = `/search?q=${encodeURIComponent(q)}`;
  }

  let unreadCount = $state(0);

  $effect(() => {
    if (!user) { unreadCount = 0; return; }
    const poll = async () => {
      try { unreadCount = (await getUnreadCount()).count; } catch {}
    };
    poll();
    const timer = setInterval(poll, 60_000);
    return () => clearInterval(timer);
  });

  async function doLogout() {
    try { await apiLogout(); } catch { /* ignore */ }
    setAuth(null);
  }
</script>

<nav>
  <a href="/" class="brand">NightBo.at</a>
  <div class="nav-links">
    <a href="/questions">{t('nav.questions')}</a>
    <a href="/skills">{t('nav.skills')}</a>
    <a href="/hierarchy">{t('nav.hierarchy')}</a>
    <a href="/library">{t('nav.library')}</a>
    <a href="/publications">{t('nav.publications')}</a>
    <a href="/books">{t('nav.books')}</a>
    <a href="/papers">{t('nav.papers')}</a>
    <a href="/terms">{t('nav.terms')}</a>
    <a href="/listings">{t('nav.listings')}</a>
  </div>

  <div class="nav-right">
    <div class="locale-menu" bind:this={localeMenuEl}>
      <button
        type="button"
        class="locale-trigger"
        onclick={() => localeMenuOpen = !localeMenuOpen}
        aria-haspopup="listbox"
        aria-expanded={localeMenuOpen}
        title={t('nav.switchLanguage')}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="10"/>
          <line x1="2" y1="12" x2="22" y2="12"/>
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
        </svg>
        <span>{currentLocaleLabel}</span>
        <svg class="chevron" width="9" height="9" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true"><path d="M1 3l4 4 4-4z"/></svg>
      </button>
      {#if localeMenuOpen}
        <ul class="locale-list" role="listbox" aria-label={t('nav.switchLanguage')}>
          {#each LOCALES as l}
            <li>
              <button
                type="button"
                class="locale-option"
                class:active={l.code === locale}
                onclick={() => pickLocale(l.code)}
                role="option"
                aria-selected={l.code === locale}
              >
                <span class="check">{l.code === locale ? '✓' : ''}</span>
                <span>{l.label}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <button type="button" class="theme-toggle" onclick={toggleTheme} title={isDark ? 'Light mode' : 'Dark mode'}>
      {#if isDark}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      {/if}
    </button>

    <form class="nav-search" onsubmit={submitSearch} role="search">
      <svg class="nav-search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      <input
        bind:this={searchEl}
        bind:value={query}
        type="search"
        placeholder={t('nav.search')}
        class="nav-search-input"
        aria-label={t('nav.search')}
      />
    </form>

    {#if user}
      <a href="/notifications" class="notif-btn" title={t('nav.notifications')}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
          <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
        </svg>
        {#if unreadCount > 0}
          <span class="notif-badge">{unreadCount > 99 ? '99+' : unreadCount}</span>
        {/if}
      </a>
      <div class="user-menu">
        <a href="/profile?did={encodeURIComponent(user.did)}" class="user-link">
          {#if user.avatar}
            <img src={user.avatar} alt="" class="user-avatar" />
          {/if}
          <span class="user-handle">@{user.handle}</span>
        </a>
        <button class="btn-logout" onclick={doLogout}>{t('nav.logout')}</button>
      </div>
    {:else}
      <button class="btn-login" onclick={() => { loginOpen = true; }}>{t('nav.login')}</button>
    {/if}

    {#if user}
      <a href="/settings" class="settings-btn" title={t('nav.settings')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
      </a>
      <a href="/creator" class="btn-drafts">{t('nav.creator')}</a>
      <a href="/drafts" class="btn-drafts">{t('nav.drafts')}</a>
    {/if}
    <a href="/new" class="btn-new">{t('nav.newArticle')}</a>
    <a href="/new-series" class="btn-new">{t('nav.newSeries')}</a>
  </div>
</nav>

<LoginModal bind:open={loginOpen} />


<style>
  nav {
    position: sticky;
    top: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    padding: 0.625rem 0;
    margin-bottom: 1.5rem;
    background: color-mix(in srgb, var(--bg-page) 85%, transparent);
    backdrop-filter: blur(8px);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 400;
    color: var(--text-primary);
    text-decoration: none;
    letter-spacing: -0.01em;
  }
  .brand:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .nav-links {
    display: flex;
    gap: 0.75rem;
    margin-left: 0.5rem;
    flex-shrink: 0;
  }
  .nav-links a {
    font-size: 14px;
    color: var(--text-secondary);
    text-decoration: none;
    transition: color 0.15s;
  }
  .nav-links a:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .nav-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .locale-menu {
    position: relative;
  }
  .locale-trigger {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    padding: 3px 7px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
    font-family: var(--font-sans);
  }
  .locale-trigger:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .locale-trigger .chevron {
    opacity: 0.7;
    transition: transform 0.15s;
  }
  .locale-trigger[aria-expanded="true"] .chevron {
    transform: rotate(180deg);
  }
  .locale-list {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 200;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--bg-page);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.12);
    min-width: 140px;
  }
  .locale-option {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 6px 8px;
    font-size: 13px;
    background: none;
    border: none;
    border-radius: 3px;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    transition: background 0.1s, color 0.1s;
  }
  .locale-option:hover {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    color: var(--accent);
  }
  .locale-option.active {
    color: var(--accent);
    font-weight: 600;
  }
  .locale-option .check {
    width: 12px;
    flex-shrink: 0;
    font-size: 11px;
    color: var(--accent);
    text-align: center;
  }
  .theme-toggle {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }
  .theme-toggle:hover {
    color: var(--accent);
  }
  .nav-search {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-input, transparent);
    transition: border-color 0.15s;
    min-width: 180px;
  }
  .nav-search:focus-within {
    border-color: var(--accent);
  }
  .nav-search-icon {
    color: var(--text-hint);
    flex-shrink: 0;
  }
  .nav-search-input {
    border: none;
    outline: none;
    background: none;
    font-size: 13px;
    color: var(--text-primary);
    padding: 3px 0;
    flex: 1;
    min-width: 0;
    font-family: inherit;
  }
  .nav-search-input::placeholder { color: var(--text-hint); }
  .nav-search-input::-webkit-search-cancel-button { appearance: none; }

  /* Notification bell */
  .notif-btn {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--text-secondary);
    padding: 4px;
    text-decoration: none;
    transition: color 0.15s;
  }
  .notif-btn:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .notif-badge {
    position: absolute;
    top: -2px;
    right: -4px;
    background: #dc2626;
    color: white;
    font-size: 10px;
    font-weight: 600;
    min-width: 16px;
    height: 16px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 3px;
    line-height: 1;
  }

  /* Auth buttons */
  .btn-login {
    font-size: 13px;
    padding: 4px 12px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    background: none;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-login:hover {
    background: var(--accent);
    color: white;
  }
  .user-menu {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .user-link {
    display: flex;
    align-items: center;
    gap: 6px;
    text-decoration: none;
    transition: opacity 0.15s;
  }
  .user-link:hover { opacity: 0.8; text-decoration: none; }
  .user-avatar {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    object-fit: cover;
  }
  .user-handle {
    color: var(--text-secondary);
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn-logout {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-hint);
    padding: 2px 4px;
    transition: color 0.15s;
  }
  .btn-logout:hover { color: var(--accent); }

  .settings-btn {
    display: flex;
    align-items: center;
    color: var(--text-secondary);
    padding: 4px;
    text-decoration: none;
    transition: color 0.15s;
  }
  .settings-btn:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .btn-drafts {
    font-size: 12px;
    padding: 3px 8px;
    border: 1px dashed var(--border);
    border-radius: 3px;
    color: var(--text-secondary);
    text-decoration: none;
    transition: all 0.15s;
  }
  .btn-drafts:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .btn-new {
    font-size: 13px;
    padding: 4px 12px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    text-decoration: none;
    transition: all 0.15s;
  }
  .btn-new:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }

</style>
