<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { listArticles, getAllArticleTeaches, getAllArticlePrereqs, listTags, getTagTree, getInterests, setInterests as apiSetInterests, listSeries, getAllSeriesArticles } from '$lib/api';
  import { getAuth } from '$lib/auth.svelte';
  import { authorName, tagName, deduplicateByTranslation, deduplicateSeriesByTranslation } from '$lib/display';
  import { t, onLocaleChange, getLocale } from '$lib/i18n/index.svelte';
  import { buildSeriesArticleMaps, buildArticleRowMap } from '$lib/series';
  import PostCard from '$lib/components/PostCard.svelte';
  import { keys } from '$lib/queries';
  import type { Article, ContentTeachRow, ContentPrereqBulkRow, Tag, TagTreeEntry, Series } from '$lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  // --- TanStack Query data fetching ---
  const articlesQuery = createQuery({ queryKey: keys.articles.all, queryFn: listArticles });
  const teachesQuery = createQuery({ queryKey: keys.articles.teaches, queryFn: getAllArticleTeaches });
  const prereqsQuery = createQuery({ queryKey: keys.articles.prereqs, queryFn: getAllArticlePrereqs });
  const tagsQuery = createQuery({ queryKey: keys.tags.all, queryFn: listTags });
  const tagTreeQuery = createQuery({ queryKey: keys.tags.tree, queryFn: getTagTree });
  const seriesQuery = createQuery({ queryKey: keys.series.all, queryFn: listSeries });
  const seriesArtsQuery = createQuery({ queryKey: keys.series.allArticles, queryFn: getAllSeriesArticles });

  let loading = $derived(
    $articlesQuery.isLoading || $tagsQuery.isLoading || $tagTreeQuery.isLoading ||
    $seriesQuery.isLoading || $seriesArtsQuery.isLoading || $teachesQuery.isLoading || $prereqsQuery.isLoading
  );

  let articles = $derived($articlesQuery.data ?? []);
  let allTags = $derived($tagsQuery.data ?? []);
  let tagTree = $derived($tagTreeQuery.data ?? []);
  let allSeries = $derived($seriesQuery.data ?? []);

  let articleTeaches = $derived(buildArticleRowMap($teachesQuery.data ?? []));
  let articlePrereqs = $derived(buildArticleRowMap($prereqsQuery.data ?? []));

  // Build series article membership
  let seriesMaps = $derived(buildSeriesArticleMaps($seriesArtsQuery.data ?? []));
  let seriesArticleUris = $derived(seriesMaps.seriesArticleUris);
  let seriesArticleMap = $derived(seriesMaps.seriesArticleMap);

  // --- Interest categories ---
  const STORAGE_KEY = 'fx_interests';
  let interests = $state<string[]>(loadLocalInterests());
  let showInterestPicker = $state(false);
  let interestsLoaded = $state(false);

  function loadLocalInterests(): string[] {
    try {
      const s = localStorage.getItem(STORAGE_KEY);
      return s ? JSON.parse(s) : [];
    } catch { return []; }
  }
  function saveInterests() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(interests));
    if (getAuth()) {
      apiSetInterests(interests).catch(() => {});
    }
  }

  // Load interests from server when data is ready
  $effect(() => {
    if (loading || interestsLoaded) return;
    interestsLoaded = true;
    if (getAuth()) {
      getInterests().then(serverInterests => {
        if (serverInterests.length > 0) {
          interests = serverInterests;
          localStorage.setItem(STORAGE_KEY, JSON.stringify(interests));
        }
      }).catch(() => {});
    }
    if (interests.length === 0 && topCategories.length > 0) {
      showInterestPicker = true;
    }
  });

  // Top-level field categories — always show these four
  const FIELD_IDS = ['math', 'physics', 'cs', 'economics'];
  function fieldFallback(id: string): string {
    return t(`field.${id}`) !== `field.${id}` ? t(`field.${id}`) : id;
  }
  let topCategories = $derived.by(() => {
    const tagMap = new Map(allTags.map(t => [t.id, t]));
    const hasParent = new Set<string>();
    const isParent = new Set<string>();
    for (const e of tagTree) {
      hasParent.add(e.child_tag);
      isParent.add(e.parent_tag);
    }
    const extraRoots = Array.from(isParent)
      .filter(id => !hasParent.has(id) && !FIELD_IDS.includes(id));

    const allRoots = [...FIELD_IDS, ...extraRoots];
    return allRoots
      .map(id => tagMap.get(id) ?? { id, name: fieldFallback(id), description: null, created_by: 'system', created_at: '' } as Tag)
      .filter((t): t is Tag => !!t);
  });

  // Build set of all descendant tag IDs for each top category
  let categoryDescendants = $derived.by(() => {
    const childrenOf = new Map<string, string[]>();
    for (const e of tagTree) {
      const arr = childrenOf.get(e.parent_tag) || [];
      arr.push(e.child_tag);
      childrenOf.set(e.parent_tag, arr);
    }
    const result = new Map<string, Set<string>>();
    for (const cat of topCategories) {
      const descendants = new Set<string>();
      const stack = [cat.id];
      while (stack.length > 0) {
        const cur = stack.pop()!;
        descendants.add(cur);
        for (const child of (childrenOf.get(cur) || [])) {
          if (!descendants.has(child)) {
            descendants.add(child);
            stack.push(child);
          }
        }
      }
      result.set(cat.id, descendants);
    }
    return result;
  });

  // Active tab
  let activeTab = $state('all');

  // Trending score: combines recency and votes
  function trendingScore(a: Article): number {
    const score = a.vote_score || 0;
    const created = new Date(a.created_at).getTime();
    const now = Date.now();
    const ageHours = Math.max(1, (now - created) / (1000 * 60 * 60));
    return (score + 1) / Math.pow(ageHours, 1.5);
  }

  // Feed items: standalone articles + series cards
  interface FeedItem {
    type: 'article' | 'series';
    article?: Article;
    series?: Series;
    articleCount?: number;
    sortKey: number;
  }

  // Standalone articles (not in any series)
  let standaloneArticles = $derived(
    articles.filter(a => !seriesArticleUris.has(a.at_uri))
  );

  function buildFeed(arts: Article[]): FeedItem[] {
    const items: FeedItem[] = [];
    const artUriSet = new Set(arts.map(a => a.at_uri));

    for (const a of arts) {
      if (!seriesArticleUris.has(a.at_uri)) {
        items.push({
          type: 'article',
          article: a,
          sortKey: trendingScore(a),
        });
      }
    }

    const dedupedSeries = deduplicateSeriesByTranslation(allSeries, locale);
    const childSeriesOf = new Map<string, string[]>();
    for (const s of dedupedSeries) {
      if (s.parent_id) {
        const arr = childSeriesOf.get(s.parent_id) || [];
        arr.push(s.id);
        childSeriesOf.set(s.parent_id, arr);
      }
    }

    for (const s of dedupedSeries) {
      if (s.parent_id) continue;

      const allMemberUris: string[] = [];
      const stack = [s.id];
      while (stack.length > 0) {
        const sid = stack.pop()!;
        const uris = seriesArticleMap.get(sid) || [];
        allMemberUris.push(...uris);
        for (const child of (childSeriesOf.get(sid) || [])) {
          stack.push(child);
        }
      }
      if (allMemberUris.length === 0) continue;

      const memberArts = allMemberUris
        .map(uri => articles.find(a => a.at_uri === uri))
        .filter((a): a is Article => !!a);

      const hasMatch = memberArts.some(a => artUriSet.has(a.at_uri));
      if (!hasMatch) continue;

      const maxScore = memberArts.reduce((acc, a) => Math.max(acc, trendingScore(a)), 0);
      items.push({
        type: 'series',
        series: s,
        articleCount: allMemberUris.length,
        sortKey: maxScore,
      });
    }

    items.sort((a, b) => b.sortKey - a.sortKey);
    return items;
  }

  // Filtered feed items for current tab
  let filteredFeed = $derived.by(() => {
    let candidateArticles: Article[];
    if (activeTab === 'all') {
      candidateArticles = [...articles].sort((a, b) => trendingScore(b) - trendingScore(a));
    } else {
      const desc = categoryDescendants.get(activeTab);
      if (!desc) return [];
      candidateArticles = articles.filter(a => {
        const tags = articleTeaches.get(a.at_uri) || [];
        return tags.some(t => desc.has(t.tag_id));
      });
    }
    candidateArticles = deduplicateByTranslation(candidateArticles, locale);
    return buildFeed(candidateArticles);
  });

  // Tabs to show: selected interests
  let visibleTabs = $derived(
    topCategories.filter(c => interests.includes(c.id))
  );

  function toggleInterest(id: string) {
    if (interests.includes(id)) {
      interests = interests.filter(i => i !== id);
    } else {
      interests = [...interests, id];
    }
  }

  function confirmInterests() {
    saveInterests();
    showInterestPicker = false;
    activeTab = 'all';
  }
