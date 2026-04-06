<script lang="ts">
  import { marked } from 'marked';
  import roadmapMd from '../content/roadmap.md?raw';

  const html = marked.parse(roadmapMd) as string;
</script>

<div class="content roadmap-content">
  {@html html}
</div>

<style>
  .content {
    font-family: var(--font-sans);
    font-size: 0.95rem;
    line-height: 1.7;
    color: var(--text-primary);
  }
  .content :global(h1) {
    font-family: var(--font-serif);
    font-weight: 400;
    margin-bottom: 0;
  }
  .content :global(h2) {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.1rem;
    margin-top: 2rem;
    margin-bottom: 0.5rem;
    color: var(--text-secondary);
  }
  .content :global(p) {
    margin: 0.5em 0 1em;
    color: var(--text-secondary);
    font-size: 0.875rem;
  }
  .content :global(ul) {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .content :global(li) {
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    font-size: 0.9rem;
    line-height: 1.5;
  }
  .content :global(li strong) {
    font-weight: 500;
    color: var(--text-primary);
  }
  /* Section-specific dot colors via h2 sibling targeting not possible in CSS;
     use class on the section wrapper via JS-free approach: color the em-dash separator */
  .roadmap-content :global(h2:nth-of-type(1)) { color: #22c55e; }  /* ✅ done */
  .roadmap-content :global(h2:nth-of-type(2)) { color: #f59e0b; }  /* 🚧 in-progress */
  .roadmap-content :global(h2:nth-of-type(3)) { color: var(--text-hint); } /* 📋 planned */
  .content :global(a) {
    color: var(--accent);
    text-decoration: none;
  }
  .content :global(a:hover) {
    text-decoration: underline;
  }
</style>
