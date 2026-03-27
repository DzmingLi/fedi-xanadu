<script lang="ts">
  import { listArticles, logout as apiLogout } from '../lib/api';
  import { getAuth, setAuth, onAuthChange } from '../lib/auth';
  import { t, getLocale, setLocale, onLocaleChange, LOCALES } from '../lib/i18n';
  import type { Locale } from '../lib/i18n';
  import type { Article, AuthUser } from '../lib/types';
  import LoginModal from './LoginModal.svelte';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  function cycleLocale() {
    const codes = LOCALES.map(l => l.code);
    const idx = codes.indexOf(locale);
    setLocale(codes[(idx + 1) % codes.length]);
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
  let allArticles = $state<Article[]>([]);
  let searchEl: HTMLInputElement | undefined = $state();

  let loginOpen = $state(false);
  let user = $state<AuthUser | null>(getAuth());

  $effect(() => {
    return onAuthChange(() => { user = getAuth(); });
  });

  async function openSearch() {
    searchOpen = true;
    if (allArticles.length === 0) {
      allArticles = await listArticles();
    }
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
  }

  function onInput() {
    const q = query.trim().toLowerCase();
    if (!q) { results = []; return; }
    results = allArticles
      .filter(a => a.title.toLowerCase().includes(q) || a.description.toLowerCase().includes(q))
      .slice(0, 8);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeSearch();
  }

  function goToArticle(uri: string) {
    window.location.hash = `#/article?uri=${encodeURIComponent(uri)}`;
    closeSearch();
  }

  async function doLogout() {
    try { await apiLogout(); } catch { /* ignore */ }
    setAuth(null);
  }
</script>

<nav>
  <a href="#/" class="brand">Fedi-Xanadu</a>
  <div class="nav-links">
    <a href="#/skills">{t('nav.skills')}</a>
    <a href="#/library">{t('nav.library')}</a>
    <a href="#/roadmap">{t('nav.roadmap')}</a>
    <a href="#/about">{t('nav.about')}</a>
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
      <div class="user-menu">
        <a href="#/profile?did={encodeURIComponent(user.did)}" class="user-link">
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
      <a href="#/drafts" class="btn-drafts">{t('nav.drafts')}</a>
    {/if}
    <a href="#/new" class="btn-new">{t('nav.newArticle')}</a>
    <a href="#/new-series" class="btn-new">{t('nav.newSeries')}</a>
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
          {#each results as a}
            <button type="button" class="search-result" onclick={() => goToArticle(a.at_uri)}>
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
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
    font-family: var(--font-sans);
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
  .search-result:hover { background: var(--bg-hover); }
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
