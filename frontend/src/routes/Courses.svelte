<script lang="ts">
  import { listCourses } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { CourseListItem } from '../lib/types';

  let courses = $state<CourseListItem[]>([]);
  let loading = $state(true);

  $effect(() => {
    listCourses().then(c => { courses = c; }).catch(() => {}).finally(() => { loading = false; });
  });
</script>

<div class="courses-page">
  <div class="page-header">
    <h1>{t('courses.title')}</h1>
    {#if getAuth()}
      <a href="/new-course" class="create-btn">+ {t('courses.create')}</a>
    {/if}
  </div>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if courses.length === 0}
    <p class="empty">{t('courses.empty')}</p>
  {:else}
    <div class="course-grid">
      {#each courses as course}
        <a href="/course?id={encodeURIComponent(course.id)}" class="course-card">
          <div class="card-top">
            {#if course.code}
              <span class="course-code">{course.code}</span>
            {/if}
            <h2 class="course-title">{course.title}</h2>
          </div>
          {#if course.institution}
            <p class="course-meta">{course.institution}</p>
          {/if}
          {#if course.description}
            <p class="course-desc">{course.description}</p>
          {/if}
          <div class="card-bottom">
            <span class="stat">
              {course.iteration_count}
              {course.iteration_count === 1 ? t('courses.iteration') : t('courses.iterations')}
            </span>
            {#if course.latest_semester}
              <span class="latest">{t('courses.latest')}: {course.latest_semester}</span>
            {/if}
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .courses-page { max-width: 960px; margin: 0 auto; }
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
  .page-header h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.8rem; margin: 0; }
  .create-btn {
    font-size: 13px; padding: 6px 16px; border: 1px solid var(--accent);
    border-radius: 4px; color: var(--accent); text-decoration: none; transition: all 0.15s;
  }
  .create-btn:hover { background: var(--accent); color: white; text-decoration: none; }
  .course-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 16px; }
  .course-card {
    display: block; background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px;
    padding: 16px; text-decoration: none; color: inherit; transition: border-color 0.15s, box-shadow 0.15s;
  }
  .course-card:hover { border-color: var(--accent); box-shadow: 0 1px 4px rgba(95,155,101,0.1); text-decoration: none; }
  .card-top { display: flex; align-items: baseline; gap: 8px; margin-bottom: 8px; flex-wrap: wrap; }
  .course-code {
    font-size: 11px; padding: 2px 8px; background: rgba(95,155,101,0.10);
    color: var(--accent); border-radius: 3px; font-family: var(--font-mono, monospace);
  }
  .course-title { font-family: var(--font-serif); font-weight: 500; font-size: 1.1rem; margin: 0; line-height: 1.3; }
  .course-meta { font-size: 12px; color: var(--text-secondary); margin: 0 0 8px; }
  .course-desc {
    font-size: 13px; color: var(--text-secondary); margin: 0 0 12px;
    display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
  }
  .card-bottom {
    display: flex; gap: 12px; font-size: 12px; color: var(--text-secondary);
    border-top: 1px solid var(--border); padding-top: 8px; margin-top: auto; flex-wrap: wrap;
  }
  .stat { font-weight: 500; }
  .latest { color: var(--text-secondary); }
  .meta { color: var(--text-secondary); }
  .empty { color: var(--text-secondary); font-style: italic; }
</style>
