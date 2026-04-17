<script lang="ts">
  import { searchArticles, logout as apiLogout, getUnreadCount } from '../lib/api';
  import { getAuth, setAuth } from '../lib/auth.svelte';
  import { t, getLocale, setLocale, LOCALES } from '../lib/i18n/index.svelte';
  import { loadLangPrefs, clearLangPrefs } from '../lib/langPrefs.svelte';
  import { loadBlocklist, clearBlocklist } from '../lib/blocklist.svelte';
  import type { Locale } from '../lib/i18n/index.svelte';
  import type { Article, AuthUser } from '../lib/types';
  import LoginModal from './LoginModal.svelte';

  let locale = $derived(getLocale());

  function cycleLocale() {
    const codes = LOCALES.map(l => l.code);
    const next = codes[(codes.indexOf(locale) + 1) % codes.length];
    setLocale(next as any);
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

  let searchOpen = $state(false);
  let query = $state('');
  let results = $state<Article[]>([]);
  let selectedIdx = $state(-1);
  let searchEl: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  let loginOpen = $state(false);
  let user = $derived(getAuth());

  // Load/clear settings + blocklist when auth state changes
  $effect(() => {
    if (user) { loadLangPrefs(); loadBlocklist(); }
    else { clearLangPrefs(); clearBlocklist(); }
  });

  async function openSearch() {
    searchOpen = true;
    setTimeout(() => searchEl?.focus(), 0);
  }

  // Listen for keyboard shortcut trigger
  $effect(() => {
    const handler = () => openSearch();
    window.addEventListener('fx:search', handler);
    return () => window.removeEventListener('fx:search', handler);
  });

  function closeSearch() {
    searchOpen = false;
    query = '';
    results = [];
    selectedIdx = -1;
  }

  function onInput() {
    const q = query.trim();
    if (!q) { results = []; selectedIdx = -1; return; }
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      try {
        results = await searchArticles(q, 20);
        selectedIdx = -1;
      } catch { results = []; }
    }, 200);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { closeSearch(); return; }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (results.length > 0) selectedIdx = (selectedIdx + 1) % results.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (results.length > 0) selectedIdx = selectedIdx <= 0 ? results.length - 1 : selectedIdx - 1;
    } else if (e.key === 'Enter' && selectedIdx >= 0 && selectedIdx < results.length) {
      e.preventDefault();
      goToArticle(results[selectedIdx].at_uri);
    }
  }

  function goToArticle(uri: string) {
    window.location.href = `/article?uri=${encodeURIComponent(uri)}`;
    closeSearch();
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
  <a href="/" class="brand">NightBoat</a>
  <div class="nav-links">
    <a href="/questions">{t('nav.questions')}</a>
    <a href="/skills">{t('nav.skills')}</a>
    <a href="/library">{t('nav.library')}</a>
    <a href="/publications">{t('nav.publications')}</a>
    <a href="/books">{t('nav.books')}</a>
    <a href="/courses">{t('nav.courses')}</a>
    <a href="/listings">{t('nav.listings')}</a>
    <a href="/events">{t('nav.events')}</a>
  </div>

  <div class="nav-right">
    <button type="button" class="locale-toggle" onclick={cycleLocale} title="Switch language">
      {(() => { const codes = LOCALES.map(l => l.code); return LOCALES[(codes.indexOf(locale) + 1) % codes.length].label; })()}
    </button>

    <button type="button" class="theme-toggle" onclick={toggleTheme} title={isDark ? 'Light mode' : 'Dark mode'}>
      {#if isDark}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      {/if}
    </button>

    <button type="button" class="search-btn" onclick={openSearch} aria-label="Search">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
    </button>

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

{#if searchOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="search-overlay" onclick={closeSearch}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="search-box" onclick={(e) => e.stopPropagation()}>
      <input
        bind:this={searchEl}
        bind:value={query}
        oninput={onInput}
        onkeydown={onKeydown}
        type="text"
        placeholder={t('nav.search')}
        class="search-input"
      />
      {#if results.length > 0}
        <div class="search-results">
          {#each results as a, i}
            <button type="button" class="search-result" class:selected={i === selectedIdx} onclick={() => goToArticle(a.at_uri)}>
              <span class="result-title">{a.title}</span>
              {#if a.description}
                <span class="result-desc">{a.description}</span>
              {/if}
            </button>
          {/each}
        </div>
      {:else if query.trim()}
        <div class="search-empty">{t('search.noResults')}</div>
      {/if}
    </div>
  </div>
{/if}

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
  .locale-toggle {
    font-size: 12px;
    font-weight: 600;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
    font-family: var(--font-sans);
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%23999' d='M0 2l4 4 4-4z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 4px center;
    padding-right: 16px;
  }
  .locale-toggle:hover {
    border-color: var(--accent);
    color: var(--accent);
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
  .search-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }
  .search-btn:hover {
    color: var(--accent);
  }

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

  /* Search overlay */
  .search-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.3);
    z-index: 200;
    display: flex;
    justify-content: center;
    padding-top: 10vh;
  }
  .search-box {
    width: 560px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
  }
  .search-input {
    width: 100%;
    padding: 14px 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    font-family: var(--font-sans);
    font-size: 16px;
    color: var(--text-primary);
    background: var(--bg-white);
    outline: none;
    border-radius: 0;
  }
  .search-input::placeholder { color: var(--text-hint); }
  .search-results { overflow-y: auto; }
  .search-result {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 10px 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .search-result:last-child { border-bottom: none; }
  .search-result:hover, .search-result.selected { background: var(--bg-hover); }
  .result-title {
    font-family: var(--font-serif);
    font-size: 15px;
    color: var(--text-primary);
  }
  .result-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .search-empty {
    padding: 16px 18px;
    color: var(--text-hint);
    font-size: 14px;
  }
</style>
