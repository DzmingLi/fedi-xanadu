<script lang="ts">
  import { listCourseNotes, getCourseDetail } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { CourseReview, CourseSession } from '../lib/types';

  let { id } = $props<{ id: string }>();
  let items = $state<CourseReview[]>([]);
  let total = $state(0);
  let sessions = $state<CourseSession[]>([]);
  let courseTitle = $state('');
  let page = $state(0);
  const pageSize = 20;
  let loading = $state(true);
  let error = $state('');

  let totalPages = $derived(Math.max(1, Math.ceil(total / pageSize)));

  async function load() {
    loading = true;
    error = '';
    try {
      const [resp, detail] = await Promise.all([
        listCourseNotes(id, pageSize, page * pageSize),
        sessions.length === 0 ? getCourseDetail(id) : Promise.resolve(null),
      ]);
      items = resp.items;
      total = resp.total;
      if (detail) {
        sessions = detail.sessions;
        courseTitle = detail.course.title;
        document.title = `${t('course.learnerNotes')} — ${courseTitle}`;
      }
    } catch (e: any) {
      error = e.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { if (id) load(); });

  function gotoPage(p: number) {
    page = Math.max(0, Math.min(p, totalPages - 1));
    load();
    window.scrollTo({ top: 0 });
  }
</script>

<div class="page">
  <a class="back" href="/course?id={encodeURIComponent(id)}">← {courseTitle}</a>
  <header>
    <h1>{t('course.learnerNotes')} <span class="count">({total})</span></h1>
    {#if getAuth()}
      <a class="write-btn" href="/new?category=note&course_id={encodeURIComponent(id)}">{t('course.writeNote')}</a>
    {/if}
  </header>

  {#if error}<p class="err">{error}</p>{/if}

  {#if loading && items.length === 0}
    <p class="meta">{t('common.loading')}</p>
  {:else if items.length === 0}
    <p class="meta">{t('course.noNotes')}</p>
  {:else}
    {#each items as n}
      <a href="/article?uri={encodeURIComponent(n.at_uri)}" class="card">
        <div class="hdr">
          <span class="author">{n.author_display_name || n.author_handle || n.did.slice(0, 16)}</span>
          <span class="date">{new Date(n.created_at).toLocaleDateString()}</span>
          {#if n.course_session_id}
            {@const lec = sessions.find(s => s.id === n.course_session_id)}
            {#if lec}<span class="session">{t('course.onLecture')} {lec.sort_order}: {lec.topic}</span>{/if}
          {/if}
        </div>
        <h3>{n.title}</h3>
        {#if n.summary}<p class="desc">{n.summary}</p>{/if}
        <div class="stats">
          <span>{n.vote_score} votes</span>
          <span>{n.comment_count} comments</span>
        </div>
      </a>
    {/each}

    {#if totalPages > 1}
      <div class="pager">
        <button onclick={() => gotoPage(page - 1)} disabled={page === 0}>← {t('common.prev')}</button>
        <span>{page + 1} / {totalPages}</span>
        <button onclick={() => gotoPage(page + 1)} disabled={page >= totalPages - 1}>{t('common.next')} →</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page { max-width: 840px; margin: 0 auto; padding: 24px 16px; }
  .back { display: inline-block; margin-bottom: 12px; font-size: 13px; color: var(--text-secondary); text-decoration: none; }
  .back:hover { color: var(--accent); }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.6rem; margin: 0; }
  .count { color: var(--text-hint); font-size: 0.85em; }
  .write-btn { padding: 6px 14px; background: var(--accent); color: white; border-radius: 4px; font-size: 13px; text-decoration: none; }
  .write-btn:hover { opacity: 0.9; text-decoration: none; }
  .meta { color: var(--text-hint); }
  .err { background: #fee; color: #c00; padding: 8px 12px; border-radius: 4px; font-size: 13px; }
  .card { display: block; padding: 16px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 12px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .card:hover { border-color: var(--accent); text-decoration: none; }
  .hdr { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; font-size: 12px; color: var(--text-hint); }
  .author { color: var(--text-primary); font-weight: 500; }
  .session { padding: 1px 6px; background: var(--bg-hover, #f5f5f5); border-radius: 3px; font-size: 11px; }
  .card h3 { font-family: var(--font-serif); font-size: 17px; margin: 6px 0; color: var(--text-primary); }
  .desc { font-size: 14px; color: var(--text-secondary); margin: 0 0 6px; line-height: 1.5; }
  .stats { display: flex; gap: 14px; font-size: 12px; color: var(--text-hint); }

  .pager { display: flex; justify-content: center; align-items: center; gap: 12px; margin-top: 24px; font-size: 13px; }
  .pager button { padding: 4px 12px; border: 1px solid var(--border); background: var(--bg-white); border-radius: 4px; cursor: pointer; }
  .pager button:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .pager button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
