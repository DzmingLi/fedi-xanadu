<script lang="ts">
  import { listSkills, listTags, getAllArticlePrereqs, getAllArticleTeaches, getRecommendedQuestions } from '../lib/api';
  import { tagName as resolveTagName, authorName } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Article, UserSkill, Tag, ContentPrereqBulkRow, ContentTeachRow } from '../lib/types';

  let skills = $state<UserSkill[]>([]);
  let tags = $state<Tag[]>([]);
  let allPrereqs = $state<ContentPrereqBulkRow[]>([]);
  let allArticleTeaches = $state<ContentTeachRow[]>([]);
  let loading = $state(true);
  let questions = $state<Article[]>([]);
  let questionsLoading = $state(true);

  // Tags the user can explore: tags that appear on articles whose required prereqs are all satisfied
  let explorableTags = $derived.by(() => {
    if (loading) return [];

    const litTagIds = new Set(skills.map(s => s.tag_id));
    const tagMap = new Map(tags.map(t => [t.id, t]));
    const tagNameMap = new Map(tags.map(t => [t.id, resolveTagName(t.names, t.name, t.id)]));

    // Group prereqs by article
    const articlePrereqMap = new Map<string, ContentPrereqBulkRow[]>();
    for (const p of allPrereqs) {
      const arr = articlePrereqMap.get(p.content_uri) || [];
      arr.push(p);
      articlePrereqMap.set(p.content_uri, arr);
    }

    // Find articles where all required prereqs are satisfied
    const reachableArticles = new Set<string>();
    // Get all unique article URIs (including those with no prereqs)
    const allArticleUris = new Set(allArticleTeaches.map(t => t.content_uri));
    for (const uri of allArticleUris) {
      const prereqs = articlePrereqMap.get(uri) || [];
      const requiredPrereqs = prereqs.filter(p => p.prereq_type === 'required');
      const allMet = requiredPrereqs.every(p => litTagIds.has(p.tag_id));
      if (allMet) {
        reachableArticles.add(uri);
      }
    }

    // Collect tags from reachable articles, excluding already-lit tags
    const tagCounts = new Map<string, number>();
    for (const t of allArticleTeaches) {
      if (reachableArticles.has(t.content_uri) && !litTagIds.has(t.tag_id)) {
        tagCounts.set(t.tag_id, (tagCounts.get(t.tag_id) || 0) + 1);
      }
    }

    // Sort by article count descending
    return Array.from(tagCounts.entries())
      .map(([id, count]) => ({ id, name: tagNameMap.get(id) || id, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 15);
  });

  $effect(() => {
    Promise.all([listSkills(), listTags(), getAllArticlePrereqs(), getAllArticleTeaches()])
      .then(([sk, tg, pr, at]) => {
        skills = sk;
        tags = tg;
        allPrereqs = pr;
        allArticleTeaches = at;
        loading = false;
      });
    getRecommendedQuestions(6)
      .then(qs => { questions = qs; })
      .catch(() => {})
      .finally(() => { questionsLoading = false; });
  });
</script>

<aside class="right-sidebar">
  <!-- Recommended questions -->
  <div class="sidebar-section">
    <div class="sidebar-heading">{t('rsidebar.questionsForYou')}</div>
    {#if questionsLoading}
      <p class="sidebar-text">{t('common.loading')}</p>
    {:else if questions.length === 0}
      <p class="sidebar-text">{t('rsidebar.noQuestions')}</p>
    {:else}
      <div class="question-list">
        {#each questions as q}
          <a href="/article?uri={encodeURIComponent(q.at_uri)}" class="q-card">
            <span class="q-title">{q.title}</span>
            <span class="q-meta">
              {authorName(q)} &middot; {t('rsidebar.answersCount', q.answer_count)}
              {#if q.vote_score > 0}
                &middot; &#9650;{q.vote_score}
              {/if}
            </span>
          </a>
        {/each}
      </div>
      <a href="/questions" class="sidebar-link-small">{t('rsidebar.viewAllQuestions')}</a>
    {/if}
  </div>

  <div class="sidebar-divider"></div>

  <div class="sidebar-section">
    <div class="sidebar-heading">{t('rsidebar.explore')}</div>
    {#if loading}
      <p class="sidebar-text">{t('common.loading')}</p>
    {:else if explorableTags.length === 0}
      <p class="sidebar-text">{t('rsidebar.lightMoreHint')}</p>
    {:else}
      <div class="explore-tags">
        {#each explorableTags as t}
          <a href="/tag?id={encodeURIComponent(t.id)}" class="explore-tag">
            <span class="explore-tag-name">{t.name}</span>
            <span class="explore-tag-count">{t.count}</span>
          </a>
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  .right-sidebar {
    position: sticky;
    top: 4rem;
    width: 200px;
    flex-shrink: 0;
    align-self: flex-start;
    padding-top: 0.5rem;
  }
  .sidebar-section {
    padding: 8px 10px;
  }
  .sidebar-heading {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 6px;
  }
  .sidebar-text {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 4px 0;
  }
  .sidebar-link-small {
    font-size: 12px;
    color: var(--accent);
    text-decoration: none;
  }
  .sidebar-link-small:hover {
    text-decoration: underline;
  }
  .sidebar-divider {
    height: 1px;
    background: var(--border);
    margin: 8px 10px;
  }

  .explore-tags {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .explore-tag {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-radius: 3px;
    text-decoration: none;
    transition: background 0.1s;
  }
  .explore-tag:hover {
    background: var(--bg-hover);
    text-decoration: none;
  }
  .explore-tag-name {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .explore-tag:hover .explore-tag-name {
    color: var(--text-primary);
  }
  .explore-tag-count {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-hover);
    padding: 1px 6px;
    border-radius: 8px;
    min-width: 20px;
    text-align: center;
  }

  /* Question cards */
  .question-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 6px;
  }
  .q-card {
    display: block;
    padding: 5px 8px;
    border-radius: 3px;
    text-decoration: none;
    border-left: 2px solid #d97706;
    transition: background 0.1s;
  }
  .q-card:hover {
    background: var(--bg-hover);
    text-decoration: none;
  }
  .q-title {
    display: block;
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .q-card:hover .q-title {
    color: var(--accent);
  }
  .q-meta {
    display: block;
    font-size: 11px;
    color: var(--text-hint);
    margin-top: 2px;
  }

  @media (max-width: 1100px) {
    .right-sidebar {
      display: none;
    }
  }
</style>
