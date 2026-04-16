<script lang="ts">
  import { serveAd, recordAdClick, type AdSlot } from '../lib/api';

  let ad = $state<AdSlot | null>(null);

  $effect(() => {
    serveAd('sidebar').then(served => {
      ad = served;
    }).catch(() => {});
  });

  function handleClick() {
    if (ad) recordAdClick(ad.id).catch(() => {});
  }
</script>

{#if ad}
  <a
    href={ad.link_url}
    target="_blank"
    rel="noopener sponsored"
    class="sponsored-item"
    onclick={handleClick}
  >
    {#if ad.image_url}
      <img src={ad.image_url} alt="" class="sponsored-icon" />
    {/if}
    <span class="sponsored-text">
      <strong>{ad.title}</strong> — {ad.body || ''}
      <span class="sponsored-label">Sponsored</span>
    </span>
  </a>
{/if}

<style>
  .sponsored-item {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 4px 0;
    text-decoration: none;
    transition: opacity 0.1s;
  }
  .sponsored-item:hover { opacity: 0.8; text-decoration: none; }
  .sponsored-icon {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    object-fit: cover;
    flex-shrink: 0;
    margin-top: 1px;
  }
  .sponsored-icon-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    background: var(--bg-hover);
    font-size: 7px;
    font-weight: 700;
    color: var(--text-hint);
    flex-shrink: 0;
    margin-top: 1px;
    letter-spacing: 0.02em;
  }
  .sponsored-text {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .sponsored-item:hover .sponsored-text { color: var(--accent); }
  .sponsored-text strong {
    font-weight: 500;
    color: var(--text-primary);
  }
  .sponsored-item:hover .sponsored-text strong { color: var(--accent); }
  .sponsored-label {
    display: block;
    font-size: 10px;
    color: var(--text-hint);
    margin-top: 1px;
  }
</style>
