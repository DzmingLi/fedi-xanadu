<script lang="ts">
  import { getCourseDetail, rateCourse, unrateCourse, setCourseLearningStatus, removeCourseLearningStatus, setSessionProgress } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';

  /** Resolve a localized field (Record<string, string>) to the current locale with fallback. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const locale = getLocale();
    return field[locale] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }
  import { marked } from 'marked';
  import type { CourseDetail } from '../lib/types';

  let { id } = $props<{ id: string }>();
  let detail = $state<CourseDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  let isOwner = $derived(detail && getAuth()?.did === detail.course.did);

  // Rating state
  let avgRating = $state(0);
  let ratingCount = $state(0);
  let myRating = $state(0);
  let hoverRating = $state(0);

  let learningStatus = $state('');
  let learningProgress = $state(0);
  let sessionDone = $state(new Map<string, boolean>());

  $effect(() => {
    if (!id) return;
    loading = true;
    error = '';
    getCourseDetail(id)
      .then(d => {
        detail = d;
        avgRating = d.rating.avg_rating;
        ratingCount = d.rating.rating_count;
        myRating = d.my_rating ?? 0;
        learningStatus = d.my_learning_status?.status ?? '';
        learningProgress = d.my_learning_status?.progress ?? 0;
        sessionDone = new Map(d.my_session_progress.map(p => [p.session_id, p.completed]));
        document.title = `${d.course.title} — NightBoat`;
      })
      .catch(e => { error = e.message; })
      .finally(() => { loading = false; });
  });

  let syllabusHtml = $derived(detail?.syllabus ? marked.parse(detail.syllabus) as string : '');

  function formatRating(r: number) {
    return r.toFixed(1);
  }

  async function submitRating(value: number) {
    myRating = value;
    const stats = await rateCourse(id, value);
    avgRating = stats.avg_rating;
    ratingCount = stats.rating_count;
  }

  async function clearRating() {
    const stats = await unrateCourse(id);
    myRating = 0;
    avgRating = stats.avg_rating;
    ratingCount = stats.rating_count;
  }

  async function setStatus(status: 'want_to_learn' | 'learning' | 'finished' | 'dropped') {
    if (learningStatus === status) {
      // toggle off
      learningStatus = '';
      learningProgress = 0;
      try { await removeCourseLearningStatus(id); } catch { /* */ }
    } else {
      try {
        const row = await setCourseLearningStatus(id, status);
        learningStatus = row.status;
        learningProgress = row.progress;
      } catch { /* */ }
    }
  }

  async function toggleSession(sessionId: string) {
    const next = !sessionDone.get(sessionId);
    sessionDone.set(sessionId, next);
    sessionDone = new Map(sessionDone);
    try {
      const row = await setSessionProgress(id, sessionId, next);
      if (row) {
        learningStatus = row.status;
        learningProgress = row.progress;
      }
    } catch {
      sessionDone.set(sessionId, !next);
      sessionDone = new Map(sessionDone);
    }
  }

  let totalSessions = $derived(detail?.sessions?.length ?? 0);
  let doneSessions = $derived(Array.from(sessionDone.values()).filter(v => v).length);

  // Detect which resource types exist across all sessions
  let hasMaterials = $derived(detail?.sessions.some(s => (s.materials ?? []).some(m => !m.optional)) ?? false);
  let hasSupplementary = $derived(detail?.sessions.some(s => (s.materials ?? []).some(m => m.optional)) ?? false);
  let hasVideo = $derived(detail?.sessions.some(s => s.resources.some(r => r.type === 'video')) ?? false);
  let hasHw = $derived(detail?.sessions.some(s => s.resources.some(r => r.type === 'hw')) ?? false);
  let hasDiscussion = $derived(detail?.sessions.some(s => s.resources.some(r => r.type === 'discussion')) ?? false);
  let colCount = $derived(2 + (hasMaterials ? 1 : 0) + (hasSupplementary ? 1 : 0) + (hasVideo ? 1 : 0) + (hasHw ? 1 : 0) + (hasDiscussion ? 1 : 0));

  // Helper to get resources by type
  function getResources(session: import('./lib/types').CourseSession, type: string) {
    return session.resources.filter(r => r.type === type);
  }

  // Icon per material kind (falsy kind → no icon)
  function matIcon(kind?: string | null): string {
    switch (kind) {
      case 'reading': return '📘';
      case 'slides': return '🖼️';
      case 'handout': return '📄';
      case 'summary': return '📝';
      case 'notes': return '📓';
      default: return '';
    }
  }
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
        {#if detail.authors.length > 0}
          <p class="course-authors">
            {#each detail.authors as a, i}
              {#if a.did}
                <a href="/profile?did={encodeURIComponent(a.did)}">{a.name}</a>
              {:else}
                <a href="/author?id={encodeURIComponent(a.id)}">{a.name}</a>
              {/if}{#if i < detail.authors.length - 1}, {/if}
            {/each}
          </p>
        {/if}
        {#if c.description}
          <p class="course-desc">{c.description}</p>
        {/if}
        {#if detail.tags.length > 0}
          <div class="course-tags">
            {#each detail.tags as tag}
              <a href="/tag?id={encodeURIComponent(tag.tag_id)}" class="course-tag">{tag.tag_name}</a>
            {/each}
          </div>
        {/if}
        {#if c.source_url}
          <a href={c.source_url} target="_blank" rel="noopener" class="source-link">
            {t('course.source')}: {c.source_url}
          </a>
        {/if}
        {#if c.source_attribution}
          <p class="attribution">{c.source_attribution}</p>
        {/if}

        <!-- Rating -->
        <div class="rating-row">
          <span class="rating-stars-display">
            {#each [1,2,3,4,5] as star}
              {@const val = avgRating / 2}
              {@const filled = val >= star}
              {@const half = !filled && val >= star - 0.5}
              <svg class="star-svg" viewBox="0 0 24 24" width="24" height="24">
                {#if filled}
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                {:else if half}
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1" clip-path="inset(0 50% 0 0)"/>
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5" clip-path="inset(0 0 0 50%)"/>
                {:else}
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                {/if}
              </svg>
            {/each}
          </span>
          {#if ratingCount > 0}
            <span class="rating-value">{formatRating(avgRating)}</span>
            <span class="rating-count">({ratingCount})</span>
          {/if}
        </div>
        {#if getAuth()}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="my-rating" onmouseleave={() => { hoverRating = 0; }}>
            <span class="my-rating-label">{t('course.myRating')}:</span>
            <span class="star-picker">
              {#each [1,2,3,4,5] as star}
                {@const activeVal = hoverRating || myRating}
                {@const leftActive = activeVal >= star * 2 - 1}
                {@const rightActive = activeVal >= star * 2}
                <svg class="star-svg" viewBox="0 0 24 24" width="20" height="20">
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <g clip-path="inset(0 50% 0 0)" onmouseenter={() => { hoverRating = star * 2 - 1; }} onclick={() => submitRating(star * 2 - 1)} role="button" tabindex="-1">
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill={leftActive ? '#f59e0b' : 'none'} stroke={leftActive ? '#f59e0b' : '#ccc'} stroke-width="1.5"/>
                  </g>
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <g clip-path="inset(0 0 0 50%)" onmouseenter={() => { hoverRating = star * 2; }} onclick={() => submitRating(star * 2)} role="button" tabindex="-1">
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill={rightActive ? '#f59e0b' : 'none'} stroke={rightActive ? '#f59e0b' : '#ccc'} stroke-width="1.5"/>
                  </g>
                </svg>
              {/each}
            </span>
            {#if myRating > 0}
              <span class="my-rating-value">{formatRating(myRating)}</span>
              <button class="clear-rating" onclick={clearRating} title={t('course.clearRating')}>×</button>
            {/if}
          </div>
          <div class="status-actions">
            <button class="status-btn" class:active={learningStatus === 'want_to_learn'} onclick={() => setStatus('want_to_learn')}>{t('course.status.wantToLearn')}</button>
            <button class="status-btn" class:active={learningStatus === 'learning'} onclick={() => setStatus('learning')}>{t('course.status.learning')}</button>
            <button class="status-btn" class:active={learningStatus === 'finished'} onclick={() => setStatus('finished')}>{t('course.status.finished')}</button>
            <button class="status-btn" class:active={learningStatus === 'dropped'} onclick={() => setStatus('dropped')}>{t('course.status.dropped')}</button>
          </div>
          {#if (learningStatus === 'learning' || learningStatus === 'dropped') && totalSessions > 0}
            <div class="progress-section">
              <div class="progress-readout">
                <span>{t('course.progress')}: {doneSessions} / {totalSessions} {t('course.sessionsDone')}</span>
                <span class="progress-pct">{Math.round((doneSessions / totalSessions) * 100)}%</span>
              </div>
              <div class="progress-bar">
                <div class="progress-fill" style="width: {(doneSessions / totalSessions) * 100}%"></div>
              </div>
            </div>
          {/if}
        {/if}
      </div>
      <div class="header-side">
        {#if isOwner}
          <a href="/new-course?edit={encodeURIComponent(c.id)}" class="edit-btn">{t('common.edit')}</a>
        {/if}
        <div class="course-meta-box">
          <span class="meta-item">{t('course.license')}: {c.license}</span>
          <span class="meta-item">{t('course.language')}: {c.lang.toUpperCase()}</span>
        </div>
      </div>
    </div>

    <div class="course-body">
      <div class="body-main">
        {#if syllabusHtml}
          <section class="syllabus">
            <h2>{t('course.syllabus')}</h2>
            <div class="content">{@html syllabusHtml}</div>
          </section>
        {/if}

        {#if detail.sessions.length > 0}
          <section class="schedule">
            <h2>{t('course.calendar')}</h2>
            <table class="schedule-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>{t('course.topic')}</th>
                  {#if hasMaterials}<th>{t('course.materials')}</th>{/if}
                  {#if hasSupplementary}<th>{t('course.supplementary')}</th>{/if}
                  {#if hasVideo}<th>{t('course.video')}</th>{/if}
                  {#if hasDiscussion}<th>{t('course.discussion')}</th>{/if}
                  {#if hasHw}<th>{t('course.hw')}</th>{/if}
                </tr>
              </thead>
              <tbody>
                {#each detail.sessions as s}
                  {@const isExam = (s.materials?.length ?? 0) === 0 && s.resources.length === 0}
                  <tr class:session-exam={isExam}>
                    <td class="session-num">
                      {s.sort_order}
                      {#if getAuth() && !isExam}
                        <button
                          class="session-check"
                          class:done={sessionDone.get(s.id)}
                          onclick={() => toggleSession(s.id)}
                          title={sessionDone.get(s.id) ? t('course.markUndone') : t('course.markDone')}
                        ></button>
                      {/if}
                    </td>
                    {#if isExam}
                      <td class="session-topic" colspan={colCount - 1}>
                        <strong>{s.topic || ''}</strong>
                      </td>
                    {:else}
                      <td class="session-topic">
                        {s.topic || ''}
                        {#if s.tags && s.tags.length > 0}
                          <div class="session-tags">
                            {#each s.tags as tag}
                              <a href="/tag?id={encodeURIComponent(tag.tag_id)}" class="session-tag">{tag.tag_name}</a>
                            {/each}
                          </div>
                        {/if}
                      </td>
                      {#if hasMaterials}
                        <td class="session-materials">
                          {#each (s.materials ?? []).filter(m => !m.optional) as m}
                            {#if m.url}
                              <a href={m.url} target="_blank" rel="noopener" class="res-link res-mat" title={m.label}>
                                {#if matIcon(m.kind)}<span class="mat-icon">{matIcon(m.kind)}</span>{/if}{m.label}
                              </a>
                            {:else}
                              <span class="res-mat-plain" title={m.label}>
                                {#if matIcon(m.kind)}<span class="mat-icon">{matIcon(m.kind)}</span>{/if}{m.label}
                              </span>
                            {/if}
                          {/each}
                        </td>
                      {/if}
                      {#if hasSupplementary}
                        <td class="session-materials">
                          {#each (s.materials ?? []).filter(m => m.optional) as m}
                            {#if m.url}
                              <a href={m.url} target="_blank" rel="noopener" class="res-link res-mat" title={m.label}>
                                {#if matIcon(m.kind)}<span class="mat-icon">{matIcon(m.kind)}</span>{/if}{m.label}
                              </a>
                            {:else}
                              <span class="res-mat-plain" title={m.label}>
                                {#if matIcon(m.kind)}<span class="mat-icon">{matIcon(m.kind)}</span>{/if}{m.label}
                              </span>
                            {/if}
                          {/each}
                        </td>
                      {/if}
                      {#if hasVideo}
                        <td class="session-video">
                          {#each getResources(s, 'video') as r}
                            <a href={r.url} target="_blank" rel="noopener" class="res-link res-video">&#9654; {r.label}</a>
                          {/each}
                        </td>
                      {/if}
                      {#if hasDiscussion}
                        <td class="session-disc">
                          {#each getResources(s, 'discussion') as r}
                            <a href={r.url} target="_blank" rel="noopener" class="res-link res-disc">&#128172; {r.label}</a>
                          {/each}
                        </td>
                      {/if}
                      {#if hasHw}
                        <td class="session-hw">
                          {#each getResources(s, 'hw') as r}
                            <a href={r.url} target="_blank" rel="noopener" class="res-link res-hw">&#9998; {r.label}</a>
                          {/each}
                        </td>
                      {/if}
                    {/if}
                  </tr>
                {/each}
              </tbody>
            </table>
          </section>
        {/if}

      </div>

      <div class="body-side">
        {#if detail.textbooks.length > 0}
          <section class="textbooks">
            <h3>{t('course.textbooks')}</h3>
            {#each detail.textbooks as tb}
              <a href="/book?id={encodeURIComponent(tb.book_id)}" class="textbook-card">
                {#if tb.cover_url}
                  <img src={tb.cover_url} alt="" class="textbook-cover" />
                {/if}
                <div class="textbook-info">
                  <span class="textbook-title">{loc(tb.title)}</span>
                  <span class="textbook-authors">{tb.authors.join(', ')}</span>
                  <span class="textbook-role">{tb.role}</span>
                </div>
              </a>
            {/each}
          </section>
        {/if}

        {#if detail.prerequisites.length > 0}
          <section class="prereqs">
            <h3>{t('course.prerequisites')}</h3>
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
            <h3>{t('course.skillTrees')}</h3>
            {#each detail.skill_trees as st}
              <a href="/skill-tree?uri={encodeURIComponent(st.tree_uri)}" class="tree-link">
                <span class="tree-role">{st.role}</span>
                {st.title}
              </a>
            {/each}
          </section>
        {/if}

        <section class="qa-summary">
          <h3>
            {t('course.qa')}
            <span class="qa-count">{detail.discussion_count}</span>
          </h3>
          {#if detail.discussions.length === 0}
            <p class="qa-empty">{t('course.noDiscussions')}</p>
          {:else}
            {#each detail.discussions as c}
              <a href="/course-discussions?id={encodeURIComponent(id)}#c-{c.id}" class="qa-item" title={c.title || c.body}>
                <span class="qa-body">{c.title || c.body.split('\n')[0]}</span>
                <span class="qa-meta">
                  <span class="qa-author">{c.author_handle || c.did.slice(0, 12)}</span>
                </span>
              </a>
            {/each}
            {#if detail.discussion_count > detail.discussions.length}
              <a href="/course-discussions?id={encodeURIComponent(id)}" class="qa-more">{t('course.viewAll')} ({detail.discussion_count}) →</a>
            {/if}
          {/if}
          {#if getAuth()}
            <a href="/course-discussions?id={encodeURIComponent(id)}#new" class="qa-ask-btn">{t('course.askQuestion')}</a>
          {/if}
        </section>
      </div>
    </div>

    <!-- Reviews (opinions on the course) -->
    <section class="reviews-section">
      <h2>{t('course.reviews')}</h2>
      {#if getAuth()}
        <a href="/new?category=review&course_id={encodeURIComponent(id)}" class="write-review-btn">{t('course.writeReview')}</a>
      {/if}
      {#if detail.reviews.length === 0}
        <p class="meta">{t('course.noReviews')}</p>
      {:else}
        {#each detail.reviews as review}
          <a href="/article?uri={encodeURIComponent(review.at_uri)}" class="review-card">
            <div class="review-header">
              <span class="review-author">{review.author_display_name || review.author_handle || review.did.slice(0, 16)}</span>
              <span class="review-date">{new Date(review.created_at).toLocaleDateString()}</span>
              {#if review.course_session_id}
                {@const lec = detail.sessions.find(s => s.id === review.course_session_id)}
                {#if lec}<span class="review-session">{t('course.onLecture') || 'Lecture'} {lec.sort_order}: {lec.topic}</span>{/if}
              {/if}
            </div>
            <h3 class="review-title">{review.title}</h3>
            {#if review.summary}
              <p class="review-desc">{review.summary}</p>
            {/if}
            <div class="review-stats">
              <span>{review.vote_score} votes</span>
              <span>{review.comment_count} comments</span>
            </div>
          </a>
        {/each}
        {#if detail.review_count > detail.reviews.length}
          <a href="/course-reviews?id={encodeURIComponent(id)}" class="view-all">{t('course.viewAll')} ({detail.review_count}) →</a>
        {/if}
      {/if}
    </section>

    <!-- Notes (learner thoughts / knowledge supplements) -->
    <section class="reviews-section">
      <h2>{t('course.learnerNotes') || 'Notes'}</h2>
      {#if getAuth()}
        <a href="/new?category=note&course_id={encodeURIComponent(id)}" class="write-review-btn">{t('course.writeNote') || 'Write a note'}</a>
      {/if}
      {#if detail.notes.length === 0}
        <p class="meta">{t('course.noNotes') || 'No notes yet.'}</p>
      {:else}
        {#each detail.notes as note}
          <a href="/article?uri={encodeURIComponent(note.at_uri)}" class="review-card">
            <div class="review-header">
              <span class="review-author">{note.author_display_name || note.author_handle || note.did.slice(0, 16)}</span>
              <span class="review-date">{new Date(note.created_at).toLocaleDateString()}</span>
              {#if note.course_session_id}
                {@const lec = detail.sessions.find(s => s.id === note.course_session_id)}
                {#if lec}<span class="review-session">{t('course.onLecture') || 'Lecture'} {lec.sort_order}: {lec.topic}</span>{/if}
              {/if}
            </div>
            <h3 class="review-title">{note.title}</h3>
            {#if note.summary}
              <p class="review-desc">{note.summary}</p>
            {/if}
            <div class="review-stats">
              <span>{note.vote_score} votes</span>
              <span>{note.comment_count} comments</span>
            </div>
          </a>
        {/each}
        {#if detail.note_count > detail.notes.length}
          <a href="/course-notes?id={encodeURIComponent(id)}" class="view-all">{t('course.viewAll')} ({detail.note_count}) →</a>
        {/if}
      {/if}
    </section>
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
  .course-authors { font-size: 14px; color: var(--text-secondary); margin: 6px 0; }
  .course-authors a { color: var(--text-primary); text-decoration: none; }
  .course-authors a:hover { color: var(--accent); }
  .course-desc { font-size: 14px; color: var(--text-secondary); line-height: 1.6; margin: 12px 0; }
  .course-tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 12px 0; }
  .course-tag { font-size: 12px; padding: 3px 10px; border-radius: 3px; background: rgba(95,155,101,0.1); color: var(--accent); text-decoration: none; transition: background 0.15s; }
  .course-tag:hover { background: rgba(95,155,101,0.2); text-decoration: none; }
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
  .syllabus h2, .schedule h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.3rem; margin: 0 0 16px; }
  .syllabus { margin-bottom: 32px; }

  .schedule { margin-bottom: 32px; }
  .schedule-table { width: 100%; border-collapse: collapse; font-size: 14px; }
  .schedule-table th { text-align: left; font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.03em; color: var(--text-hint); padding: 8px 12px; border-bottom: 2px solid var(--border); }
  .schedule-table td { padding: 8px 12px; border-bottom: 1px solid var(--border); vertical-align: top; }
  .schedule-table tbody tr:hover { background: var(--bg-hover, rgba(0,0,0,0.02)); }
  .session-exam { background: var(--bg-hover, rgba(0,0,0,0.02)); }
  .session-num { font-weight: 600; color: var(--text-hint); width: 40px; }
  .session-topic { color: var(--text-primary); width: 40%; }
  .session-readings { color: var(--text-hint); font-size: 13px; white-space: nowrap; width: 18%; }
  .session-video { white-space: nowrap; }
  .session-notes { white-space: nowrap; }
  .session-hw { white-space: nowrap; }
  .session-materials { max-width: 260px; }
  .session-materials > * { display: block; max-width: 100%; overflow: hidden; text-overflow: ellipsis; margin-bottom: 3px; }
  .session-materials > *:last-child { margin-bottom: 0; }
  .mat-icon { margin-right: 3px; font-size: 11px; }
  .res-mat { font-size: 11px; color: var(--text-primary); background: var(--bg-hover, #f5f5f5); padding: 2px 8px; border-radius: 3px; text-decoration: none; white-space: nowrap; }
  a.res-mat { color: var(--text-primary); }
  a.res-mat:hover { color: var(--accent); text-decoration: none; opacity: 0.85; }
  .res-mat-plain { font-size: 11px; color: var(--text-secondary); white-space: nowrap; }
  .res-link { font-size: 11px; padding: 2px 8px; border-radius: 3px; text-decoration: none; white-space: nowrap; transition: opacity 0.15s; }
  .res-link:hover { opacity: 0.8; text-decoration: none; }
  .res-video { background: rgba(220,38,38,0.1); color: #dc2626; }
  .res-notes { background: rgba(59,130,246,0.1); color: #3b82f6; }
  .res-hw { background: rgba(16,185,129,0.1); color: #059669; }
  .res-disc { background: rgba(168,85,247,0.1); color: #7c3aed; }
  .session-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
  .session-tag { font-size: 10px; padding: 1px 6px; border-radius: 3px; background: rgba(95,155,101,0.08); color: var(--accent); text-decoration: none; }
  .session-tag:hover { background: rgba(95,155,101,0.18); text-decoration: none; }
  .res-reading { font-size: 11px; color: var(--text-hint); }
  .series-link { display: block; padding: 12px 16px; border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: 0 4px 4px 0; margin-bottom: 8px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .series-link:hover { border-color: var(--accent); text-decoration: none; }
  .series-role { font-size: 11px; font-weight: 600; color: var(--accent); text-transform: uppercase; display: block; margin-bottom: 2px; }
  .series-title { font-family: var(--font-serif); font-size: 1.05rem; display: block; }
  .series-desc { font-size: 13px; color: var(--text-secondary); display: block; margin-top: 4px; }

  .qa-summary { margin-bottom: 20px; }
  .qa-summary h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; }
  .qa-count { font-size: 12px; color: var(--text-hint); font-weight: 400; }
  .qa-empty { font-size: 12px; color: var(--text-hint); margin: 4px 0 8px; }
  .qa-item { display: flex; flex-direction: column; gap: 3px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 6px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .qa-item:hover { border-color: var(--accent); text-decoration: none; }
  .qa-body { font-size: 12px; color: var(--text-primary); line-height: 1.4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .qa-meta { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--text-hint); }
  .qa-author { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; flex: 1; }
  .qa-replies { flex-shrink: 0; margin-left: 6px; }
  .qa-more { display: block; font-size: 12px; color: var(--accent); text-decoration: none; margin-top: 4px; }
  .qa-more:hover { text-decoration: underline; }
  .qa-ask-btn { display: inline-block; margin-top: 6px; padding: 4px 10px; font-size: 12px; background: var(--accent); color: white; border-radius: 3px; text-decoration: none; }
  .qa-ask-btn:hover { opacity: 0.9; text-decoration: none; }

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

  /* Rating */
  .rating-row { display: flex; align-items: center; gap: 6px; margin: 12px 0 4px; }
  .rating-stars-display { display: flex; gap: 1px; }
  .rating-value { font-size: 14px; font-weight: 600; color: var(--text-primary); }
  .rating-count { font-size: 12px; color: var(--text-hint); }
  .my-rating { display: flex; align-items: center; gap: 6px; margin: 4px 0 12px; }
  .my-rating-label { font-size: 12px; color: var(--text-hint); }
  .star-picker { display: flex; gap: 1px; cursor: pointer; }
  .star-svg { display: block; }
  .my-rating-value { font-size: 12px; color: #f59e0b; font-weight: 600; }
  .clear-rating { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 4px; line-height: 1; }
  .clear-rating:hover { color: #c00; }

  .status-actions { display: flex; gap: 6px; margin-top: 10px; flex-wrap: wrap; }
  .status-btn { padding: 4px 12px; border: 1px solid var(--border); background: var(--bg-white); border-radius: 4px; font-size: 12px; cursor: pointer; color: var(--text-secondary); transition: all 0.15s; }
  .status-btn:hover { border-color: var(--accent); color: var(--accent); }
  .status-btn.active { background: var(--accent); color: white; border-color: var(--accent); }

  .progress-section { margin-top: 10px; max-width: 360px; }
  .progress-readout { display: flex; justify-content: space-between; font-size: 12px; color: var(--text-secondary); margin-bottom: 4px; }
  .progress-pct { color: var(--accent); font-weight: 500; }
  .progress-bar { height: 4px; background: var(--border); border-radius: 2px; overflow: hidden; }
  .progress-fill { height: 100%; background: var(--accent); transition: width 0.2s; }

  .session-check { width: 14px; height: 14px; border: 1.5px solid var(--border); border-radius: 3px; background: transparent; cursor: pointer; margin-left: 6px; padding: 0; vertical-align: middle; position: relative; }
  .session-check:hover { border-color: var(--accent); }
  .session-check.done { background: var(--accent); border-color: var(--accent); }
  .session-check.done::after { content: ''; position: absolute; left: 3px; top: 0px; width: 4px; height: 8px; border: solid white; border-width: 0 2px 2px 0; transform: rotate(45deg); }

  /* Reviews */
  .reviews-section { margin-top: 40px; padding-top: 24px; border-top: 1px solid var(--border); }
  .reviews-section h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.3rem; margin: 0 0 16px; }
  .write-review-btn { font-size: 13px; padding: 5px 14px; border: 1px solid var(--accent); border-radius: 4px; color: var(--accent); text-decoration: none; display: inline-block; margin-bottom: 16px; }
  .write-review-btn:hover { background: var(--accent); color: white; text-decoration: none; }
  .review-card { display: block; padding: 16px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 12px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .review-card:hover { border-color: var(--accent); text-decoration: none; }
  .review-header { display: flex; justify-content: space-between; font-size: 12px; color: var(--text-hint); margin-bottom: 4px; }
  .review-title { font-family: var(--font-serif); font-size: 1rem; margin: 0 0 4px; font-weight: 400; }
  .review-desc { font-size: 13px; color: var(--text-secondary); margin: 0 0 8px; }
  .review-stats { font-size: 11px; color: var(--text-hint); display: flex; gap: 12px; }

  .view-all { display: inline-block; margin-top: 8px; font-size: 13px; color: var(--accent); text-decoration: none; }
  .view-all:hover { text-decoration: underline; }

  @media (max-width: 768px) {
    .course-header { flex-direction: column; }
    .header-side { width: 100%; }
    .course-body { flex-direction: column; }
    .body-side { width: 100%; }
  }
</style>
