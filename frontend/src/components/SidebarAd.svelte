<script lang="ts">
  import { serveAd, recordAdClick, type AdSlot } from '../lib/api';

  const DEMO_AD: AdSlot = {
    id: 'demo',
    title: 'NightBoat — Knowledge Sharing Platform',
    body: 'Federated, prereq-aware academic content. Join the community.',
    image_url: null,
    link_url: '/about',
  };

  let ad = $state<AdSlot | null>(null);
  let fallback = $state(false);

  $effect(() => {
    loadAd();
  });

  async function loadAd() {
    try {
      const served = await serveAd('sidebar');
      if (served) { ad = served; return; }
    } catch { /* API not ready yet */ }
    // No direct-sales ad available — show demo for now
    ad = DEMO_AD;
  }

  function handleClick() {
    if (ad && ad.id !== 'demo') recordAdClick(ad.id).catch(() => {});
  }
</script>

{#if ad}
  <div class="ad-slot">
    <a href={ad.link_url} target={ad.id === 'demo' ? '_self' : '_blank'} rel="noopener sponsored" class="ad-link" onclick={handleClick}>
      {#if ad.image_url}
        <img src={ad.image_url} alt={ad.title} class="ad-img" />
      {/if}
      <span class="ad-title">{ad.title}</span>
      {#if ad.body}
        <span class="ad-body">{ad.body}</span>
      {/if}
    </a>
    <span class="ad-label">AD</span>
  </div>
{/if}

<style>
  .ad-slot {
    position: relative;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-white);
  }
  .ad-link {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-decoration: none;
    color: inherit;
  }
  .ad-link:hover { text-decoration: none; }
  .ad-img {
    width: 100%;
    border-radius: 4px;
    object-fit: cover;
  }
  .ad-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.4;
  }
  .ad-link:hover .ad-title { color: var(--accent); }
  .ad-body {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .ad-label {
    position: absolute;
    top: 4px;
    right: 6px;
    font-size: 9px;
    font-weight: 700;
    color: var(--text-hint);
    letter-spacing: 0.05em;
    opacity: 0.6;
  }
</style>
