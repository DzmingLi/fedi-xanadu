<script lang="ts">
  import { onMount } from 'svelte';
  import { serveAd, recordAdClick, type AdSlot } from '../lib/api';

  let ad = $state<AdSlot | null>(null);

  onMount(() => {
    serveAd('sidebar').then(served => {
      ad = served;
    }).catch(() => {});
  });

  function handleClick() {
    if (ad) recordAdClick(ad.id).catch(() => {});
  }
</script>

{#if ad}
  <div style="padding: 4px 10px;">
    <a
      href={ad.link_url}
      target="_blank"
      rel="noopener sponsored"
      onclick={handleClick}
      style="display: block; padding: 10px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-white); text-decoration: none; color: inherit; transition: border-color 0.15s;"
    >
      {#if ad.image_url}
        <img src={ad.image_url} alt="" style="width: 100%; border-radius: 4px; margin-bottom: 6px; object-fit: cover;" />
      {/if}
      <div style="font-size: 13px; font-weight: 500; color: var(--text-primary); line-height: 1.4;">{ad.title}</div>
      {#if ad.body}
        <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.4; margin-top: 4px;">{ad.body}</div>
      {/if}
      <div style="display: flex; align-items: center; justify-content: space-between; margin-top: 6px;">
        <span style="font-size: 10px; color: var(--text-hint);">Sponsored</span>
        <span style="font-size: 10px; color: var(--text-hint);">{new URL(ad.link_url, location.href).hostname.replace('www.', '')}</span>
      </div>
    </a>
  </div>
{/if}
