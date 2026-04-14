<script lang="ts">
  import { t } from '../lib/i18n/index.svelte';
  document.title = `${t('footer.guidelines') || 'Community Guidelines'} — NightBoat`;
</script>

<div class="guidelines-page">
  <h1>{t('footer.guidelines') || 'Community Guidelines'}</h1>
  <p class="placeholder">{t('guidelines.comingSoon') || 'Community guidelines are being drafted. Check back soon.'}</p>
</div>

<style>
  .guidelines-page { max-width: 700px; margin: 0 auto; }
  .guidelines-page h1 { font-family: var(--font-serif); font-weight: 400; }
  .placeholder { color: var(--text-hint); font-size: 14px; padding: 2rem 0; }
</style>
