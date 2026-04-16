<script lang="ts">
  import { getCourseDetail, rateCourse, listComments, createComment, voteComment } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { marked } from 'marked';
  import type { CourseDetail, Comment } from '../lib/types';

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

  // Comments / Q&A state
  let comments = $state<Comment[]>([]);
  let newComment = $state('');
  let replyTo = $state<string | null>(null);
  let replyText = $state('');

  $effect(() => {
    if (!id) return;
    loading = true;
    error = '';
    getCourseDetail(id)
      .then(d => {
        detail = d;
        avgRating = d.rating.avg_rating;
        ratingCount = d.rating.rating_count;
        document.title = `${d.course.title} — NightBoat`;
      })
      .catch(e => { error = e.message; })
      .finally(() => { loading = false; });

    // Load comments
    listComments(`course:${id}`).then(c => { comments = c; }).catch(() => {});
  });

  let syllabusHtml = $derived(detail?.syllabus ? marked.parse(detail.syllabus) as string : '');

  function formatRating(r: number) {
    return (r / 2).toFixed(1);
  }

  async function submitRating(value: number) {
    myRating = value;
    const stats = await rateCourse(id, value);
    avgRating = stats.avg_rating;
    ratingCount = stats.rating_count;
  }

  async function postComment() {
    if (!newComment.trim()) return;
    const c = await createComment(`course:${id}`, newComment.trim());
    comments = [c, ...comments];
    newComment = '';
  }

  async function postReply(parentId: string) {
    if (!replyText.trim()) return;
    const c = await createComment(`course:${id}`, replyText.trim(), parentId);
    comments = [c, ...comments];
    replyText = '';
    replyTo = null;
  }

  // Separate top-level and replies
  let topComments = $derived(comments.filter(c => !c.parent_id));
  let replies = $derived((parentId: string) => comments.filter(c => c.parent_id === parentId));

  // Detect which columns have any data
  let hasReadings = $derived(detail?.sessions.some(s => s.readings) ?? false);
  let hasVideo = $derived(detail?.sessions.some(s => s.video_url || s.notes_url) ?? false);
  let hasHw = $derived(detail?.sessions.some(s => s.assignment_url || s.discussion_url) ?? false);
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
            {/if}
          </div>
        {/if}
      </div>
      <div class="header-side">
        {#if detail.staff.length > 0}
          <div class="staff-list">
            <h3>{t('course.staff')}</h3>
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
              {@const colCount = 2 + (hasReadings ? 1 : 0) + (hasVideo ? 1 : 0) + (hasHw ? 1 : 0)}
              <thead>
                <tr>
                  <th>#</th>
                  <th>{t('course.topic')}</th>
                  {#if hasReadings}<th>{t('course.readings')}</th>{/if}
                  {#if hasVideo}<th>{t('course.video')}</th>{/if}
                  {#if hasHw}<th>{t('course.hw')}</th>{/if}
                </tr>
              </thead>
              <tbody>
                {#each detail.sessions as s}
                  {@const isExam = !s.readings && !s.video_url && !s.notes_url && !s.assignment_url && !s.discussion_url}
                  <tr class:session-exam={isExam}>
                    <td class="session-num">{s.sort_order}</td>
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
                      {#if hasReadings}
                        <td class="session-readings">
                          {#if s.readings}
                            {#if s.readings.startsWith('/')}
                              <a href={s.readings} class="res-link res-notes">&#128196; {t('course.notes')}</a>
                            {:else}
                              <span class="res-reading">{s.readings}</span>
                            {/if}
                          {/if}
                        </td>
                      {/if}
                      {#if hasVideo}
                        <td class="session-video">
                          {#if s.video_url}
                            <a href={s.video_url} target="_blank" rel="noopener" class="res-link res-video">&#9654; {t('course.video')}</a>
                          {/if}
                          {#if s.notes_url}
                            <a href={s.notes_url} target="_blank" rel="noopener" class="res-link res-notes">&#128196; {t('course.notes')}</a>
                          {/if}
                        </td>
                      {/if}
                      {#if hasHw}
                        <td class="session-hw">
                          {#if s.assignment_url}
                            <a href={s.assignment_url} target="_blank" rel="noopener" class="res-link res-hw">&#9998; {s.assignment_label || t('course.hw')}</a>
                          {/if}
                          {#if s.discussion_url}
                            <a href={s.discussion_url} target="_blank" rel="noopener" class="res-link res-disc">&#128172; {s.discussion_label || t('course.discussion')}</a>
                          {/if}
                        </td>
                      {/if}
                    {/if}
                  </tr>
                {/each}
              </tbody>
            </table>
          </section>
        {/if}

        {#if detail.series.length > 0}
          <section class="course-series">
            <h2>{t('course.materials')}</h2>
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
            <h3>{t('course.textbooks')}</h3>
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
      </div>
    </div>

    <!-- Reviews -->
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
            </div>
            <h3 class="review-title">{review.title}</h3>
            {#if review.description}
              <p class="review-desc">{review.description}</p>
            {/if}
            <div class="review-stats">
              <span>{review.vote_score} votes</span>
              <span>{review.comment_count} comments</span>
            </div>
          </a>
        {/each}
      {/if}
    </section>

    <!-- Q&A Discussion -->
    <section class="qa-section">
      <h2>{t('course.qa')}</h2>
      {#if getAuth()}
        <div class="comment-form">
          <textarea bind:value={newComment} placeholder={t('course.askQuestion')} rows="3"></textarea>
          <button class="post-btn" onclick={postComment} disabled={!newComment.trim()}>{t('course.post')}</button>
        </div>
      {/if}
      {#if topComments.length === 0}
        <p class="meta">{t('course.noDiscussions')}</p>
      {:else}
        {#each topComments as c}
          <div class="comment">
            <div class="comment-header">
              <a href="/profile?did={encodeURIComponent(c.did)}" class="comment-author">{c.author_handle || c.did.slice(0, 16)}</a>
              <span class="comment-date">{new Date(c.created_at).toLocaleDateString()}</span>
            </div>
            <p class="comment-body">{c.body}</p>
            <div class="comment-actions">
              <span class="comment-score">{c.vote_score}</span>
              {#if getAuth()}
                <button class="reply-btn" onclick={() => { replyTo = replyTo === c.id ? null : c.id; }}>{t('course.reply')}</button>
              {/if}
            </div>
            {#if replyTo === c.id}
              <div class="reply-form">
                <textarea bind:value={replyText} placeholder={t('course.reply')} rows="2"></textarea>
                <button class="post-btn" onclick={() => postReply(c.id)} disabled={!replyText.trim()}>{t('course.post')}</button>
              </div>
            {/if}
            {#each replies(c.id) as r}
              <div class="comment reply">
                <div class="comment-header">
                  <a href="/profile?did={encodeURIComponent(r.did)}" class="comment-author">{r.author_handle || r.did.slice(0, 16)}</a>
                  <span class="comment-date">{new Date(r.created_at).toLocaleDateString()}</span>
                </div>
                <p class="comment-body">{r.body}</p>
              </div>
            {/each}
          </div>
        {/each}
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
  .syllabus h2, .course-series h2, .schedule h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.3rem; margin: 0 0 16px; }
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
  .session-video { white-space: nowrap; width: 15%; }
  .session-hw { white-space: nowrap; width: 12%; }
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

  /* Q&A */
  .qa-section { margin-top: 32px; padding-top: 24px; border-top: 1px solid var(--border); }
  .qa-section h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.3rem; margin: 0 0 16px; }
  .comment-form { margin-bottom: 20px; }
  .comment-form textarea { width: 100%; padding: 10px; border: 1px solid var(--border); border-radius: 6px; font-size: 14px; font-family: inherit; resize: vertical; background: var(--bg-page); color: var(--text-primary); }
  .comment-form textarea:focus { outline: none; border-color: var(--accent); }
  .post-btn { margin-top: 8px; padding: 6px 16px; background: var(--accent); color: white; border: none; border-radius: 4px; font-size: 13px; cursor: pointer; }
  .post-btn:disabled { opacity: 0.5; cursor: default; }
  .comment { padding: 12px 0; border-bottom: 1px solid var(--border); }
  .comment.reply { margin-left: 24px; padding: 8px 0; border-bottom: 1px dashed var(--border); }
  .comment-header { display: flex; gap: 8px; align-items: center; margin-bottom: 4px; }
  .comment-author { font-size: 13px; font-weight: 500; color: var(--text-primary); text-decoration: none; }
  .comment-author:hover { color: var(--accent); text-decoration: none; }
  .comment-date { font-size: 11px; color: var(--text-hint); }
  .comment-body { font-size: 14px; color: var(--text-primary); line-height: 1.6; margin: 0; }
  .comment-actions { display: flex; gap: 12px; align-items: center; margin-top: 4px; }
  .comment-score { font-size: 12px; color: var(--text-hint); }
  .reply-btn { font-size: 12px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 0; }
  .reply-form { margin-top: 8px; margin-left: 24px; }
  .reply-form textarea { width: 100%; padding: 8px; border: 1px solid var(--border); border-radius: 4px; font-size: 13px; font-family: inherit; resize: vertical; background: var(--bg-page); color: var(--text-primary); }

  @media (max-width: 768px) {
    .course-header { flex-direction: column; }
    .header-side { width: 100%; }
    .course-body { flex-direction: column; }
    .body-side { width: 100%; }
  }
</style>
