<script lang="ts">
  import type { BookShortReview, SeriesShortReview } from '../types';
  import { t } from '../i18n/index.svelte';

  let { review, onDelete }: {
    review: BookShortReview | SeriesShortReview;
    onDelete?: () => void;
  } = $props();

  function formatDate(s: string) {
    return new Date(s).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

</script>

<div class="short-review-card">
  <div class="sr-header">
    {#if review.author_avatar}
      <img src={review.author_avatar} alt="" class="sr-avatar" />
    {:else}
      <div class="sr-avatar placeholder"></div>
    {/if}
    <div class="sr-meta">
      <span class="sr-handle">{review.author_display_name || review.author_handle || review.did.slice(0, 12)}</span>
    </div>
    <span class="sr-date">{formatDate(review.created_at)}</span>
  </div>
  <p class="sr-body">{review.body}</p>
  {#if onDelete}
    <button class="sr-delete" onclick={onDelete}>{t('books.deleteShortReview')}</button>
  {/if}
</div>

<style>
  .short-review-card {
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 8px;
    padding: 12px 16px;
    background: var(--surface, #fff);
  }
  .sr-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }
  .sr-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .sr-avatar.placeholder {
    background: var(--border, #e5e7eb);
  }
  .sr-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sr-handle {
    font-weight: 600;
    font-size: 0.9rem;
  }
.sr-date {
    font-size: 0.8rem;
    color: var(--text-muted, #6b7280);
    white-space: nowrap;
  }
  .sr-body {
    margin: 0;
    font-size: 0.92rem;
    line-height: 1.6;
    white-space: pre-wrap;
  }
  .sr-delete {
    margin-top: 8px;
    background: none;
    border: none;
    color: var(--text-muted, #6b7280);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0;
  }
  .sr-delete:hover { color: #dc2626; }
</style>
