<script lang="ts">
  import { onMount } from 'svelte';
  import { searchArticles, getAllArticleTeaches, getAllArticlePrereqs } from '../lib/api';

  // Svelte action: focus the input once mounted (preferred over the native
  // `autofocus` attribute, which svelte-check flags as an a11y pitfall).
  function focusOnMount(el: HTMLInputElement) {
    el.focus();
  }
  import { t } from '../lib/i18n/index.svelte';
  import type { Article, ContentTeachRow, ContentPrereqBulkRow } from '../lib/types';
  import PostCard from '../lib/components/PostCard.svelte';
  import { navigate } from '../lib/router';

  let { q = '' }: { q?: string } = $props();

  let results = $state<Article[]>([]);
  let teaches = $state(new Map<string, ContentTeachRow[]>());
  let prereqs = $state(new Map<string, ContentPrereqBulkRow[]>());
  let loading = $state(false);
  // svelte-ignore state_referenced_locally
  let query = $state(q);

  function buildRowMap<T extends { content_uri: string }>(rows: T[]): Map<string, T[]> {
    const m = new Map<string, T[]>();
    for (const r of rows) {
      const arr = m.get(r.content_uri) ?? [];
      arr.push(r);
      m.set(r.content_uri, arr);
    }
    return m;
  }

  async function runSearch(qq: string) {
    if (!qq.trim()) { results = []; return; }
    loading = true;
    try {
      const [arts, tag, pre] = await Promise.all([
        searchArticles(qq, 50),
        getAllArticleTeaches(),
        getAllArticlePrereqs(),
      ]);
      results = arts;
      teaches = buildRowMap(tag);
      prereqs = buildRowMap(pre);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    document.title = q ? `${q} — ${t('nav.search')} — NightBoat` : `${t('nav.search')} — NightBoat`;
    runSearch(q);
  });

  $effect(() => {
    if (query !== q) {
      document.title = query ? `${query} — ${t('nav.search')} — NightBoat` : `${t('nav.search')} — NightBoat`;
      runSearch(query);
    }
  });

  function onSubmit(e: Event) {
    e.preventDefault();
    navigate(`/search?q=${encodeURIComponent(query.trim())}`);
  }
</script>

<div class="search-page">
  <form class="search-form" onsubmit={onSubmit}>
    <input
      type="text"
      class="search-box"
      bind:value={query}
      placeholder={t('nav.search')}
      use:focusOnMount
    />
  </form>

  {#if loading}
    <p class="state">{t('common.loading')}</p>
  {:else if !q.trim()}
    <p class="state">{t('search.typeToSearch')}</p>
  {:else if results.length === 0}
    <p class="state">{t('search.noResults')}</p>
  {:else}
    <p class="count">{results.length} {t('search.resultsSuffix')}</p>
    {#each results as article (article.at_uri)}
      <PostCard
        {article}
        articleTeaches={teaches.get(article.at_uri) ?? []}
        articlePrereqs={prereqs.get(article.at_uri) ?? []}
      />
    {/each}
  {/if}
</div>

<style>
  .search-page {
    max-width: 800px;
    margin: 0 auto;
    padding: 24px 16px;
  }
  .search-form { margin-bottom: 24px; }
  .search-box {
    width: 100%;
    padding: 12px 16px;
    font-size: 16px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-white);
    color: var(--text-primary);
    box-sizing: border-box;
  }
  .search-box:focus {
    outline: none;
    border-color: var(--accent);
  }
  .state {
    text-align: center;
    color: var(--text-secondary);
    margin-top: 40px;
  }
  .count {
    color: var(--text-hint);
    font-size: 13px;
    margin: 0 0 16px;
  }
</style>
