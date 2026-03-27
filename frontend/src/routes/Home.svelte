<script lang="ts" module>
  // Module-level cache — survives component destroy/recreate during SPA navigation
  import type { Article, ContentTeachRow, ContentPrereqBulkRow, Tag, TagTreeEntry, Series } from '../lib/types';
  let _cache: {
    articles: Article[];
    allTags: Tag[];
    tagTree: TagTreeEntry[];
    allSeries: Series[];
    seriesArticleUris: Set<string>;
    seriesArticleMap: Map<string, string[]>;
    articleTeaches: Map<string, ContentTeachRow[]>;
    articlePrereqs: Map<string, ContentPrereqBulkRow[]>;
    interests: string[];
    ts: number;
  } | null = null;
</script>

<script lang="ts">
  import { listArticles, getAllArticleTeaches, getAllArticlePrereqs, listTags, getTagTree, getInterests, setInterests as apiSetInterests, listSeries, getAllSeriesArticles } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { authorName, tagName, deduplicateByTranslation } from '../lib/display';
  import { t, onLocaleChange, getLocale } from '../lib/i18n';
  import { buildSeriesArticleMaps, buildArticleRowMap } from '../lib/series';
  import PostCard from '../lib/components/PostCard.svelte';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let articles = $state<Article[]>(_cache?.articles ?? []);
  let allTags = $state<Tag[]>(_cache?.allTags ?? []);
  let tagTree = $state<TagTreeEntry[]>(_cache?.tagTree ?? []);
  let loading = $state(!_cache);

  let articleTeaches = $state(new Map<string, ContentTeachRow[]>(_cache?.articleTeaches ?? []));
  let articlePrereqs = $state(new Map<string, ContentPrereqBulkRow[]>(_cache?.articlePrereqs ?? []));

  // Series data
  let allSeries = $state<Series[]>(_cache?.allSeries ?? []);
  let seriesArticleUris = $state(new Set<string>(_cache?.seriesArticleUris ?? []));
  let seriesArticleMap = $state(new Map<string, string[]>(_cache?.seriesArticleMap ?? []));

  // --- Interest categories ---
  const STORAGE_KEY = 'fx_interests';
  let interests = $state<string[]>(_cache?.interests ?? loadLocalInterests());
  let showInterestPicker = $state(false);
  let interestsLoaded = $state(!!_cache);

  function loadLocalInterests(): string[] {
    try {
      const s = localStorage.getItem(STORAGE_KEY);
      return s ? JSON.parse(s) : [];
    } catch { return []; }
  }
  function saveInterests() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(interests));
    // If logged in, also save to server
    if (getAuth()) {
      apiSetInterests(interests).catch(() => {});
    }
  }

  // Top-level field categories — always show these four
  const FIELD_IDS = ['math', 'physics', 'cs', 'economics'];
  function fieldFallback(id: string): string {
    return t(`field.${id}`) !== `field.${id}` ? t(`field.${id}`) : id;
  }
  let topCategories = $derived.by(() => {
    const tagMap = new Map(allTags.map(t => [t.id, t]));
    // Also discover additional roots from tagTree
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
    sortKey: number; // for ordering
  }

  // Standalone articles (not in any series)
  let standaloneArticles = $derived(
    articles.filter(a => !seriesArticleUris.has(a.at_uri))
  );

  function buildFeed(arts: Article[]): FeedItem[] {
    const items: FeedItem[] = [];
    const artUriSet = new Set(arts.map(a => a.at_uri));

    // Add standalone articles (not in any series)
    for (const a of arts) {
      if (!seriesArticleUris.has(a.at_uri)) {
        items.push({
          type: 'article',
          article: a,
          sortKey: trendingScore(a),
        });
      }
    }

    // Add series cards — only top-level series (no parent)
    // Collect all descendant article URIs for each top-level series
    const childSeriesOf = new Map<string, string[]>();
    for (const s of allSeries) {
      if (s.parent_id) {
        const arr = childSeriesOf.get(s.parent_id) || [];
        arr.push(s.id);
        childSeriesOf.set(s.parent_id, arr);
      }
    }

    for (const s of allSeries) {
      if (s.parent_id) continue; // skip sub-series

      // Gather articles from this series and all descendant sub-series
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
    // Deduplicate translations: show only the locale-preferred version
    candidateArticles = deduplicateByTranslation(candidateArticles, locale);
    return buildFeed(candidateArticles);
  });

  // Tabs to show: selected interests
  let visibleTabs = $derived(
    topCategories.filter(c => interests.includes(c.id))
  );

  $effect(() => {
    const fetchData = async () => {
      const [arts, tags, prereqs, tgs, tree, seriesList, seriesArts] = await Promise.all([
        listArticles(),
        getAllArticleTeaches(),
        getAllArticlePrereqs(),
        listTags(),
        getTagTree(),
        listSeries(),
        getAllSeriesArticles(),
      ]);

      articles = arts;
      allTags = tgs;
      tagTree = tree;
      allSeries = seriesList;

      // Build series article membership
      const saMaps = buildSeriesArticleMaps(seriesArts);
      seriesArticleUris = saMaps.seriesArticleUris;
      seriesArticleMap = saMaps.seriesArticleMap;

      articleTeaches = buildArticleRowMap(tags);
      articlePrereqs = buildArticleRowMap(prereqs);

      // Load interests from server if logged in
      if (getAuth()) {
        try {
          const serverInterests = await getInterests();
          if (serverInterests.length > 0) {
            interests = serverInterests;
            localStorage.setItem(STORAGE_KEY, JSON.stringify(interests));
          }
        } catch { /* use local */ }
      }
      interestsLoaded = true;

      loading = false;

      // Update module cache
      _cache = {
        articles, allTags, tagTree, allSeries,
        seriesArticleUris, seriesArticleMap,
        articleTeaches, articlePrereqs, interests,
        ts: Date.now(),
      };

      // Show picker if no interests saved and we have categories
      if (interests.length === 0 && topCategories.length > 0) {
        showInterestPicker = true;
      }
    };
    fetchData();
  });

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
      <p class="meta"><a href="#/new">{t('home.writeOne')}</a></p>
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
  .picker-desc {
    font-size: 11px;
    color: var(--text-hint);
    line-height: 1.3;
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

  /* Card styles are now in PostCard.svelte */
</style>
