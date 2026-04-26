<script lang="ts">
  import { getCourseDetail, deleteCourse } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import CommentThread from '../components/CommentThread.svelte';
  import type { CourseDetail } from '../lib/types';

  let { id }: { id: string } = $props();

  let detail = $state<CourseDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  $effect(() => { load(); });
  async function load() {
    if (!id) return;
    loading = true;
    try {
      detail = await getCourseDetail(id);
      document.title = `${detail.course.title} — NightBoat`;
    } catch (e: any) {
      error = e?.message || 'Error';
    } finally {
      loading = false;
    }
  }

  async function onDelete() {
    if (!detail) return;
    if (!confirm(t('courses.deleteConfirm'))) return;
    try {
      await deleteCourse(detail.course.id);
      navigate('/courses');
    } catch (e: any) {
      alert(e?.message || 'Delete failed');
    }
  }

  const isOwner = $derived(!!getAuth() && !!detail);
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <article class="course-page">
    <header class="course-header">
      {#if detail.course.code}
        <div class="course-code">{detail.course.code}</div>
      {/if}
      <h1 class="course-title">{detail.course.title}</h1>
      {#if detail.course.institution}
        <p class="course-institution">{detail.course.institution}</p>
      {/if}
      {#if detail.course.description}
        <p class="course-desc">{detail.course.description}</p>
      {/if}
      {#if isOwner}
        <div class="course-actions">
          <button class="danger" onclick={onDelete}>{t('common.delete')}</button>
        </div>
      {/if}
    </header>

    <section class="iterations">
      <h2>{t('courses.iterations')}  <span class="count">{detail.terms.length}</span></h2>
      {#if detail.terms.length === 0}
        <p class="empty">{t('courses.noTerms')}</p>
      {:else}
        <ul class="term-list">
          {#each detail.terms as term}
            <li class="term-row">
              <a href="/term?id={encodeURIComponent(term.id)}" class="term-link">
                <span class="term-semester">{term.semester || '—'}</span>
                <span class="term-title">{term.title}</span>
                {#if term.author_names && term.author_names.length > 0}
                  <span class="term-instructor">{term.author_names.join(', ')}</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="discussion">
      <h2>{t('courses.discussion')}  <span class="count">{detail.discussion_count}</span></h2>
      <CommentThread contentUri={`course:${detail.course.id}`} />
    </section>
  </article>
{/if}

<style>
  .course-page { max-width: 760px; margin: 0 auto; padding: 24px 16px; }
  .course-header { margin-bottom: 32px; padding-bottom: 16px; border-bottom: 1px solid var(--border); }
  .course-code {
    display: inline-block; font-size: 12px; padding: 2px 10px;
    background: rgba(95,155,101,0.10); color: var(--accent); border-radius: 3px;
    font-family: var(--font-mono, monospace); margin-bottom: 8px;
  }
  .course-title { font-family: var(--font-serif); font-weight: 500; margin: 4px 0 8px; }
  .course-institution { color: var(--text-secondary); margin: 0 0 12px; }
  .course-desc { color: var(--text-secondary); line-height: 1.6; margin: 0; }
  .course-actions { margin-top: 16px; }
  .danger {
    background: transparent; border: 1px solid #c00; color: #c00;
    padding: 4px 12px; border-radius: 3px; cursor: pointer; font-size: 13px;
  }
  .danger:hover { background: #c00; color: white; }

  section { margin-bottom: 32px; }
  section h2 {
    font-family: var(--font-serif); font-weight: 400; font-size: 1.2rem;
    margin: 0 0 12px; color: var(--text-primary);
  }
  .count { color: var(--text-secondary); font-size: 0.9rem; font-family: inherit; margin-left: 4px; }

  .term-list { list-style: none; padding: 0; margin: 0; border: 1px solid var(--border); border-radius: 4px; }
  .term-row { border-bottom: 1px solid var(--border); }
  .term-row:last-child { border-bottom: none; }
  .term-link {
    display: grid; grid-template-columns: 140px 1fr auto; align-items: center; gap: 16px;
    padding: 10px 16px; text-decoration: none; color: inherit; transition: background 0.15s;
  }
  .term-link:hover { background: rgba(95,155,101,0.05); text-decoration: none; }
  .term-semester { font-weight: 500; color: var(--accent); font-size: 13px; }
  .term-title { font-size: 14px; }
  .term-instructor { font-size: 12px; color: var(--text-secondary); }

  .empty { color: var(--text-secondary); font-style: italic; }
  .meta { color: var(--text-secondary); padding: 24px; }
  .error { color: red; padding: 24px; }
</style>
