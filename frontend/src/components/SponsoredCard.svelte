<script lang="ts">
  import { serveAd, recordAdClick, type AdSlot } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';

  let ad = $state<AdSlot | null>(null);

  $effect(() => {
    serveAd('feed').then(served => {
      if (served) ad = served;
    }).catch(() => {});
  });

  function handleClick() {
    if (ad) recordAdClick(ad.id).catch(() => {});
  }
</script>

{#if ad}
  <a href={ad.link_url} target="_blank" rel="noopener sponsored" class="post-card sponsored" onclick={handleClick}>
    <div class="card-top">
      <span class="post-title">{ad.title}</span>
      <span class="sponsored-badge">Sponsored</span>
    </div>

    {#if ad.body}
      <p class="post-desc">{ad.body}</p>
    {/if}

    <div class="card-bottom">
      {#if ad.image_url}
        <img src={ad.image_url} alt="" class="sponsored-icon" />
      {/if}
      <span class="sponsored-domain">{new URL(ad.link_url, location.href).hostname.replace('www.', '')}</span>
    </div>
  </a>
{/if}

<style>
  .post-card {
    display: block;
    position: relative;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px 20px;
    margin-bottom: 12px;
    transition: border-color 0.15s, box-shadow 0.15s;
    text-decoration: none;
    color: inherit;
  }
  .post-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .card-top {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .post-title {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    color: var(--text-primary);
    line-height: 1.35;
    flex: 1;
    min-width: 0;
  }
  .post-card:hover .post-title {
    color: var(--accent);
  }
  .sponsored-badge {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-hint);
    background: var(--bg-hover);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .post-desc {
    margin: 8px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.55;
  }
  .card-bottom {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sponsored-icon {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    object-fit: cover;
  }
  .sponsored-domain {
    font-size: 12px;
    color: var(--text-hint);
  }
</style>
