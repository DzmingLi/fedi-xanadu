<script lang="ts">
  import { getCourseDetail } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { marked } from 'marked';
  import type { CourseDetail } from '../lib/types';

  let { id } = $props<{ id: string }>();
  let detail = $state<CourseDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  let isOwner = $derived(detail && getAuth()?.did === detail.course.did);

  $effect(() => {
    if (!id) return;
    loading = true;
    error = '';
    getCourseDetail(id)
      .then(d => {
        detail = d;
        document.title = `${d.course.title} — NightBoat`;
      })
      .catch(e => { error = e.message; })
      .finally(() => { loading = false; });
  });

  let syllabusHtml = $derived(detail?.syllabus ? marked.parse(detail.syllabus) as string : '');
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if error}
  <p class="error">Error: {error}</p>
{:else if detail}
  {@const c = detail.course}
  <div class="course-page">
    <div class="course-header">
      <div class="header-main">
        {#if c.code}
          <span class="course-code">{c.code}</span>
        {/if}
        <h1 class="course-title">{c.title}</h1>
        {#if c.institution || c.department}
          <p class="course-institution">
            {c.institution || ''}{#if c.institution && c.department} — {/if}{c.department || ''}
          </p>
        {/if}
        {#if c.semester}
          <p class="course-semester">{c.semester}</p>
        {/if}
        {#if c.description}
          <p class="course-desc">{c.description}</p>
        {/if}
        {#if c.source_url}
          <a href={c.source_url} target="_blank" rel="noopener" class="source-link">
            Source: {c.source_url}
          </a>
        {/if}
        {#if c.source_attribution}
          <p class="attribution">{c.source_attribution}</p>
        {/if}
      </div>
      <div class="header-side">
        {#if detail.staff.length > 0}
          <div class="staff-list">
            <h3>Staff</h3>
            {#each detail.staff as s}
              <a href="/profile?did={encodeURIComponent(s.user_did)}" class="staff-item">
                {#if s.avatar_url}
                  <img src={s.avatar_url} alt="" class="staff-avatar" />
                {:else}
                  <div class="staff-avatar placeholder">{(s.display_name || s.handle || '?').charAt(0).toUpperCase()}</div>
                {/if}
                <div class="staff-info">
                  <span class="staff-name">{s.display_name || s.handle || s.user_did.slice(0, 16)}</span>
                  <span class="staff-role">{s.role}</span>
                </div>
              </a>
            {/each}
          </div>
        {/if}
        {#if isOwner}
          <a href="/new-course?edit={encodeURIComponent(c.id)}" class="edit-btn">{t('common.edit')}</a>
        {/if}
        <div class="course-meta-box">
          <span class="meta-item">License: {c.license}</span>
          <span class="meta-item">Language: {c.lang.toUpperCase()}</span>
        </div>
      </div>
    </div>

    <div class="course-body">
      <div class="body-main">
        {#if syllabusHtml}
          <section class="syllabus">
            <h2>Syllabus</h2>
            <div class="content">{@html syllabusHtml}</div>
          </section>
        {/if}

        {#if detail.schedule.length > 0}
          <section class="schedule">
            <h2>Calendar</h2>
            <table class="schedule-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>Topic</th>
                  <th>Notes</th>
                </tr>
              </thead>
              <tbody>
                {#each detail.schedule as s}
                  <tr>
                    <td class="session-num">{s.session}</td>
                    <td class="session-topic">{s.topic}</td>
                    <td class="session-notes">
                      {#if s.notes}<span class="note-badge">{s.notes}</span>{/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </section>
        {/if}

        {#if detail.series.length > 0}
          <section class="course-series">
            <h2>Course Materials</h2>
            {#each detail.series as s}
              <a href="/series?id={encodeURIComponent(s.series_id)}" class="series-link">
                <span class="series-role">{s.role}</span>
                <span class="series-title">{s.title}</span>
                {#if s.description}<span class="series-desc">{s.description}</span>{/if}
              </a>
            {/each}
          </section>
        {/if}
      </div>

      <div class="body-side">
        {#if detail.textbooks.length > 0}
          <section class="textbooks">
            <h3>Textbooks</h3>
            {#each detail.textbooks as tb}
              <a href="/book?id={encodeURIComponent(tb.book_id)}" class="textbook-card">
                {#if tb.cover_url}
                  <img src={tb.cover_url} alt="" class="textbook-cover" />
                {/if}
                <div class="textbook-info">
                  <span class="textbook-title">{tb.title}</span>
                  <span class="textbook-authors">{tb.authors.join(', ')}</span>
                  <span class="textbook-role">{tb.role}</span>
                </div>
              </a>
            {/each}
          </section>
        {/if}

        {#if detail.prerequisites.length > 0}
          <section class="prereqs">
            <h3>Prerequisites</h3>
            {#each detail.prerequisites as p}
              <a href="/course?id={encodeURIComponent(p.prereq_course_id)}" class="prereq-link">
                {#if p.code}<span class="prereq-code">{p.code}</span>{/if}
                {p.title}
                {#if p.institution}<span class="prereq-inst">{p.institution}</span>{/if}
              </a>
            {/each}
          </section>
        {/if}

        {#if detail.skill_trees.length > 0}
          <section class="skill-trees">
            <h3>Skill Trees</h3>
            {#each detail.skill_trees as st}
              <a href="/skill-tree?uri={encodeURIComponent(st.tree_uri)}" class="tree-link">
                <span class="tree-role">{st.role}</span>
                {st.title}
              </a>
            {/each}
          </section>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .course-page { max-width: 100%; }
  .course-header { display: flex; gap: 32px; margin-bottom: 32px; padding-bottom: 24px; border-bottom: 1px solid var(--border); }
  .header-main { flex: 1; min-width: 0; }
  .header-side { width: 260px; flex-shrink: 0; }
  .course-code { font-size: 13px; font-weight: 600; color: var(--accent); background: rgba(95,155,101,0.1); padding: 3px 10px; border-radius: 3px; display: inline-block; margin-bottom: 8px; }
  .course-title { font-family: var(--font-serif); font-size: 2rem; font-weight: 400; margin: 0 0 8px; line-height: 1.3; }
  .course-institution { font-size: 14px; color: var(--text-secondary); margin: 4px 0; }
  .course-semester { font-size: 13px; color: var(--text-hint); margin: 2px 0; }
  .course-desc { font-size: 14px; color: var(--text-secondary); line-height: 1.6; margin: 12px 0; }
  .source-link { font-size: 12px; color: var(--text-hint); text-decoration: none; word-break: break-all; }
  .source-link:hover { color: var(--accent); }
  .attribution { font-size: 12px; color: var(--text-hint); font-style: italic; margin: 4px 0; }

  .staff-list { margin-bottom: 16px; }
  .staff-list h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .staff-item { display: flex; align-items: center; gap: 8px; padding: 6px 0; text-decoration: none; color: inherit; }
  .staff-item:hover { opacity: 0.8; text-decoration: none; }
  .staff-avatar { width: 32px; height: 32px; border-radius: 50%; object-fit: cover; flex-shrink: 0; }
  .staff-avatar.placeholder { display: flex; align-items: center; justify-content: center; background: var(--accent); color: white; font-size: 14px; }
  .staff-info { display: flex; flex-direction: column; }
  .staff-name { font-size: 13px; color: var(--text-primary); }
  .staff-role { font-size: 11px; color: var(--text-hint); text-transform: capitalize; }
  .edit-btn { font-size: 13px; padding: 5px 14px; border: 1px solid var(--border); border-radius: 4px; color: var(--text-secondary); text-decoration: none; display: inline-block; margin-bottom: 12px; }
  .edit-btn:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }
  .course-meta-box { font-size: 12px; color: var(--text-hint); display: flex; flex-direction: column; gap: 4px; }

  .course-body { display: flex; gap: 32px; }
  .body-main { flex: 1; min-width: 0; }
  .body-side { width: 260px; flex-shrink: 0; }
  .syllabus h2, .course-series h2, .schedule h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.3rem; margin: 0 0 16px; }
  .syllabus { margin-bottom: 32px; }

  .schedule { margin-bottom: 32px; }
  .schedule-table { width: 100%; border-collapse: collapse; font-size: 14px; }
  .schedule-table th { text-align: left; font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.03em; color: var(--text-hint); padding: 8px 12px; border-bottom: 2px solid var(--border); }
  .schedule-table td { padding: 8px 12px; border-bottom: 1px solid var(--border); vertical-align: top; }
  .schedule-table tbody tr:hover { background: var(--bg-hover, rgba(0,0,0,0.02)); }
  .session-num { font-weight: 600; color: var(--text-hint); width: 40px; }
  .session-topic { color: var(--text-primary); }
  .session-notes { width: 160px; }
  .note-badge { font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 3px; background: rgba(217,119,6,0.1); color: #d97706; }
  .series-link { display: block; padding: 12px 16px; border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: 0 4px 4px 0; margin-bottom: 8px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .series-link:hover { border-color: var(--accent); text-decoration: none; }
  .series-role { font-size: 11px; font-weight: 600; color: var(--accent); text-transform: uppercase; display: block; margin-bottom: 2px; }
  .series-title { font-family: var(--font-serif); font-size: 1.05rem; display: block; }
  .series-desc { font-size: 13px; color: var(--text-secondary); display: block; margin-top: 4px; }

  .textbooks { margin-bottom: 20px; }
  .textbooks h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .textbook-card { display: flex; gap: 12px; padding: 10px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 8px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .textbook-card:hover { border-color: var(--accent); text-decoration: none; }
  .textbook-cover { width: 48px; height: 64px; object-fit: cover; border-radius: 3px; flex-shrink: 0; background: var(--bg-page); }
  .textbook-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .textbook-title { font-size: 13px; font-weight: 500; color: var(--text-primary); line-height: 1.3; }
  .textbook-authors { font-size: 12px; color: var(--text-secondary); }
  .textbook-role { font-size: 11px; color: var(--text-hint); text-transform: capitalize; }
  .prereqs, .skill-trees { margin-bottom: 20px; }
  .prereqs h3, .skill-trees h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .prereq-link, .tree-link { display: block; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 6px; text-decoration: none; color: var(--text-primary); font-size: 13px; transition: border-color 0.15s; }
  .prereq-link:hover, .tree-link:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }
  .prereq-code { font-weight: 600; color: var(--accent); margin-right: 6px; }
  .prereq-inst { font-size: 12px; color: var(--text-hint); margin-left: 6px; }
  .tree-role { font-size: 11px; color: var(--text-hint); text-transform: capitalize; display: block; margin-bottom: 2px; }

  @media (max-width: 768px) {
    .course-header { flex-direction: column; }
    .header-side { width: 100%; }
    .course-body { flex-direction: column; }
    .body-side { width: 100%; }
  }
</style>
