<script lang="ts">
  import { getTag, getArticlesByTag, listSkills, lightSkill, unlightSkill, getArticleVotes } from '../lib/api';
  import { authorName, tagName } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Tag, Article, UserSkill, VoteSummary } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let tag = $state<Tag | null>(null);
  let articles = $state<Article[]>([]);
  let skills = $state<UserSkill[]>([]);
  let voteMap = $state(new Map<string, number>());
  let loading = $state(true);

  let isLit = $derived(skills.some(s => s.tag_id === id));

  // Top = sorted by vote score descending
  let topArticles = $derived(
    [...articles].sort((a, b) => (voteMap.get(b.at_uri) ?? 0) - (voteMap.get(a.at_uri) ?? 0)).slice(0, 20)
  );
  // Trending = recent articles weighted by recency (just sort by date for now)
  let trendingArticles = $derived(
    [...articles].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 20)
  );

  $effect(() => {
    loading = true;
    Promise.all([getTag(id), getArticlesByTag(id), listSkills()]).then(async ([t, arts, sk]) => {
      tag = t;
      document.title = `${t.name} — NightBoat`;
      articles = arts;
      skills = sk;

      // Fetch votes for all articles
      const votes = await Promise.all(arts.map(a => getArticleVotes(a.at_uri).catch(() => ({ score: 0 }) as VoteSummary)));
      const map = new Map<string, number>();
      arts.forEach((a, i) => map.set(a.at_uri, votes[i].score));
      voteMap = map;

      loading = false;
    });
  });

  async function toggleSkill() {
    if (isLit) {
      await unlightSkill(id);
    } else {
      await lightSkill(id);
    }
    skills = await listSkills();
  }

</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if tag}
  <div class="tag-header">
    <div class="tag-title-row">
      <h1>{tagName(tag.names, tag.name, tag.id)}</h1>
      <button class="skill-btn" class:lit={isLit} onclick={toggleSkill}>
        {isLit ? t('tags.mastered') : t('tags.light')}
      </button>
    </div>
    {#if tag.description}
      <p class="tag-desc">{tag.description}</p>
    {/if}
    <p class="tag-meta">{articles.length} {t('tags.articles')}</p>
  </div>

  {#if articles.length === 0}
    <p class="meta">{t('tags.empty')}</p>
  {:else}
    <div class="columns">
      <div class="column">
        <h2>Top Articles</h2>
        {#each topArticles as a}
          <a href="/article?uri={encodeURIComponent(a.at_uri)}" class="article-item">
            <span class="article-score">{voteMap.get(a.at_uri) ?? 0}</span>
            <div class="article-info">
              <span class="article-title">{a.title}</span>
              {#if a.description}
                <span class="article-desc">{a.description}</span>
              {/if}
              <span class="article-meta">{authorName(a)} &middot; {a.created_at.split(' ')[0]}</span>
            </div>
          </a>
        {/each}
      </div>

      <div class="column">
        <h2>Trending</h2>
        {#each trendingArticles as a}
          <a href="/article?uri={encodeURIComponent(a.at_uri)}" class="article-item">
            <span class="article-score">{voteMap.get(a.at_uri) ?? 0}</span>
            <div class="article-info">
              <span class="article-title">{a.title}</span>
              {#if a.description}
                <span class="article-desc">{a.description}</span>
              {/if}
              <span class="article-meta">{authorName(a)} &middot; {a.created_at.split(' ')[0]}</span>
            </div>
          </a>
        {/each}
      </div>
    </div>
  {/if}
{/if}

<style>
  .tag-header {
    margin-bottom: 1.5rem;
  }
  .tag-title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .tag-title-row h1 {
    margin: 0;
  }
  .skill-btn {
    padding: 4px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: none;
    color: var(--accent);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .skill-btn:hover {
    background: var(--accent);
    color: white;
  }
  .skill-btn.lit {
    background: var(--accent);
    color: white;
  }
  .skill-btn.lit:hover {
    opacity: 0.85;
  }
  .tag-desc {
    margin: 0.5rem 0 0;
    color: var(--text-secondary);
    font-size: 15px;
  }
  .tag-meta {
    margin: 0.25rem 0 0;
    font-size: 13px;
    color: var(--text-hint);
  }

  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
  }
  @media (max-width: 700px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }

  .column h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1rem;
    padding-bottom: 0.25em;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.5rem;
    margin-top: 0;
  }

  .article-score {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-hint);
    min-width: 28px;
    text-align: center;
    flex-shrink: 0;
  }
  .article-info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 8px;
    text-decoration: none;
    color: inherit;
    background: var(--bg-white);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .article-item:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .article-title {
    font-family: var(--font-serif);
    font-size: 15px;
    color: var(--text-primary);
    line-height: 1.35;
  }
  .article-item:hover .article-title {
    color: var(--accent);
  }
  .article-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 3px;
    line-height: 1.45;
  }
  .article-meta {
    font-size: 12px;
    color: var(--text-hint);
    margin-top: 4px;
  }
</style>