</script>

<!-- Interest picker modal -->
{#if showInterestPicker && !loading}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="picker-overlay" onclick={() => { if (interests.length > 0) confirmInterests(); }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="picker-modal" onclick={(e) => e.stopPropagation()}>
      <h2>{t('home.selectInterests')}</h2>
      <p class="picker-hint">{t('home.interestHint')}</p>
      <div class="picker-grid">
        {#each topCategories as cat}
          <button
            class="picker-item"
            class:selected={interests.includes(cat.id)}
            onclick={() => toggleInterest(cat.id)}
          >
            <span class="picker-name">{cat.name}</span>
          </button>
        {/each}
      </div>
      <div class="picker-actions">
        <button class="picker-confirm" onclick={confirmInterests} disabled={interests.length === 0}>
          {t('home.confirm')} ({interests.length} {t('home.fields')})
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Main content -->
<div class="home-header">
  <h1>{interests.length === 0 ? t('home.trending') : t('home.recent')}</h1>
  <button class="edit-interests" onclick={() => { showInterestPicker = true; }} title={t('home.selectInterests')}>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
  </button>
</div>

{#if loading}
  <p class="meta">Loading...</p>
{:else}
  <!-- Category tabs -->
  {#if visibleTabs.length > 0}
    <div class="tab-bar">
      <button class="tab" class:active={activeTab === 'all'} onclick={() => { activeTab = 'all'; }}>
        {t('home.all')}
      </button>
      {#each visibleTabs as cat}
        <button class="tab" class:active={activeTab === cat.id} onclick={() => { activeTab = cat.id; }}>
          {cat.name}
        </button>
      {/each}
    </div>
  {/if}

  {#if filteredFeed.length === 0}
    <div class="empty">
      <p>{t('home.noArticles')}</p>
      <p class="meta"><a href="/new">{t('home.writeOne')}</a></p>
    </div>
  {:else}
    {#each filteredFeed as item}
      {#if item.type === 'article' && item.article}
        <PostCard
          article={item.article}
          articleTeaches={articleTeaches.get(item.article.at_uri) || []}
          articlePrereqs={articlePrereqs.get(item.article.at_uri) || []}
          variant="home"
        />
      {:else if item.type === 'series' && item.series}
        <PostCard
          series={item.series}
          articleCount={item.articleCount}
          variant="home"
        />
      {/if}
    {/each}
  {/if}
{/if}

<style>
  /* Interest picker modal */
  .picker-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.4);
    z-index: 300;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 10vh;
  }
  .picker-modal {
    width: 520px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 8px;
    padding: 28px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.18);
  }
  .picker-modal h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 0 0 4px;
    font-size: 1.3rem;
  }
  .picker-hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 20px;
  }
  .picker-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
  }
  .picker-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 16px 12px;
    border: 2px solid var(--border);
    border-radius: 8px;
    background: var(--bg-white);
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }
  .picker-item:hover {
    border-color: var(--accent);
  }
  .picker-item.selected {
    border-color: var(--accent);
    background: rgba(95, 155, 101, 0.08);
  }
  .picker-name {
    font-family: var(--font-serif);
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .picker-actions {
    margin-top: 20px;
    display: flex;
    justify-content: flex-end;
  }
  .picker-confirm {
    padding: 8px 20px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .picker-confirm:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Header */
  .home-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }
  .home-header h1 {
    margin: 0;
  }
  .edit-interests {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    padding: 4px;
    display: flex;
    transition: color 0.15s;
  }
  .edit-interests:hover { color: var(--accent); }

  /* Tab bar */
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 16px;
    overflow-x: auto;
  }
  .tab {
    padding: 8px 16px;
    font-size: 14px;
    font-family: var(--font-sans);
    color: var(--text-secondary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab:hover {
    color: var(--text-primary);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    font-weight: 500;
  }
</style>
