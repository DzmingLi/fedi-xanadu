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
  <a
    href={ad.link_url}
    target="_blank"
    rel="noopener sponsored"
    class="happening-item"
    onclick={handleClick}
  >
    <span class="happening-icon">📌</span>
    <span class="happening-text">
      {ad.title}{#if ad.body} — {ad.body}{/if}
    </span>
  </a>
{/if}
