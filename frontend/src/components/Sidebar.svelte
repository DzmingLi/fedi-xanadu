<script lang="ts">
  import { listFollows, markFollowSeen, type FollowedUser } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import SidebarAd from './SidebarAd.svelte';

  let locale = $derived(getLocale());
  let follows = $state<FollowedUser[]>([]);
  let user = $derived(getAuth());

  $effect(() => {
    void user;
    loadFollows();
  });

  async function loadFollows() {
    if (!getAuth()) { follows = []; return; }
    try {
      follows = await listFollows();
    } catch { follows = []; }
  }

  function displayName(f: FollowedUser): string {
    return f.display_name || f.handle || f.follows_did.slice(0, 20);
  }

  async function goToProfile(f: FollowedUser) {
    if (f.has_update) {
      // Mark as seen
      await markFollowSeen(f.follows_did).catch(() => {});
      f.has_update = false;
    }
    window.location.href = `/profile?did=${encodeURIComponent(f.follows_did)}`;
  }
</script>

<aside class="sidebar">
  <nav class="sidebar-nav">
    <a href="/" class="sidebar-link active-home">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
      {t('sidebar.home')}
    </a>
    <a href="/questions" class="sidebar-link">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      {t('nav.questions')}
    </a>
    <a href="/skills" class="sidebar-link">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>
      {t('sidebar.skills')}
    </a>
    <a href="/library" class="sidebar-link">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/></svg>
      {t('sidebar.library')}
    </a>
  </nav>

  <!-- Followed users -->
  {#if user && follows.length > 0}
    <div class="sidebar-divider"></div>
    <div class="sidebar-section">
      <div class="sidebar-heading">{t('home.following')}</div>
      <nav class="sidebar-nav follows-list">
        {#each follows as f}
          <button class="sidebar-link follow-link" onclick={() => goToProfile(f)}>
            {#if f.avatar_url}
              <img src={f.avatar_url} alt="" class="follow-avatar" />
            {:else}
              <span class="follow-avatar-placeholder"></span>
            {/if}
            <span class="follow-name">{displayName(f)}</span>
            {#if f.has_update}
              <span class="update-dot"></span>
            {/if}
          </button>
        {/each}
      </nav>
    </div>
  {/if}

  <div class="sidebar-divider"></div>

  <nav class="sidebar-nav sidebar-secondary">
    <a href="/guide" class="sidebar-link">{t('sidebar.guide')}</a>
    <a href="/about" class="sidebar-link">{t('sidebar.about')}</a>
  </nav>

  <div class="sidebar-divider"></div>

  <div class="sidebar-section">
    <div class="sidebar-heading">{t('sidebar.happening') || 'Happening'}</div>
    <a href="/listings" class="happening-item">
      <span class="happening-icon">📢</span>
      <span class="happening-text">{t('sidebar.hiring') || 'Academic hiring — browse open positions'}</span>
    </a>
    <a href="/events" class="happening-item">
      <span class="happening-icon">📅</span>
      <span class="happening-text">{t('nav.events') || 'Events'}</span>
    </a>
    <a href="/about" class="happening-item">
      <span class="happening-icon">✍️</span>
      <span class="happening-text">{t('sidebar.creators') || 'Creator onboarding — start writing today'}</span>
    </a>
    <a href="/about" class="happening-item">
      <span class="happening-icon">🎁</span>
      <span class="happening-text">{t('sidebar.incentives') || 'Creator incentives coming soon'}</span>
    </a>
  </div>

  <SidebarAd />

  <div class="sidebar-section">
    <div class="sidebar-heading">NightBoat</div>
    <p class="sidebar-text">{t('sidebar.desc')}</p>
    <p class="sidebar-text"><a href="/about">{t('sidebar.learnMore')} &rarr;</a></p>
  </div>
</aside>

<style>
  .sidebar {
    position: sticky;
    top: 4rem;
    width: 200px;
    flex-shrink: 0;
    align-self: flex-start;
    padding-top: 0.5rem;
  }
  .sidebar-nav {
    display: flex;
    flex-direction: column;
  }
  .sidebar-link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: 14px;
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: 3px;
    transition: background 0.1s, color 0.1s;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    width: 100%;
    font-family: var(--font-sans);
  }
  .sidebar-link:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    text-decoration: none;
  }
  .sidebar-link.active-home {
    color: var(--text-primary);
    font-weight: 500;
  }
  .sidebar-secondary .sidebar-link {
    font-size: 13px;
    padding: 4px 10px;
  }
  .sidebar-divider {
    height: 1px;
    background: var(--border);
    margin: 8px 10px;
  }
  .sidebar-section {
    padding: 4px 10px;
  }
  .sidebar-heading {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 4px;
  }
  .sidebar-text {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 4px 0;
  }

  /* Follow list */
  .follows-list {
    gap: 1px;
  }
  .follow-link {
    font-size: 13px;
    padding: 4px 10px;
    position: relative;
  }
  .follow-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .follow-avatar-placeholder {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--border);
    flex-shrink: 0;
  }
  .follow-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .update-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #ef4444;
    flex-shrink: 0;
  }

  /* Happening */
  .happening-item {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 4px 0;
    text-decoration: none;
    transition: opacity 0.1s;
  }
  .happening-item:hover { opacity: 0.8; text-decoration: none; }
  .happening-icon { font-size: 13px; flex-shrink: 0; line-height: 1.4; }
  .happening-text { font-size: 12px; color: var(--text-secondary); line-height: 1.4; }
  .happening-item:hover .happening-text { color: var(--accent); }

  @media (max-width: 960px) {
    .sidebar {
      display: none;
    }
  }
</style>
