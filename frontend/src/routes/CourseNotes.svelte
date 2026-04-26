<script lang="ts">
  // Course-level notes — same fan-out approach as CourseReviews. Notes
  // currently live per-iteration on the backend; this view collects
  // them all under the umbrella course and sorts by created_at desc.
  import { listTermNotes, getCourseDetail } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { TermReview, Term } from '../lib/types';

  let { id } = $props<{ id: string }>();

  type Row = TermReview & { term_id: string; term_label: string };

  let items = $state<Row[]>([]);
  let courseTitle = $state('');
  let terms = $state<Term[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const detail = await getCourseDetail(id);
      courseTitle = detail.course.title;
      terms = detail.terms;
      document.title = `${t('course.notes')} — ${courseTitle}`;
      const perTerm = await Promise.all(
        terms.map(t =>
          listTermNotes(t.id, 50, 0)
            .then(r => r.items.map(it => ({ ...it, term_id: t.id, term_label: t.semester || t.title })))
            .catch(() => [] as Row[]),
        ),
      );
      items = perTerm.flat().sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      );
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
    <h1>{t('course.notes')} <span class="count">({items.length})</span></h1>
    {#if getAuth() && terms.length > 0}
      <a class="write-btn" href="/new?category=note&term_id={encodeURIComponent(terms[0].id)}">{t('course.writeNote')}</a>
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
          <span class="iter-tag">{n.term_label}</span>
        </div>
        <h3>{n.title}</h3>
        {#if n.summary}<p class="desc">{n.summary}</p>{/if}
        <div class="stats">
          <span>{n.vote_score} votes</span>
          <span>{n.comment_count} comments</span>
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
  .iter-tag { padding: 1px 8px; background: rgba(95,155,101,0.10); color: var(--accent); border-radius: 3px; font-size: 11px; }
  .card h3 { font-family: var(--font-serif); font-size: 17px; margin: 6px 0; color: var(--text-primary); }
  .desc { font-size: 14px; color: var(--text-secondary); margin: 0 0 6px; line-height: 1.5; }
  .stats { display: flex; gap: 14px; font-size: 12px; color: var(--text-hint); }
</style>
