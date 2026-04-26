<script lang="ts">
  import { getCourseDetail, rateCourse, unrateCourse, setCourseLearningStatus, removeCourseLearningStatus, setSessionProgress, createCourseSession, updateCourseSession, deleteCourseSession, addCourseTag, removeCourseTag, lookupTag, searchTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { tagStore } from '../lib/tagStore.svelte';
  $effect(() => { tagStore.ensure(); });

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

  // Course-level tag editor state
  let tagInput = $state('');
  let tagSuggestions = $state<{ tag_id: string; name: string; id: string }[]>([]);
  let tagSaving = $state(false);
  let tagError = $state('');
  let tagSuggestTimer: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(tagSuggestTimer);
    const q = tagInput.trim();
    if (!q || q.startsWith('tg-')) { tagSuggestions = []; return; }
    tagSuggestTimer = setTimeout(async () => {
      try { tagSuggestions = (await searchTags(q)) as any; } catch { tagSuggestions = []; }
    }, 150);
  });

  async function asTagId(input: string): Promise<string | null> {
    const s = input.trim();
    if (!s) return null;
    if (s.startsWith('tg-')) return s;
    try { return (await lookupTag(s)).tag_id; }
    catch { tagError = t('books.tagNotFound').replace('{name}', s); return null; }
  }

  async function submitAddTag(fromSuggestion?: string) {
    if (!detail) return;
    tagError = '';
    const id = fromSuggestion ?? await asTagId(tagInput);
    if (!id) return;
    if (detail.tags.some(t => t.tag_id === id)) { tagInput = ''; tagSuggestions = []; return; }
    tagSaving = true;
    try {
      await addCourseTag(detail.course.id, id);
      // Refresh so tag_name comes back from server
      detail = await getCourseDetail(detail.course.id);
      tagInput = '';
      tagSuggestions = [];
    } catch (e: any) { tagError = e.message ?? String(e); }
    finally { tagSaving = false; }
  }

  async function submitRemoveTag(tagId: string) {
    if (!detail) return;
    try {
      await removeCourseTag(detail.course.id, tagId);
      detail = await getCourseDetail(detail.course.id);
    } catch (e: any) { tagError = e.message ?? String(e); }
  }

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

  // Each attachment kind becomes its own column, in a fixed order. A
  // column shows up only when at least one session has an entry of
  // that kind. `code`/`other` collapse into a single "misc" column so
  // we don't sprout columns for one-off oddities. Column order matches
  // the typical reading flow: watch the lecture → take notes / read →
  // homework / discussion → odds and ends.
  const KIND_COLUMNS: { id: string; kinds: string[]; labelKey: string }[] = [
    { id: 'video',      kinds: ['video'],      labelKey: 'course.video' },
    { id: 'notes',      kinds: ['notes'],      labelKey: 'course.notes' },
    { id: 'slides',     kinds: ['slides'],     labelKey: 'course.slides' },
    { id: 'recitation', kinds: ['recitation'], labelKey: 'course.recitation' },
    { id: 'reading',    kinds: ['reading'],    labelKey: 'course.reading' },
    { id: 'outline',    kinds: ['outline'],    labelKey: 'course.outline' },
    { id: 'summary',    kinds: ['summary'],    labelKey: 'course.summary' },
    { id: 'homework',   kinds: ['homework'],   labelKey: 'course.hw' },
    { id: 'discussion', kinds: ['discussion'], labelKey: 'course.discussion' },
    { id: 'misc',       kinds: ['code', 'other'], labelKey: 'course.misc' },
  ];
  function groupForKind(kind: string): string {
    const col = KIND_COLUMNS.find(c => c.kinds.includes(kind));
    return col?.id ?? 'misc';
  }
  function groupAttachments(s: import('../lib/types').CourseSession, group: string) {
    return (s.attachments ?? []).filter(a => groupForKind(a.kind) === group);
  }
  let visibleColumns = $derived(
    KIND_COLUMNS.filter(col =>
      detail?.sessions.some(s => groupAttachments(s, col.id).length > 0) ?? false,
    ),
  );
  let colCount = $derived(2 + visibleColumns.length + (isOwner ? 1 : 0));

  function attachIcon(kind: string): string {
    switch (kind) {
      case 'video': return '▶';
      case 'reading': return '📘';
      case 'slides': return '🖼️';
      case 'recitation': return '🎓';
      case 'summary': return '📝';
      case 'notes': return '📓';
      case 'code': return '⚙';
      case 'homework': return '✎';
      case 'discussion': return '💬';
      case 'outline': return '🗒';
      default: return '🔗';
    }
  }

  // Session CRUD state
  let showSessionEdit = $state(false);
  let sessionEditId = $state('');
  let sessionEditTopic = $state('');
  let sessionEditDate = $state('');
  let sessionEditOrder = $state<number | ''>('');
  let sessionEditSaving = $state(false);
  let sessionEditError = $state('');

  function openSessionAdd() {
    sessionEditId = '';
    sessionEditTopic = '';
    sessionEditDate = '';
    sessionEditOrder = (detail?.sessions.length ?? 0) + 1;
    sessionEditError = '';
    showSessionEdit = true;
  }

  function openSessionEdit(s: import('../lib/types').CourseSession) {
    sessionEditId = s.id;
    sessionEditTopic = s.topic || '';
    sessionEditDate = s.date || '';
    sessionEditOrder = s.sort_order;
    sessionEditError = '';
    showSessionEdit = true;
  }

  async function saveSessionEdit() {
    sessionEditSaving = true;
    sessionEditError = '';
    try {
      const payload = {
        topic: sessionEditTopic.trim() || undefined,
        date: sessionEditDate.trim() || undefined,
        sort_order: typeof sessionEditOrder === 'number' ? sessionEditOrder : undefined,
      };
      if (sessionEditId) {
        await updateCourseSession(id, sessionEditId, payload);
      } else {
        await createCourseSession(id, payload);
      }
      showSessionEdit = false;
      const d = await getCourseDetail(id);
      detail = d;
    } catch (e: any) {
      sessionEditError = e.message || 'Save failed';
    } finally {
      sessionEditSaving = false;
    }
  }

  async function confirmDeleteSession(sessionId: string) {
    if (!confirm(t('course.deleteSessionConfirm'))) return;
    try {
      await deleteCourseSession(id, sessionId);
      const d = await getCourseDetail(id);
      detail = d;
    } catch (e: any) {
      alert(e.message || 'Delete failed');
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
        {#if detail.siblings.length > 0}
          <div class="course-siblings" aria-label={t('course.otherIterations')}>
            <span class="siblings-label">{t('course.otherIterations')}:</span>
            {#each detail.siblings as s (s.id)}
              <a class="sibling-chip" href="/course?id={encodeURIComponent(s.id)}">
                {s.semester || s.title}
              </a>
            {/each}
          </div>
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
        <div class="course-tags">
          {#each detail.tags as tag}
            <span class="course-tag">
              <a href="/tag?id={encodeURIComponent(tag.tag_id)}">{tag.tag_name}</a>
              {#if isOwner}
                <button class="tag-rm" title={t('course.removeTag')} onclick={() => submitRemoveTag(tag.tag_id)}>×</button>
              {/if}
            </span>
          {/each}
          {#if isOwner}
            <div class="course-tag-add">
              <input
                class="tag-add-input"
                bind:value={tagInput}
                placeholder={t('course.addTagPlaceholder')}
                onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitAddTag(); } }}
                disabled={tagSaving}
              />
              <button class="tag-add-btn" onclick={() => submitAddTag()} disabled={tagSaving || !tagInput.trim()}>
                {tagSaving ? t('common.saving') : t('common.add')}
              </button>
              {#if tagSuggestions.length > 0}
                <div class="tag-suggest-list">
                  {#each tagSuggestions.slice(0, 8) as s}
                    <button type="button" onclick={() => submitAddTag(s.tag_id)}>{s.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>
        {#if tagError}<p class="tag-error">{tagError}</p>{/if}
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

        {#if detail.sessions.length > 0 || isOwner}
          <section class="schedule">
            <h2>{t('course.calendar')}</h2>
            <table class="schedule-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>{t('course.topic')}</th>
                  {#each visibleColumns as col}
                    <th>{t(col.labelKey)}</th>
                  {/each}
                  {#if isOwner}<th class="session-actions-col"></th>{/if}
                </tr>
              </thead>
              <tbody>
                {#each detail.sessions as s}
                  {@const atts = s.attachments ?? []}
                  {@const isSection = s.kind === 'section'}
                  {@const isExam = !isSection && atts.length === 0}
                  {@const lectureIdx = detail.sessions
                    .slice(0, detail.sessions.indexOf(s))
                    .filter(p => p.kind !== 'section').length + 1}
                  {#if isSection}
                    <tr class="session-section">
                      <td colspan={colCount}>{s.topic || ''}</td>
                    </tr>
                  {:else}
                  <tr class:session-exam={isExam}>
                    <td class="session-num">
                      {lectureIdx}
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
                      <td class="session-topic" colspan={colCount - 1 - (isOwner ? 1 : 0)}>
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
                      {#each visibleColumns as col}
                        <td class="session-cell-{col.id}">
                          {#each groupAttachments(s, col.id).filter(a => a.required) as a}
                            {#if a.url}
                              <a href={a.url} target="_blank" rel="noopener"
                                 class="res-chip res-chip-{a.kind}" title={a.label}>
                                <span class="chip-icon">{attachIcon(a.kind)}</span>{a.label}
                              </a>
                            {:else}
                              <span class="res-text" title={a.label}>{a.label}</span>
                            {/if}
                          {/each}
                          {#each groupAttachments(s, col.id).filter(a => !a.required) as a}
                            {#if a.url}
                              <a href={a.url} target="_blank" rel="noopener"
                                 class="res-chip res-chip-{a.kind} res-chip-optional" title={a.label}>
                                <span class="chip-icon">{attachIcon(a.kind)}</span>{a.label}
                              </a>
                            {:else}
                              <span class="res-text res-text-optional" title={a.label}>{a.label}</span>
                            {/if}
                          {/each}
                        </td>
                      {/each}
                    {/if}
                    {#if isOwner}
                      <td class="session-actions">
                        <button class="session-action-btn" onclick={() => openSessionEdit(s)} title={t('course.editSession')}>✎</button>
                        <button class="session-action-btn" onclick={() => confirmDeleteSession(s.id)} title="Delete">🗑</button>
                      </td>
                    {/if}
                  </tr>
                  {/if}
                {/each}
                {#if isOwner}
                  <tr class="session-add-row">
                    <td colspan={colCount}>
                      <button class="session-add-btn" onclick={openSessionAdd}>{t('course.addSession')}</button>
                    </td>
                  </tr>
                {/if}
              </tbody>
            </table>
          </section>
        {/if}

      </div>

      <div class="body-side">
        {#snippet bookCard(tb: NonNullable<typeof detail>['textbooks'][number])}
          <a href="/book?id={encodeURIComponent(tb.book_id)}" class="textbook-card">
            {#if tb.cover_url}
              <img src={tb.cover_url} alt="" class="textbook-cover" />
            {/if}
            <div class="textbook-info">
              <span class="textbook-title">{loc(tb.title)}</span>
              <span class="textbook-authors">{tb.authors.join(', ')}</span>
            </div>
          </a>
        {/snippet}

        {#if detail.textbooks.some(t => t.role === 'required')}
          <section class="textbooks">
            <h3>{t('course.textbooks')}</h3>
            {#each detail.textbooks.filter(t => t.role === 'required') as tb}
              {@render bookCard(tb)}
            {/each}
          </section>
        {/if}

        {#if detail.textbooks.some(t => t.role !== 'required')}
          <section class="textbooks">
            <h3>{t('course.recommendedReading')}</h3>
            {#each detail.textbooks.filter(t => t.role !== 'required') as tb}
              {@render bookCard(tb)}
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

        {#if detail.resources.length > 0}
          <section class="course-resources">
            <h3>{t('course.resources')}</h3>
            {#each detail.resources as r}
              <a href={r.url} target="_blank" rel="noopener" class="resource-link">
                {#if r.kind}<span class="resource-kind">{r.kind}</span>{/if}
                <span class="resource-label">{r.label}</span>
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

{#if showSessionEdit}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => showSessionEdit = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{sessionEditId ? t('course.editSession') : t('course.newSession')}</h3>
      {#if sessionEditError}<p class="error-msg">{sessionEditError}</p>{/if}
      <div class="form-group">
        <label>{t('course.sessionOrder')}</label>
        <input type="number" bind:value={sessionEditOrder} />
      </div>
      <div class="form-group">
        <label>{t('course.topic')}</label>
        <input bind:value={sessionEditTopic} placeholder="Week 0 · Scratch" />
      </div>
      <div class="form-group">
        <label>{t('course.sessionDate')}</label>
        <input bind:value={sessionEditDate} placeholder="2026-09-01" />
      </div>
      <div class="modal-actions">
        <button class="btn btn-secondary" onclick={() => showSessionEdit = false}>Cancel</button>
        <button class="btn btn-primary" onclick={saveSessionEdit} disabled={sessionEditSaving}>
          {sessionEditSaving ? 'Saving...' : 'Save'}
        </button>
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
  .course-siblings { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 8px 0 4px; font-size: 12px; }
  .siblings-label { color: var(--text-hint); }
  .sibling-chip { padding: 2px 9px; border: 1px solid var(--border); border-radius: 3px; color: var(--text-secondary); text-decoration: none; background: var(--bg-white); transition: border-color 0.15s, color 0.15s; }
  .sibling-chip:hover { border-color: var(--accent); color: var(--accent); }
  .course-authors { font-size: 14px; color: var(--text-secondary); margin: 6px 0; }
  .course-authors a { color: var(--text-primary); text-decoration: none; }
  .course-authors a:hover { color: var(--accent); }
  .course-desc { font-size: 14px; color: var(--text-secondary); line-height: 1.6; margin: 12px 0; }
  .course-tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 12px 0; align-items: center; }
  .course-tag { display: inline-flex; align-items: center; gap: 3px; font-size: 12px; padding: 3px 10px; border-radius: 3px; background: rgba(95,155,101,0.1); color: var(--accent); transition: background 0.15s; }
  .course-tag a { color: inherit; text-decoration: none; }
  .course-tag:hover { background: rgba(95,155,101,0.2); }
  .tag-rm { background: none; border: none; cursor: pointer; color: var(--accent); font-size: 14px; line-height: 1; padding: 0 2px; opacity: 0.6; }
  .tag-rm:hover { opacity: 1; color: #c00; }
  .course-tag-add { position: relative; display: inline-flex; gap: 4px; }
  .tag-add-input { padding: 2px 8px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white); color: var(--text-primary); min-width: 140px; }
  .tag-add-btn { padding: 2px 8px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white); color: var(--text-primary); cursor: pointer; }
  .tag-add-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .tag-suggest-list { position: absolute; top: 100%; left: 0; background: var(--bg-white); border: 1px solid var(--border); border-radius: 3px; z-index: 10; min-width: 160px; box-shadow: 0 2px 6px rgba(0,0,0,0.08); }
  .tag-suggest-list button { display: block; width: 100%; text-align: left; padding: 4px 10px; border: none; background: none; color: var(--text-primary); font-size: 13px; cursor: pointer; }
  .tag-suggest-list button:hover { background: var(--bg-hover); }
  .tag-error { font-size: 12px; color: #c00; margin: 4px 0; }
  .source-link { font-size: 12px; color: var(--text-hint); text-decoration: none; word-break: break-all; }
  .source-link:hover { color: var(--accent); }
  .attribution { font-size: 12px; color: var(--text-hint); font-style: italic; margin: 4px 0; }

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
  .session-video { white-space: nowrap; }
  .session-cell-video,
  .session-cell-materials,
  .session-cell-hw,
  .session-cell-discussion,
  .session-cell-misc {
    max-width: 280px; display: flex; flex-wrap: wrap; gap: 4px;
    vertical-align: top;
  }
  .res-chip {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: 11px; padding: 2px 8px; border-radius: 3px;
    text-decoration: none; white-space: nowrap;
    background: var(--bg-hover, #f5f5f5); color: var(--text-primary);
    transition: opacity 0.15s;
    max-width: 100%; overflow: hidden; text-overflow: ellipsis;
  }
  .res-chip:hover { opacity: 0.85; text-decoration: none; }
  .chip-icon { font-size: 11px; flex-shrink: 0; }
  /* Per-kind tinting: video reads as primary action, others stay subtler.
     We don't tint every kind — visual noise scales fast and chip text
     already carries meaning. */
  .res-chip-video { background: rgba(220,38,38,0.10); color: #dc2626; }
  .res-chip-homework { background: rgba(16,185,129,0.10); color: #059669; }
  .res-chip-discussion { background: rgba(168,85,247,0.10); color: #7c3aed; }
  .res-chip-optional { opacity: 0.65; font-style: italic; }
  /* URL-less attachments (e.g. textbook citations) — plain inline text. */
  .res-text {
    display: inline-block;
    font-size: 11px; padding: 2px 0;
    color: var(--text-secondary, #555);
    white-space: nowrap;
  }
  .res-text-optional { opacity: 0.65; font-style: italic; }
  /* Section dividers — thematic header rows that group adjacent lectures.
     Mirror the source schedules (CMU/Cornell) which use a tinted band. */
  .session-section td {
    background: rgba(95,155,101,0.10);
    color: var(--text-primary);
    font-weight: 600;
    font-family: var(--font-serif);
    text-align: center;
    padding: 6px 12px;
    border-top: 1px solid rgba(95,155,101,0.35);
    border-bottom: 1px solid rgba(95,155,101,0.35);
  }
  .session-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
  .session-tag { font-size: 10px; padding: 1px 6px; border-radius: 3px; background: rgba(95,155,101,0.08); color: var(--accent); text-decoration: none; }
  .session-tag:hover { background: rgba(95,155,101,0.18); text-decoration: none; }

  .qa-summary { margin-bottom: 20px; }
  .qa-summary h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; }
  .qa-count { font-size: 12px; color: var(--text-hint); font-weight: 400; }
  .qa-empty { font-size: 12px; color: var(--text-hint); margin: 4px 0 8px; }
  .qa-item { display: flex; flex-direction: column; gap: 3px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 6px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .qa-item:hover { border-color: var(--accent); text-decoration: none; }
  .qa-body { font-size: 12px; color: var(--text-primary); line-height: 1.4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .qa-meta { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--text-hint); }
  .qa-author { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; flex: 1; }
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
  .prereqs, .skill-trees { margin-bottom: 20px; }
  .prereqs h3, .skill-trees h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .prereq-link, .tree-link { display: block; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 6px; text-decoration: none; color: var(--text-primary); font-size: 13px; transition: border-color 0.15s; }
  .prereq-link:hover, .tree-link:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }

  .course-resources { margin-bottom: 20px; }
  .course-resources h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .resource-link { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 5px; text-decoration: none; color: var(--text-primary); font-size: 13px; transition: border-color 0.15s; }
  .resource-link:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }
  .resource-kind { font-size: 10px; text-transform: uppercase; color: var(--text-hint); background: var(--bg-hover, #f5f5f5); padding: 1px 6px; border-radius: 3px; flex-shrink: 0; }
  .resource-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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

  .session-actions { white-space: nowrap; text-align: right; }
  .session-action-btn {
    background: none; border: none; cursor: pointer; font-size: 13px;
    color: var(--text-hint); padding: 2px 6px;
  }
  .session-action-btn:hover { color: var(--accent); }
  .session-actions-col { width: 1%; }
  .session-add-row td { padding: 4px 0; border: none; }
  .session-add-btn {
    background: none; border: 1px dashed var(--border); color: var(--text-hint);
    padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;
    width: 100%;
  }
  .session-add-btn:hover { border-color: var(--accent); color: var(--accent); }

  .modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.5); z-index: 1000;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--bg-white, var(--bg-page, #fff)); border-radius: 8px; padding: 24px;
    width: 90%; max-width: 420px; max-height: 90vh; overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0,0,0,0.3);
  }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); font-weight: 400; }
  .modal .form-group { margin-bottom: 12px; }
  .modal .form-group label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px; }
  .modal input {
    width: 100%; padding: 8px; border: 1px solid var(--border);
    border-radius: 4px; font-size: 14px; background: var(--bg-page, #fff);
    color: var(--text-primary, #333); box-sizing: border-box;
  }
  .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
  .error-msg { color: #c33; font-size: 13px; margin: 0 0 12px; }

  @media (max-width: 768px) {
    .course-header { flex-direction: column; }
    .header-side { width: 100%; }
    .course-body { flex-direction: column; }
    .body-side { width: 100%; }
  }
</style>
