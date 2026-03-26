<script lang="ts">
  import { listSkills, listTags, getAllArticlePrereqs, getAllArticleTags } from '../lib/api';
  import { tagName as resolveTagName } from '../lib/display';
  import { t } from '../lib/i18n';
  import type { UserSkill, Tag, ArticlePrereqBulkRow, ArticleTagRow } from '../lib/types';

  let skills = $state<UserSkill[]>([]);
  let tags = $state<Tag[]>([]);
  let allPrereqs = $state<ArticlePrereqBulkRow[]>([]);
  let allArticleTags = $state<ArticleTagRow[]>([]);
  let loading = $state(true);

  // Tags the user can explore: tags that appear on articles whose required prereqs are all satisfied
  let explorableTags = $derived.by(() => {
    if (loading) return [];

    const litTagIds = new Set(skills.map(s => s.tag_id));
    const tagMap = new Map(tags.map(t => [t.id, t]));
    const tagNameMap = new Map(tags.map(t => [t.id, resolveTagName(t.names, t.name, t.id)]));

    // Group prereqs by article
    const articlePrereqMap = new Map<string, ArticlePrereqBulkRow[]>();
    for (const p of allPrereqs) {
      const arr = articlePrereqMap.get(p.article_uri) || [];
      arr.push(p);
      articlePrereqMap.set(p.article_uri, arr);
    }

    // Find articles where all required prereqs are satisfied
    const reachableArticles = new Set<string>();
    // Get all unique article URIs (including those with no prereqs)
    const allArticleUris = new Set(allArticleTags.map(t => t.article_uri));
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
    for (const t of allArticleTags) {
      if (reachableArticles.has(t.article_uri) && !litTagIds.has(t.tag_id)) {
        tagCounts.set(t.tag_id, (tagCounts.get(t.tag_id) || 0) + 1);
      }
    }

    // Sort by article count descending
    return Array.from(tagCounts.entries())
      .map(([id, count]) => ({ id, name: tagNameMap.get(id) || id, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 15);
  });

  let litCount = $derived(skills.length);

  $effect(() => {
    Promise.all([listSkills(), listTags(), getAllArticlePrereqs(), getAllArticleTags()])
      .then(([sk, tg, pr, at]) => {
        skills = sk;
        tags = tg;
        allPrereqs = pr;
        allArticleTags = at;
        loading = false;
      });
  });
</script>

<aside class="right-sidebar">
  <div class="sidebar-section">
    <div class="sidebar-heading">{t('rsidebar.yourSkills')}</div>
    <p class="sidebar-text">{t('rsidebar.litTags', litCount)}</p>
    <a href="#/skills" class="sidebar-link-small">{t('rsidebar.manageTree')}</a>
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
          <a href="#/tag?id={encodeURIComponent(t.id)}" class="explore-tag">
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

  @media (max-width: 1100px) {
    .right-sidebar {
      display: none;
    }
  }
</style>
