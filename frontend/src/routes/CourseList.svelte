<script lang="ts">
  import { listCourses } from '../lib/api';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import { getAuth } from '../lib/auth';
  import type { Course } from '../lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let courses = $state<Course[]>([]);
  let loading = $state(true);

  $effect(() => {
    loadCourses();
  });

  async function loadCourses() {
    loading = true;
    try {
      courses = await listCourses(100, 0);
    } catch { /* */ }
    loading = false;
  }

  const scheduleLabel = (s: string) => {
    if (s === 'weekly') return t('courses.weekly');
    if (s === 'module') return t('courses.module');
    return t('courses.custom');
  };
</script>

<h1>{t('courses.title')}</h1>
<p class="subtitle">{t('courses.subtitle')}</p>

{#if getAuth()}
  <a href="#/new-course" class="create-btn">{t('courses.create')}</a>
{/if}

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if courses.length === 0}
  <p class="empty">{t('courses.empty')}</p>
{:else}
  <div class="course-grid">
    {#each courses as course}
      <a href="#/course?id={encodeURIComponent(course.id)}" class="course-card">
        {#if course.cover_url}
          <img src={course.cover_url} alt={course.title} class="course-cover" />
        {/if}
        <div class="course-info">
          <h3 class="course-title">{course.title}</h3>
          <span class="course-schedule">{scheduleLabel(course.schedule_type)}</span>
          {#if course.description}
            <p class="course-desc">{course.description.slice(0, 150)}{course.description.length > 150 ? '...' : ''}</p>
          {/if}
        </div>
      </a>
    {/each}
  </div>
{/if}

<style>
  h1 { margin-bottom: 0; }
  .subtitle {
    color: var(--text-secondary);
    font-size: 14px;
    margin: 4px 0 16px;
  }
  .create-btn {
    display: inline-block;
    padding: 6px 16px;
    font-size: 13px;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    text-decoration: none;
    margin-bottom: 16px;
    transition: all 0.15s;
  }
  .create-btn:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }
  .empty { color: var(--text-hint); font-size: 14px; }
  .course-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 12px;
  }
  .course-card {
    display: flex;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-white);
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .course-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 2px 8px rgba(0,0,0,0.05);
    text-decoration: none;
  }
  .course-cover {
    width: 80px;
    height: 80px;
    object-fit: cover;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .course-info { flex: 1; min-width: 0; }
  .course-title {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .course-schedule {
    display: inline-block;
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-light);
    padding: 1px 6px;
    border-radius: 3px;
    margin-top: 3px;
  }
  .course-desc {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.4;
  }
</style>
