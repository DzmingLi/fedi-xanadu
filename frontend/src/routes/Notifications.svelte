<script lang="ts">
  import { listNotifications, markNotificationRead, markAllNotificationsRead } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import type { Notification } from '../lib/types';

  let notifications = $state<Notification[]>([]);
  let loading = $state(true);

  let unreadCount = $derived(notifications.filter(n => !n.read).length);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      notifications = await listNotifications(100);
    } catch { /* */ }
    loading = false;
  }

  function actionText(kind: string): string {
    switch (kind) {
      case 'comment_reply': return t('notification.commentReply');
      case 'article_comment': return t('notification.articleComment');
      case 'new_follower': return t('notification.newFollower');
      case 'article_fork': return t('notification.articleFork');
      case 'new_answer': return t('notification.newAnswer');
      default: return kind;
    }
  }

  function notificationHref(n: Notification): string {
    switch (n.kind) {
      case 'comment_reply':
      case 'article_comment':
        return n.target_uri ? `#/article?uri=${encodeURIComponent(n.target_uri)}` : '#/';
      case 'article_fork':
        return n.context_id ? `#/article?uri=${encodeURIComponent(n.context_id)}` : '#/';
      case 'new_answer':
        return n.target_uri ? `#/question?uri=${encodeURIComponent(n.target_uri)}` : '#/';
      case 'new_follower':
        return `#/profile?did=${encodeURIComponent(n.actor_did)}`;
      default:
        return '#/';
    }
  }

  async function clickNotification(n: Notification) {
    if (!n.read) {
      await markNotificationRead(n.id).catch(() => {});
      n.read = true;
      notifications = [...notifications];
    }
    window.location.hash = notificationHref(n);
  }

  async function markAllRead() {
    await markAllNotificationsRead().catch(() => {});
    notifications = notifications.map(n => ({ ...n, read: true }));
  }

  function timeAgo(dateStr: string): string {
    const diff = Date.now() - new Date(dateStr).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return t('notification.justNow');
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    return `${days}d`;
  }
</script>

<h1>{t('nav.notifications')}</h1>

{#if unreadCount > 0}
  <button class="mark-all-btn" onclick={markAllRead}>{t('notification.markAllRead')}</button>
{/if}

{#if loading}
  <p class="meta">Loading...</p>
{:else if notifications.length === 0}
  <p class="empty">{t('notification.empty')}</p>
{:else}
  <div class="notification-list">
    {#each notifications as n (n.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="notification-item" class:unread={!n.read} onclick={() => clickNotification(n)}>
        <div class="notification-body">
          <span class="actor">
            {n.actor_handle ? `@${n.actor_handle}` : n.actor_did.slice(0, 20)}
          </span>
          <span class="action">{actionText(n.kind)}</span>
          {#if n.target_title}
            <span class="target">"{n.target_title}"</span>
          {/if}
        </div>
        <span class="time">{timeAgo(n.created_at)}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  h1 { margin-bottom: 16px; }

  .mark-all-btn {
    font-size: 13px;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
    color: var(--text-secondary);
    margin-bottom: 16px;
    transition: all 0.15s;
  }
  .mark-all-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .notification-list {
    display: flex;
    flex-direction: column;
  }

  .notification-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    transition: background 0.1s;
  }
  .notification-item:hover {
    background: var(--bg-hover);
  }
  .notification-item.unread {
    background: rgba(95, 155, 101, 0.06);
    border-left: 3px solid var(--accent);
  }

  .notification-body {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: baseline;
    font-size: 14px;
    min-width: 0;
  }
  .actor {
    font-weight: 500;
    color: var(--text-primary);
  }
  .action {
    color: var(--text-secondary);
  }
  .target {
    color: var(--text-primary);
    font-style: italic;
  }
  .time {
    font-size: 12px;
    color: var(--text-hint);
    flex-shrink: 0;
    margin-left: 12px;
  }
  .empty {
    color: var(--text-hint);
    font-size: 14px;
  }
</style>
