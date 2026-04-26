<script lang="ts">
  import { getPaper } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import type { PaperDetailResponse } from '../lib/generated/PaperDetailResponse';

  let { id } = $props<{ id: string }>();

  let detail = $state<PaperDetailResponse | null>(null);
  let loading = $state(true);
  let error = $state('');

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  // Comments live at `paper:{id}` so they aggregate across PDF mirrors and the
  // native article body — that's the whole point of papers as a first-class
  // entity. Mirror the convention used by books / book-series.
  const contentUri = $derived(`paper:${id}`);

  // Per-version label for the pill next to the link.
  function versionBadge(kind: string): string {
    switch (kind) {
      case 'preprint':  return t('paper.preprint');
      case 'accepted':  return t('paper.accepted');
      case 'published': return t('paper.published');
      case 'native':    return t('paper.native');
      default:          return kind;
    }
  }

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      detail = await getPaper(id);
      document.title = `${loc(detail.paper.title)} — NightBoat`;
    } catch (e: any) {
      error = e?.message || 'Error loading paper';
    } finally {
      loading = false;
    }
  }

  // Authors line: prefer the curated `authors_detail` (links to author pages);
  // fall back to the plain string array on `paper.authors` when no curated
  // row is linked.
  function authorsLine(d: PaperDetailResponse): { name: string; id?: string }[] {
    if (d.authors_detail.length > 0) {
      return d.authors_detail.map(a => ({ name: a.name, id: a.author_id }));
    }
    return d.paper.authors.map(name => ({ name }));
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <article class="paper-page">
    <header class="paper-header">
      <h1 class="paper-title">{loc(detail.paper.title)}</h1>

      <div class="paper-meta">
        {#if authorsLine(detail).length > 0}
          <span class="paper-authors">
            {#each authorsLine(detail) as a, i}
              {#if a.id}
                <a href="/author?id={encodeURIComponent(a.id)}" class="paper-author">{a.name}</a>{#if i < authorsLine(detail).length - 1}, {/if}
              {:else}
                <span class="paper-author">{a.name}</span>{#if i < authorsLine(detail).length - 1}, {/if}
              {/if}
            {/each}
          </span>
        {/if}
        {#if detail.paper.venue}
          <span class="meta-sep">·</span>
          <span class="paper-venue">{detail.paper.venue}</span>
        {/if}
        {#if detail.paper.year}
          <span class="meta-sep">·</span>
          <span class="paper-year">{detail.paper.year}</span>
        {/if}
        {#if !detail.paper.accepted}
          <span class="paper-state">under review</span>
        {/if}
      </div>

      {#if detail.paper.doi || detail.paper.arxiv_id}
        <div class="paper-ids">
          {#if detail.paper.doi}
            <a class="paper-id" href="https://doi.org/{detail.paper.doi}" target="_blank" rel="noopener">DOI: {detail.paper.doi}</a>
          {/if}
          {#if detail.paper.arxiv_id}
            <a class="paper-id" href="https://arxiv.org/abs/{detail.paper.arxiv_id}" target="_blank" rel="noopener">arXiv: {detail.paper.arxiv_id}</a>
          {/if}
        </div>
      {/if}
    </header>

    {#if loc(detail.paper.abstract_)}
      <section class="paper-abstract">
        <h2>{t('paper.abstract')}</h2>
        <p>{loc(detail.paper.abstract_)}</p>
      </section>
    {/if}

    {#if detail.versions.length > 0}
      <section class="paper-versions">
        <h2>{t('paper.versions')}</h2>
        <ul class="version-list">
          {#each detail.versions as v}
            <li class="version-item">
              <span class="version-kind kind-{v.kind}">{versionBadge(v.kind)}</span>
              {#if v.kind === 'native' && v.article_uri}
                <a href="/article?uri={encodeURIComponent(v.article_uri)}" class="version-link">
                  {v.label || t('paper.readNative')}
                </a>
              {:else if v.url}
                <a href={v.url} target="_blank" rel="noopener" class="version-link">
                  {v.label || v.url}
                </a>
              {/if}
              {#if v.year}<span class="version-year">{v.year}</span>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="paper-discussion">
      <h2>{t('paper.discussion')}</h2>
      <CommentThread contentUri={contentUri} />
    </section>
  </article>
{/if}

<style>
  .paper-page { max-width: 760px; margin: 0 auto; padding: 24px 16px; }
  .paper-header { margin-bottom: 24px; }
  .paper-title { margin: 0 0 12px 0; font-size: 28px; line-height: 1.25; color: var(--text-primary); }
  .paper-meta { font-size: 14px; color: var(--text-secondary); display: flex; flex-wrap: wrap; gap: 6px; align-items: baseline; }
  .paper-author { color: var(--text-primary); text-decoration: none; }
  a.paper-author:hover { color: var(--accent); text-decoration: underline; }
  .meta-sep { opacity: 0.6; }
  .paper-state { color: var(--accent); font-style: italic; margin-left: 6px; }
  .paper-ids { margin-top: 8px; display: flex; gap: 12px; flex-wrap: wrap; }
  .paper-id { font-size: 13px; color: var(--accent); text-decoration: none; font-family: monospace; }
  .paper-id:hover { text-decoration: underline; }
  .paper-abstract { margin-bottom: 24px; padding: 12px 16px; border-left: 2px solid var(--border); }
  .paper-abstract h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.04em; margin: 0 0 6px 0; color: var(--text-secondary); }
  .paper-abstract p { margin: 0; line-height: 1.6; }
  .paper-versions { margin-bottom: 24px; }
  .paper-versions h2 { font-size: 16px; margin: 0 0 8px 0; }
  .version-list { list-style: none; padding: 0; margin: 0; }
  .version-item { display: flex; align-items: baseline; gap: 8px; padding: 6px 0; border-bottom: 1px solid var(--border-faint); }
  .version-kind { font-size: 11px; padding: 2px 8px; border-radius: 10px; background: var(--surface-2); text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-secondary); }
  .kind-native { background: var(--accent); color: var(--surface); }
  .version-link { color: var(--text-primary); text-decoration: none; flex: 1; }
  .version-link:hover { color: var(--accent); text-decoration: underline; }
  .version-year { font-size: 12px; color: var(--text-secondary); }
  .paper-discussion h2 { font-size: 16px; margin: 24px 0 12px 0; }
  .meta { color: var(--text-secondary); }
  .error { color: red; }
</style>
