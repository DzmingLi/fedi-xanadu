<script lang="ts">
  // Course-level reviews — backed by a single endpoint that returns
  // every review across every iteration. Each row carries the optional
  // iteration tag (`term_id` + `term_semester`) so we can render an
  // inline chip when the contributor declared one.
  import { listCourseReviews, getCourseDetail } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { TermReview } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let items = $state<TermReview[]>([]);
  let total = $state(0);
  let courseTitle = $state('');
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const [detail, page] = await Promise.all([
        getCourseDetail(id),
        listCourseReviews(id, 50, 0),
      ]);
      courseTitle = detail.course.title;
      document.title = `${t('course.reviews')} — ${courseTitle}`;
      items = page.items;
      total = page.total;
    } catch (e: any) {
      error = e.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { if (id) load(); });
</script>

<div class="page">
  <a class="back" href="/course?id={encodeURIComponent(id)}">← {courseTitle}</a>
  <header>
    <h1>{t('course.reviews')} <span class="count">({total})</span></h1>
    {#if getAuth()}
      <a class="write-btn" href="/new?category=review&course_id={encodeURIComponent(id)}">{t('course.writeReview')}</a>
    {/if}
  </header>

  {#if error}<p class="err">{error}</p>{/if}

  {#if loading && items.length === 0}
    <p class="meta">{t('common.loading')}</p>
  {:else if items.length === 0}
    <p class="meta">{t('course.noReviews')}</p>
  {:else}
    {#each items as r}
      <a href={r.at_uri ? `/article?uri=${encodeURIComponent(r.at_uri)}` : '#'} class="card">
        <div class="hdr">
          <span class="author">{r.author_display_name || r.author_handle || r.did.slice(0, 16)}</span>
          <span class="date">{new Date(r.created_at).toLocaleDateString()}</span>
          {#if r.term_id && r.term_semester}
            <a class="iter-tag"
               href="/course?id={encodeURIComponent(id)}&term={encodeURIComponent(r.term_id)}"
               onclick={(e) => e.stopPropagation()}>
              {t('course.tookIn').replace('{semester}', r.term_semester)}
            </a>
          {/if}
        </div>
        <h3>{r.title}</h3>
        {#if r.summary}<p class="desc">{r.summary}</p>{/if}
        <div class="stats">
          <span>{r.vote_score} votes</span>
          <span>{r.comment_count} comments</span>
        </div>
      </a>
    {/each}
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
  .iter-tag { padding: 1px 8px; background: rgba(95,155,101,0.10); color: var(--accent); border-radius: 3px; font-size: 11px; text-decoration: none; }
  .iter-tag:hover { background: rgba(95,155,101,0.18); text-decoration: none; }
  .card h3 { font-family: var(--font-serif); font-size: 17px; margin: 6px 0; color: var(--text-primary); }
  .desc { font-size: 14px; color: var(--text-secondary); margin: 0 0 6px; line-height: 1.5; }
  .stats { display: flex; gap: 14px; font-size: 12px; color: var(--text-hint); }
</style>
