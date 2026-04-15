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
    <h1>Courses</h1>
    {#if getAuth()}
      <a href="/new-course" class="create-btn">+ New Course</a>
    {/if}
  </div>

  {#if loading}
    <p class="meta">Loading...</p>
  {:else if courses.length === 0}
    <p class="empty">No courses yet.</p>
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
          {#if course.institution || course.semester}
            <p class="course-meta">
              {#if course.institution}{course.institution}{/if}
              {#if course.institution && course.semester} &middot; {/if}
              {#if course.semester}{course.semester}{/if}
            </p>
          {/if}
          {#if course.description}
            <p class="course-desc">{course.description}</p>
          {/if}
          <div class="card-bottom">
            <span class="stat">{course.series_count} series</span>
            <span class="stat">{course.staff_count} staff</span>
            {#if course.author_handle}
              <span class="author">@{course.author_handle}</span>
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
    padding: 20px; text-decoration: none; color: inherit; transition: border-color 0.15s, box-shadow 0.15s;
  }
  .course-card:hover { border-color: var(--accent); box-shadow: 0 2px 8px rgba(0,0,0,0.06); text-decoration: none; }
  .card-top { margin-bottom: 8px; }
  .course-code { font-size: 12px; font-weight: 600; color: var(--accent); background: rgba(95,155,101,0.1); padding: 2px 8px; border-radius: 3px; margin-bottom: 6px; display: inline-block; }
  .course-title { font-family: var(--font-serif); font-size: 1.15rem; margin: 4px 0 0; line-height: 1.35; }
  .course-meta { font-size: 13px; color: var(--text-secondary); margin: 4px 0; }
  .course-desc { font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin: 8px 0; display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .card-bottom { display: flex; gap: 12px; align-items: center; margin-top: 12px; font-size: 12px; color: var(--text-hint); }
  .author { margin-left: auto; }
  .empty { color: var(--text-hint); }
</style>
