<script lang="ts">
  import { marked } from 'marked';
  import aboutMd from '../content/about.md?raw';

  const html = marked.parse(aboutMd) as string;
</script>

<div class="content">
  {@html html}
</div>

<style>
  .content {
    font-family: var(--font-serif);
    font-size: 1.125rem;
    line-height: 1.7;
    color: var(--text-primary);
  }
  .content :global(h1) {
    font-family: var(--font-serif);
    font-weight: 400;
  }
  .content :global(h2) {
    font-family: var(--font-serif);
    font-weight: 400;
    margin-top: 2.5rem;
    padding-bottom: 0.25em;
    border-bottom: 1px solid var(--border);
  }
  .content :global(h3) {
    font-family: var(--font-serif);
    margin-top: 1.75rem;
  }
  .content :global(p) {
    margin: 1em 0;
  }
  .content :global(ul) {
    margin: 1em 0;
    padding-left: 1.5rem;
  }
  .content :global(li) {
    margin: 0.4rem 0;
  }
  .content :global(a) {
    color: var(--accent);
    text-decoration: none;
  }
  .content :global(a:hover) {
    text-decoration: underline;
  }
  .content :global(strong) {
    font-weight: 600;
  }
</style>
