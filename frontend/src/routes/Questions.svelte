<script lang="ts">
  import { listQuestions, getAllArticleTeaches } from '../lib/api';
  import { tagName, authorName } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { buildArticleRowMap } from '../lib/series';
  import { getAuth } from '../lib/auth.svelte';
  import type { Article, ContentTeachRow } from '../lib/types';

  let locale = $derived(getLocale());

  let questions = $state<Article[]>([]);
  let articleTeaches = $state(new Map<string, ContentTeachRow[]>());
  let loading = $state(true);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      const [qs, tags] = await Promise.all([
        listQuestions(),
        getAllArticleTeaches(),
      ]);
      questions = qs;
      articleTeaches = buildArticleRowMap(tags);
    } catch { /* */ }
    loading = false;
  }

  function navToTag(e: MouseEvent | KeyboardEvent, tagId: string) {
    if (e instanceof KeyboardEvent && e.key !== 'Enter') return;
    e.preventDefault();
    e.stopPropagation();
    window.location.href = `/tag?id=${encodeURIComponent(tagId)}`;
  }
</script>

<div class="questions-header">
  <h1>{t('qa.questions')}</h1>
  {#if getAuth()}
    <a href="/new-question" class="btn-ask">{t('qa.askQuestion')}</a>
  {/if}
</div>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if questions.length === 0}
  <p class="empty">{t('qa.noQuestions')}</p>
{:else}
  {#each questions as q}
    <a href="/question?uri={encodeURIComponent(q.at_uri)}" class="question-card">
      <div class="q-top">
        <span class="q-badge">{t('qa.questionBadge')}</span>
        <span class="q-title">{q.title}</span>
      </div>

      {#if articleTeaches.get(q.at_uri)?.length}
        <div class="q-tags">
          {#each articleTeaches.get(q.at_uri) || [] as tag}
            <span class="tag" role="link" tabindex="0" onclick={(e) => navToTag(e, tag.tag_id)} onkeydown={(e) => navToTag(e, tag.tag_id)}>{tagName(tag.tag_names, tag.tag_name, tag.tag_id)}</span>
          {/each}
        </div>
      {/if}

      {#if q.description}
        <p class="q-desc">{q.description}</p>
      {/if}

      <div class="q-bottom">
        <span class="q-meta">{authorName(q)} &middot; {q.created_at.split(' ')[0]}</span>
        <span class="q-stats">
          <span class="stat">{t('qa.answerCount', q.answer_count)}</span>
          {#if q.vote_score !== 0}
            <span class="stat">&#9650; {q.vote_score}</span>
          {/if}
        </span>
      </div>
    </a>
  {/each}
{/if}

<style>
  .questions-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }
  .questions-header h1 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.5rem;
    margin: 0;
  }
  .btn-ask {
    font-size: 13px;
    padding: 5px 14px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--accent);
    text-decoration: none;
    transition: all 0.15s;
  }
  .btn-ask:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }

  .question-card {
    display: block;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-left: 3px solid #d97706;
    border-radius: 4px;
    padding: 14px 18px;
    margin-bottom: 10px;
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .question-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .q-top {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .q-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .q-title {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    color: var(--text-primary);
    line-height: 1.35;
    flex: 1;
    min-width: 0;
  }
  .question-card:hover .q-title {
    color: var(--accent);
  }
  .q-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
  .q-desc {
    margin: 8px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .q-bottom {
    margin-top: 8px;
    display: flex;
    align-items: center;
  }
  .q-meta {
    font-size: 13px;
    color: var(--text-hint);
  }
  .q-stats {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }
  .stat {
    font-size: 12px;
    color: var(--text-hint);
  }
  .empty { color: var(--text-hint); font-size: 14px; }
</style>
