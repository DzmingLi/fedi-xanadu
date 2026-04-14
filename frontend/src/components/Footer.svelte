<script lang="ts">
  import { t } from '../lib/i18n/index.svelte';
</script>

<footer class="site-footer">
  <div class="footer-inner">
    <div class="footer-links">
      <a href="/about">{t('nav.about')}</a>
      <a href="/roadmap">{t('nav.roadmap')}</a>
      <a href="/guidelines">{t('footer.guidelines') || 'Guidelines'}</a>
      <a href="https://github.com/DzmingLi/fedi-xanadu" target="_blank" rel="noopener">GitHub</a>
    </div>
    <p class="footer-copy">&copy; {new Date().getFullYear()} NightBoat. {t('footer.rights') || 'All rights reserved.'}</p>
  </div>
</footer>

<style>
  .site-footer {
    margin-top: 3rem;
    padding: 1.5rem 1rem;
    border-top: 1px solid var(--border);
    text-align: center;
  }
  .footer-inner { max-width: 760px; margin: 0 auto; }
  .footer-links { display: flex; justify-content: center; flex-wrap: wrap; gap: 12px; margin-bottom: 8px; }
  .footer-links a { font-size: 13px; color: var(--text-hint); text-decoration: none; }
  .footer-links a:hover { color: var(--accent); }
  .footer-copy { font-size: 12px; color: var(--text-hint); margin: 0; }
</style>
