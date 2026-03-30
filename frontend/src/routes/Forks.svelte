<script lang="ts">
  import { getArticle, getArticleForks, castVote } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { Article, ForkWithTitle } from '../lib/types';

  let { uri } = $props<{ uri: string }>();

  let article = $state<Article | null>(null);
  let forks = $state<ForkWithTitle[]>([]);
  let loading = $state(true);
  let isLoggedIn = $derived(!!getAuth());

  $effect(() => {
    if (!uri) return;
    loading = true;
    Promise.all([getArticle(uri), getArticleForks(uri)]).then(([a, f]) => {
      article = a;
      forks = f;
      loading = false;
    });
  });

  async function vote(forkUri: string, value: number) {
    if (!isLoggedIn) return;
    await castVote(forkUri, value);
    forks = await getArticleForks(uri);
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if article}
  <h1>Forks of "{article.title}"</h1>
  <p class="meta">
    <a href="#/article?uri={encodeURIComponent(uri)}">{t('forks.backToOriginal')}</a>
    &middot; {forks.length} forks
  </p>

  {#if forks.length === 0}
    <div class="empty">
      <p>{t('forks.createHint')}</p>
    </div>
  {:else}
    <div class="fork-list">
      {#each forks as f, i}
        <div class="fork-card">
          <div class="fork-rank">#{i + 1}</div>
          <div class="fork-body">
            <a href="#/article?uri={encodeURIComponent(f.forked_uri)}" class="fork-title">{f.title}</a>
            <div class="fork-info">
              <a href="#/profile?did={encodeURIComponent(f.did)}" class="fork-author">
                {f.author_handle ? `@${f.author_handle}` : f.did.slice(0, 24) + '…'}
              </a>
            </div>
          </div>
          <div class="fork-votes">
            <button class="vote-btn" title={t('common.upvote')} onclick={() => vote(f.forked_uri, 1)} disabled={!isLoggedIn}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="18 15 12 9 6 15"/></svg>
            </button>
            <span class="score">{f.vote_score}</span>
            <button class="vote-btn" title={t('common.downvote')} onclick={() => vote(f.forked_uri, -1)} disabled={!isLoggedIn}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"/></svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  h1 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.5rem;
  }
  .fork-list {
    margin-top: 1rem;
  }
  .fork-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .fork-rank {
    font-size: 13px;
    color: var(--text-hint);
    width: 28px;
    text-align: center;
    flex-shrink: 0;
  }
  .fork-body {
    flex: 1;
    min-width: 0;
  }
  .fork-title {
    font-family: var(--font-serif);
    font-size: 15px;
    color: var(--text-primary);
    text-decoration: none;
    display: block;
  }
  .fork-title:hover { color: var(--accent); }
  .fork-info {
    font-size: 12px;
    color: var(--text-hint);
    margin-top: 2px;
  }
  .fork-author {
    color: var(--text-hint);
    text-decoration: none;
  }
  .fork-author:hover { color: var(--accent); }
  .fork-votes {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .vote-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px;
    color: var(--text-hint);
    display: flex;
    transition: color 0.15s;
  }
  .vote-btn:hover:not(:disabled) { color: var(--accent); }
  .vote-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .score {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 20px;
    text-align: center;
  }
  .empty {
    text-align: center;
    padding: 3rem 0;
    color: var(--text-secondary);
  }
</style>
