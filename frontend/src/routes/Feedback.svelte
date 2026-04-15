<script lang="ts">
  import { t } from '../lib/i18n/index.svelte';
  document.title = `${t('footer.feedback') || 'Feedback'} — NightBoat`;
</script>

<div class="feedback-page">
  <h1>{t('footer.feedback') || 'Feedback'}</h1>
  <p class="placeholder">{t('feedback.comingSoon') || 'Bug reports and feedback system is coming soon.'}</p>
  <p class="placeholder">{t('feedback.githubHint') || 'In the meantime, please report issues on'} <a href="https://github.com/DzmingLi/fedi-xanadu/issues" target="_blank" rel="noopener">GitHub Issues</a>.</p>
</div>

<style>
  .feedback-page { max-width: 700px; margin: 0 auto; }
  .feedback-page h1 { font-family: var(--font-serif); font-weight: 400; }
  .placeholder { color: var(--text-hint); font-size: 14px; }
  .placeholder a { color: var(--accent); }
</style>
