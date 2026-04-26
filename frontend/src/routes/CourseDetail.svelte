<script lang="ts">
  // The unified umbrella-course page. One Course = many Terms (iterations).
  //
  // Layout:
  //   header: course code + title + institution + description; on the
  //           right, a row of pills, one per Term (semester labels).
  //           Clicking a pill swaps `?term=…` and re-renders the body.
  //   body:   metadata for the selected Term (semester / instructors /
  //           source) + its calendar (sessions table). The calendar
  //           rendering is copied verbatim from TermDetail.svelte so
  //           sections, lecture numbering, and attachment columns match.
  //   foot:   course-level discussion thread on contentUri = "course:{id}".
  //           Discussion is shared across all iterations and does NOT
  //           re-render when the term switches.
  //
  // If `?term` is missing we default to the latest iteration (terms come
  // back sorted semester-DESC). Term details are lazy-loaded on demand
  // and cached per session so flipping between pills feels instant after
  // the first hit.
  import {
    getCourseDetail, deleteCourse, getTermDetail,
    rateTerm, unrateTerm, setTermLearningStatus, removeTermLearningStatus,
    setSessionProgress, createTermSession, updateTermSession, deleteTermSession,
  } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import type { CourseDetail, TermDetail } from '../lib/types';

  let { id, term: termProp = '' }: { id: string; term?: string } = $props();

  let detail = $state<CourseDetail | null>(null);
  let loading = $state(true);
  let error = $state('');

  // Per-term lazy load + cache. Switching pills hits the cache after
  // the first load, so flipping iterations is instantaneous.
  let termCache = $state<Map<string, TermDetail>>(new Map());
  let activeTermId = $state('');
  let termLoading = $state(false);
  let termError = $state('');

  // Per-term interactive state (rating, learning status, session checks).
  // Reset whenever `activeTermId` changes.
  let avgRating = $state(0);
  let ratingCount = $state(0);
  let myRating = $state(0);
  let hoverRating = $state(0);
  let learningStatus = $state('');
  let sessionDone = $state(new Map<string, boolean>());

  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const locale = getLocale();
    return field[locale] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  $effect(() => { if (id) loadCourse(); });

  async function loadCourse() {
    loading = true;
    error = '';
    try {
      detail = await getCourseDetail(id);
      document.title = `${detail.course.title} — NightBoat`;
      // Pick term from prop, falling back to the latest iteration. The
      // server orders terms semester-DESC so terms[0] is the newest.
      const wanted = termProp && detail.terms.some(t => t.id === termProp)
        ? termProp
        : (detail.terms[0]?.id ?? '');
      if (wanted) await selectTerm(wanted, /*push*/ false);
    } catch (e: any) {
      error = e?.message || 'Error';
    } finally {
      loading = false;
    }
  }

  // React to ?term= changes (back/forward only — pill clicks update
  // activeTermId directly via selectTerm). The fallback to terms[0] runs
  // ONCE in loadCourse(); re-evaluating it here would snap activeTermId
  // back to terms[0] every time the user clicked another pill, since
  // termProp doesn't change synchronously with selectTerm.
  $effect(() => {
    if (!detail) return;
    if (termProp && termProp !== activeTermId
        && detail.terms.some(t => t.id === termProp)) {
      selectTerm(termProp, /*push*/ false);
    }
  });

  async function selectTerm(termId: string, push: boolean) {
    activeTermId = termId;
    if (push) {
      const url = `/course?id=${encodeURIComponent(id)}&term=${encodeURIComponent(termId)}`;
      history.pushState(null, '', url);
      // Notify App.svelte's router so currentPath updates and termProp
      // reflects the new URL on the next render.
      window.dispatchEvent(new PopStateEvent('popstate'));
    }
    if (termCache.has(termId)) {
      hydrateInteractive(termCache.get(termId)!);
      return;
    }
    termLoading = true;
    termError = '';
    try {
      const td = await getTermDetail(termId);
      termCache.set(termId, td);
      termCache = new Map(termCache);
      hydrateInteractive(td);
    } catch (e: any) {
      termError = e?.message || 'Failed to load iteration';
    } finally {
      termLoading = false;
    }
  }

  function hydrateInteractive(td: TermDetail) {
    avgRating = td.rating.avg_rating;
    ratingCount = td.rating.rating_count;
    myRating = td.my_rating ?? 0;
    learningStatus = td.my_learning_status?.status ?? '';
    sessionDone = new Map(td.my_session_progress.map(p => [p.session_id, p.completed]));
  }

  let activeTerm = $derived<TermDetail | null>(
    activeTermId ? termCache.get(activeTermId) ?? null : null,
  );

  // ── Calendar column model — copied from TermDetail.svelte verbatim.
  // Each attachment kind owns its own column; columns appear only when at
  // least one session has an entry of that kind. `code`/`other` collapse
  // into a single "misc" column. Order matches reading flow: video →
  // notes → … → homework → discussion → odds and ends.
  const KIND_COLUMNS: { id: string; kinds: string[]; labelKey: string }[] = [
    { id: 'video',      kinds: ['video'],      labelKey: 'term.video' },
    { id: 'notes',      kinds: ['notes'],      labelKey: 'term.notes' },
    { id: 'slides',     kinds: ['slides'],     labelKey: 'term.slides' },
    { id: 'recitation', kinds: ['recitation'], labelKey: 'term.recitation' },
    { id: 'reading',    kinds: ['reading'],    labelKey: 'term.reading' },
    { id: 'outline',    kinds: ['outline'],    labelKey: 'term.outline' },
    { id: 'summary',    kinds: ['summary'],    labelKey: 'term.summary' },
    { id: 'homework',   kinds: ['homework'],   labelKey: 'term.hw' },
    { id: 'discussion', kinds: ['discussion'], labelKey: 'term.discussion' },
    { id: 'misc',       kinds: ['code', 'other'], labelKey: 'term.misc' },
  ];
  function groupForKind(kind: string): string {
    const col = KIND_COLUMNS.find(c => c.kinds.includes(kind));
    return col?.id ?? 'misc';
  }
  function groupAttachments(s: import('../lib/types').TermSession, group: string) {
    return (s.attachments ?? []).filter(a => groupForKind(a.kind) === group);
  }
  let visibleColumns = $derived(
    KIND_COLUMNS.filter(col =>
      activeTerm?.sessions.some(s => groupAttachments(s, col.id).length > 0) ?? false,
    ),
  );
  let isTermOwner = $derived(!!activeTerm && getAuth()?.did === activeTerm.term.did);
  let colCount = $derived(2 + visibleColumns.length + (isTermOwner ? 1 : 0));

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

  // Rating / status / progress handlers — same shape as TermDetail.
  async function submitRating(value: number) {
    if (!activeTermId) return;
    myRating = value;
    const stats = await rateTerm(activeTermId, value);
    avgRating = stats.avg_rating;
    ratingCount = stats.rating_count;
  }
  async function clearRating() {
    if (!activeTermId) return;
    const stats = await unrateTerm(activeTermId);
    myRating = 0;
    avgRating = stats.avg_rating;
    ratingCount = stats.rating_count;
  }
  async function setStatus(status: 'want_to_learn' | 'learning' | 'finished' | 'dropped') {
    if (!activeTermId) return;
    if (learningStatus === status) {
      learningStatus = '';
      try { await removeTermLearningStatus(activeTermId); } catch { /* */ }
    } else {
      try {
        const row = await setTermLearningStatus(activeTermId, status);
        learningStatus = row.status;
      } catch { /* */ }
    }
  }
  async function toggleSession(sessionId: string) {
    if (!activeTermId) return;
    const next = !sessionDone.get(sessionId);
    sessionDone.set(sessionId, next);
    sessionDone = new Map(sessionDone);
    try {
      const row = await setSessionProgress(activeTermId, sessionId, next);
      if (row) learningStatus = row.status;
    } catch {
      sessionDone.set(sessionId, !next);
      sessionDone = new Map(sessionDone);
    }
  }
  let totalSessions = $derived(activeTerm?.sessions?.length ?? 0);
  let doneSessions = $derived(Array.from(sessionDone.values()).filter(v => v).length);
  function formatRating(r: number) { return r.toFixed(1); }

  // Session CRUD modal state — owner-only path, same as TermDetail.
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
    sessionEditOrder = (activeTerm?.sessions.length ?? 0) + 1;
    sessionEditError = '';
    showSessionEdit = true;
  }
  function openSessionEdit(s: import('../lib/types').TermSession) {
    sessionEditId = s.id;
    sessionEditTopic = s.topic || '';
    sessionEditDate = s.date || '';
    sessionEditOrder = s.sort_order;
    sessionEditError = '';
    showSessionEdit = true;
  }
  async function saveSessionEdit() {
    if (!activeTermId) return;
    sessionEditSaving = true;
    sessionEditError = '';
    try {
      const payload = {
        topic: sessionEditTopic.trim() || undefined,
        date: sessionEditDate.trim() || undefined,
        sort_order: typeof sessionEditOrder === 'number' ? sessionEditOrder : undefined,
      };
      if (sessionEditId) {
        await updateTermSession(activeTermId, sessionEditId, payload);
      } else {
        await createTermSession(activeTermId, payload);
      }
      showSessionEdit = false;
      // Refresh just this term in cache.
      const fresh = await getTermDetail(activeTermId);
      termCache.set(activeTermId, fresh);
      termCache = new Map(termCache);
      hydrateInteractive(fresh);
    } catch (e: any) {
      sessionEditError = e.message || 'Save failed';
    } finally {
      sessionEditSaving = false;
    }
  }
  async function confirmDeleteSession(sessionId: string) {
    if (!activeTermId) return;
    if (!confirm(t('term.deleteSessionConfirm'))) return;
    try {
      await deleteTermSession(activeTermId, sessionId);
      const fresh = await getTermDetail(activeTermId);
      termCache.set(activeTermId, fresh);
      termCache = new Map(termCache);
      hydrateInteractive(fresh);
    } catch (e: any) {
      alert(e.message || 'Delete failed');
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

  // Pill click — pushes URL; popstate re-fires the prop watcher and
  // selectTerm runs from there. We could call selectTerm directly but
  // routing via the URL keeps back/forward honest.
  function pickTerm(termId: string) {
    selectTerm(termId, /*push*/ true);
  }
</script>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  <article class="course-page">
    <header class="course-header">
      <div class="header-main">
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
      </div>
    </header>

    <!-- Course-level textbooks — apply across every iteration. -->
    {#if detail.textbooks.length > 0}
      <section class="course-textbooks">
        <h3 class="section-heading">{t('courses.textbooks')}</h3>
        <div class="textbook-carousel">
          {#each detail.textbooks as tb}
            <a href="/book?id={encodeURIComponent(tb.book_id)}" class="book-card book-card-large">
              {#if tb.cover_url}
                <img src={tb.cover_url} alt="" class="book-card-cover" />
              {:else}
                <div class="book-card-cover book-card-cover-placeholder" aria-hidden="true"></div>
              {/if}
              <div class="book-card-info">
                <span class="book-card-title">{loc(tb.title)}</span>
                <span class="book-card-authors">{tb.authors.join(', ')}</span>
                {#if tb.role !== 'required'}
                  <span class="book-card-role">{tb.role}</span>
                {/if}
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if detail.terms.length > 0}
      <div class="term-switcher-bar" aria-label={t('courses.iterations')}>
        <span class="switcher-label">{t('courses.iterations')}</span>
        <div class="pill-row">
          {#each detail.terms as term, i (term.id)}
            <button
              class="term-pill"
              class:active={term.id === activeTermId}
              onclick={() => pickTerm(term.id)}
              title={term.title}
            >
              {term.semester || term.title}
            </button>
          {/each}
        </div>
        {#if detail.terms.length > 1}
          {@const idx = detail.terms.findIndex(t => t.id === activeTermId)}
          {#if idx >= 0}
            <span class="switcher-hint">{idx + 1} / {detail.terms.length}</span>
          {/if}
        {/if}
      </div>
    {/if}

    <!-- Per-term body (metadata + calendar) -->
    {#if detail.terms.length === 0}
      <p class="empty">{t('courses.noTerms')}</p>
    {:else if termLoading && !activeTerm}
      <p class="meta">{t('common.loading')}</p>
    {:else if termError}
      <p class="error">{termError}</p>
    {:else if activeTerm}
      {@const td = activeTerm}
      <section class="term-block">
        <div class="term-meta-callout">
          <div class="meta-left">
            {#if td.term.semester}
              <span class="meta-chip meta-semester">{td.term.semester}</span>
            {/if}
          </div>
          <div class="meta-center">
            {#if td.authors.length > 0}
              <span class="meta-instructor">
                {#each td.authors as a, i}
                  {#if a.did}
                    <a href="/profile?did={encodeURIComponent(a.did)}">{a.name}</a>
                  {:else}
                    <a href="/author?id={encodeURIComponent(a.id)}">{a.name}</a>
                  {/if}{#if i < td.authors.length - 1}, {/if}
                {/each}
              </span>
            {/if}
          </div>
          <div class="meta-right">
            {#if td.term.source_url}
              <a href={td.term.source_url} target="_blank" rel="noopener" class="meta-source">
                {t('term.source')} <span class="ext-arrow" aria-hidden="true">↗</span>
              </a>
            {/if}
            {#if isTermOwner}
              <a href="/new-term?edit={encodeURIComponent(td.term.id)}" class="edit-btn">{t('common.edit')}</a>
            {/if}
          </div>
        </div>

        <!-- Rating + status row, scoped to the selected iteration. -->
        <div class="rating-row">
          <span class="rating-stars-display">
            {#each [1,2,3,4,5] as star}
              {@const val = avgRating / 2}
              {@const filled = val >= star}
              {@const half = !filled && val >= star - 0.5}
              <svg class="star-svg" viewBox="0 0 24 24" width="20" height="20">
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
          {#if getAuth()}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span class="my-rating" onmouseleave={() => { hoverRating = 0; }}>
              <span class="my-rating-label">{t('term.myRating')}:</span>
              <span class="star-picker">
                {#each [1,2,3,4,5] as star}
                  {@const activeVal = hoverRating || myRating}
                  {@const leftActive = activeVal >= star * 2 - 1}
                  {@const rightActive = activeVal >= star * 2}
                  <svg class="star-svg" viewBox="0 0 24 24" width="18" height="18">
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
                <button class="clear-rating" onclick={clearRating} title={t('term.clearRating')}>×</button>
              {/if}
            </span>
            <span class="status-actions">
              <button class="status-btn" class:active={learningStatus === 'want_to_learn'} onclick={() => setStatus('want_to_learn')}>{t('term.status.wantToLearn')}</button>
              <button class="status-btn" class:active={learningStatus === 'learning'} onclick={() => setStatus('learning')}>{t('term.status.learning')}</button>
              <button class="status-btn" class:active={learningStatus === 'finished'} onclick={() => setStatus('finished')}>{t('term.status.finished')}</button>
              <button class="status-btn" class:active={learningStatus === 'dropped'} onclick={() => setStatus('dropped')}>{t('term.status.dropped')}</button>
            </span>
          {/if}
        </div>

        <div class="term-body">
          <div class="body-full">
            {#if td.sessions.length > 0 || isTermOwner}
              <section class="schedule">
                <h2 class="schedule-heading">{t('term.calendar')}</h2>
                <div class="schedule-table-wrap">
                <table class="schedule-table">
                  <thead>
                    <tr>
                      <th class="th-num">#</th>
                      <th class="th-topic">{t('term.topic')}</th>
                      {#each visibleColumns as col}
                        <th class="th-col th-col-{col.id}">{t(col.labelKey)}</th>
                      {/each}
                      {#if isTermOwner}<th class="session-actions-col"></th>{/if}
                    </tr>
                  </thead>
                  <tbody>
                    {#each td.sessions as s}
                      {@const atts = s.attachments ?? []}
                      {@const isSection = s.kind === 'section'}
                      {@const isExam = !isSection && atts.length === 0}
                      {@const lectureIdx = td.sessions
                        .slice(0, td.sessions.indexOf(s))
                        .filter(p => p.kind !== 'section').length + 1}
                      {#if isSection}
                        <tr class="session-section">
                          <td colspan={colCount}>
                            <span class="section-icon" aria-hidden="true">§</span>
                            <span class="section-label">{s.topic || ''}</span>
                            <span class="section-icon" aria-hidden="true">§</span>
                          </td>
                        </tr>
                      {:else}
                      <tr class:session-exam={isExam} class="session-row">
                        <td class="session-num">
                          <span class="session-num-text">{lectureIdx}</span>
                          {#if getAuth() && !isExam}
                            <button
                              class="session-check"
                              class:done={sessionDone.get(s.id)}
                              onclick={() => toggleSession(s.id)}
                              title={sessionDone.get(s.id) ? t('term.markUndone') : t('term.markDone')}
                              aria-label={sessionDone.get(s.id) ? t('term.markUndone') : t('term.markDone')}
                            ></button>
                          {/if}
                        </td>
                        {#if isExam}
                          <td class="session-topic session-topic-exam" colspan={colCount - 1 - (isTermOwner ? 1 : 0)}>
                            <strong>{s.topic || ''}</strong>
                          </td>
                        {:else}
                          <td class="session-topic">
                            <span class="session-topic-text">{s.topic || ''}</span>
                            {#if s.tags && s.tags.length > 0}
                              <div class="session-tags">
                                {#each s.tags as tag}
                                  <a href="/tag?id={encodeURIComponent(tag.tag_id)}" class="session-tag">{tag.tag_name}</a>
                                {/each}
                              </div>
                            {/if}
                          </td>
                          {#each visibleColumns as col}
                            <td class="session-cell session-cell-{col.id}">
                              <div class="cell-chips">
                                {#each groupAttachments(s, col.id).filter(a => a.required) as a}
                                  {#if a.url}
                                    <a href={a.url} target="_blank" rel="noopener"
                                       class="res-chip res-chip-{a.kind}" title={a.label}>
                                      <span class="chip-icon" aria-hidden="true">{attachIcon(a.kind)}</span><span class="chip-label">{a.label}</span>
                                    </a>
                                  {:else}
                                    <span class="res-text" title={a.label}>{a.label}</span>
                                  {/if}
                                {/each}
                                {#each groupAttachments(s, col.id).filter(a => !a.required) as a}
                                  {#if a.url}
                                    <a href={a.url} target="_blank" rel="noopener"
                                       class="res-chip res-chip-{a.kind} res-chip-optional" title={a.label}>
                                      <span class="chip-icon" aria-hidden="true">{attachIcon(a.kind)}</span><span class="chip-label">{a.label}</span>
                                    </a>
                                  {:else}
                                    <span class="res-text res-text-optional" title={a.label}>{a.label}</span>
                                  {/if}
                                {/each}
                              </div>
                            </td>
                          {/each}
                        {/if}
                        {#if isTermOwner}
                          <td class="session-actions">
                            <button class="session-action-btn" onclick={() => openSessionEdit(s)} title={t('term.editSession')}>✎</button>
                            <button class="session-action-btn" onclick={() => confirmDeleteSession(s.id)} title="Delete">🗑</button>
                          </td>
                        {/if}
                      </tr>
                      {/if}
                    {/each}
                    {#if isTermOwner}
                      <tr class="session-add-row">
                        <td colspan={colCount}>
                          <button class="session-add-btn" onclick={openSessionAdd}>{t('term.addSession')}</button>
                        </td>
                      </tr>
                    {/if}
                  </tbody>
                </table>
                </div>
              </section>
            {/if}
          </div>

        </div>

        <!-- Term-level materials (textbooks, extra resources). Below the
             calendar so the schedule gets full page width. -->
        {#snippet bookCard(tb: NonNullable<typeof td>['textbooks'][number])}
          <a href="/book?id={encodeURIComponent(tb.book_id)}" class="book-card book-card-compact">
            {#if tb.cover_url}
              <img src={tb.cover_url} alt="" class="book-card-cover" />
            {:else}
              <div class="book-card-cover book-card-cover-placeholder" aria-hidden="true"></div>
            {/if}
            <div class="book-card-info">
              <span class="book-card-title">{loc(tb.title)}</span>
              <span class="book-card-authors">{tb.authors.join(', ')}</span>
            </div>
          </a>
        {/snippet}

        {#if td.textbooks.length > 0 || td.resources.length > 0}
          <div class="term-materials">
            {#if td.textbooks.some(t => t.role === 'required')}
              <section class="textbooks">
                <h3 class="section-heading section-heading-sm">{t('term.textbooks')}</h3>
                <div class="textbook-grid">
                  {#each td.textbooks.filter(t => t.role === 'required') as tb}
                    {@render bookCard(tb)}
                  {/each}
                </div>
              </section>
            {/if}
            {#if td.textbooks.some(t => t.role !== 'required')}
              <section class="textbooks">
                <h3 class="section-heading section-heading-sm">{t('term.recommendedReading')}</h3>
                <div class="textbook-grid">
                  {#each td.textbooks.filter(t => t.role !== 'required') as tb}
                    {@render bookCard(tb)}
                  {/each}
                </div>
              </section>
            {/if}
            {#if td.resources.length > 0}
              <section class="term-resources">
                <h3 class="section-heading section-heading-sm">{t('term.resources')}</h3>
                <div class="resource-list">
                  {#each td.resources as r}
                    <a href={r.url} target="_blank" rel="noopener" class="resource-link">
                      <span class="resource-icon" aria-hidden="true">{attachIcon(r.kind || 'other')}</span>
                      {#if r.kind}<span class="resource-kind">{r.kind}</span>{/if}
                      <span class="resource-label">{r.label}</span>
                      <span class="resource-arrow" aria-hidden="true">↗</span>
                    </a>
                  {/each}
                </div>
              </section>
            {/if}
          </div>
        {/if}
      </section>
    {/if}

    <!-- Course-level discussion / reviews / notes — anchored to the
         course, not the iteration, so they survive term switches. -->
    <nav class="course-tabs" aria-label="Course discussions">
      <div class="tab-strip">
        <a class="tab-link" href="/course-discussions?id={encodeURIComponent(detail.course.id)}">
          <span class="tab-label">{t('course.discussion')}</span>
          <span class="tab-count">{detail.discussion_count}</span>
        </a>
        <a class="tab-link" href="/course-reviews?id={encodeURIComponent(detail.course.id)}">
          <span class="tab-label">{t('course.reviews')}</span>
        </a>
        <a class="tab-link" href="/course-notes?id={encodeURIComponent(detail.course.id)}">
          <span class="tab-label">{t('course.notes')}</span>
        </a>
      </div>
      {#if getAuth()}
        <div class="tab-actions">
          <a class="tab-action" href="/new?category=review&course_id={encodeURIComponent(detail.course.id)}">{t('course.writeReview')}</a>
          <a class="tab-action" href="/new?category=note&course_id={encodeURIComponent(detail.course.id)}">{t('course.writeNote')}</a>
        </div>
      {/if}
    </nav>

    <section class="discussion-card">
      <header class="discussion-header">
        <h2 class="discussion-heading">{t('course.discussion')}</h2>
        <span class="count-badge">{detail.discussion_count}</span>
      </header>
      <div class="discussion-body">
        <CommentThread contentUri={`course:${detail.course.id}`} />
      </div>
    </section>
  </article>
{/if}

{#if showSessionEdit}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => showSessionEdit = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{sessionEditId ? t('term.editSession') : t('term.newSession')}</h3>
      {#if sessionEditError}<p class="error-msg">{sessionEditError}</p>{/if}
      <div class="form-group">
        <label for="session-order">{t('term.sessionOrder')}</label>
        <input id="session-order" type="number" bind:value={sessionEditOrder} />
      </div>
      <div class="form-group">
        <label for="session-topic">{t('term.topic')}</label>
        <input id="session-topic" bind:value={sessionEditTopic} placeholder="Week 0 · Scratch" />
      </div>
      <div class="form-group">
        <label for="session-date">{t('term.sessionDate')}</label>
        <input id="session-date" bind:value={sessionEditDate} placeholder="2026-09-01" />
      </div>
      <div class="modal-actions">
        <button class="btn btn-secondary" onclick={() => showSessionEdit = false}>{t('common.cancel')}</button>
        <button class="btn btn-primary" onclick={saveSessionEdit} disabled={sessionEditSaving}>
          {sessionEditSaving ? t('common.saving') : t('common.save')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ─────────────────────────────────────────────────────────────
     Layout shell
     ───────────────────────────────────────────────────────────── */
  .course-page { max-width: 100%; padding: 8px 0 48px; }

  /* ─────────────────────────────────────────────────────────────
     Header — stamped code chip + serif title + readable description
     ───────────────────────────────────────────────────────────── */
  .course-header {
    margin-bottom: 48px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border);
  }
  .header-main { min-width: 0; }
  .course-code {
    display: inline-block;
    font-size: 11px;
    letter-spacing: 0.08em;
    padding: 4px 10px;
    background: rgba(95, 155, 101, 0.12);
    color: var(--accent);
    border-radius: 4px;
    border: 1px solid rgba(95, 155, 101, 0.18);
    font-family: var(--font-mono, ui-monospace, "SF Mono", Menlo, monospace);
    font-weight: 600;
    text-transform: uppercase;
    box-shadow: 0 1px 2px rgba(95, 155, 101, 0.08);
    margin-bottom: 14px;
  }
  .course-title {
    font-family: var(--font-serif);
    font-weight: 500;
    margin: 0 0 6px;
    font-size: 2.1rem;
    line-height: 1.15;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }
  .course-institution {
    color: var(--text-secondary);
    margin: 0 0 16px;
    font-size: 14px;
    font-weight: 500;
  }
  .course-desc {
    color: var(--text-secondary);
    line-height: 1.65;
    margin: 0;
    max-width: 70ch;
    font-size: 15px;
  }
  .course-actions { margin-top: 20px; }
  .danger {
    background: transparent; border: 1px solid #c00; color: #c00;
    padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;
    transition: background 0.15s, color 0.15s;
  }
  .danger:hover { background: #c00; color: white; }

  /* ─────────────────────────────────────────────────────────────
     Term switcher — sticky horizontal pill bar
     ───────────────────────────────────────────────────────────── */
  .term-switcher-bar {
    position: sticky;
    top: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    margin: 0 -14px 32px;
    background: color-mix(in srgb, var(--bg-page) 92%, transparent);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border-bottom: 1px solid var(--border);
  }
  .switcher-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-hint);
    font-weight: 600;
    flex-shrink: 0;
  }
  .pill-row {
    display: flex;
    flex-wrap: nowrap;
    gap: 6px;
    overflow-x: auto;
    flex: 1;
    scrollbar-width: thin;
  }
  .pill-row::-webkit-scrollbar { height: 4px; }
  .pill-row::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }
  .switcher-hint {
    font-size: 11px;
    color: var(--text-hint);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }
  .term-pill {
    padding: 5px 13px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg-white);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.15s, background 0.15s, color 0.15s, box-shadow 0.15s, transform 0.05s;
  }
  .term-pill:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: rgba(95, 155, 101, 0.04);
  }
  .term-pill:active { transform: translateY(1px); }
  .term-pill.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    box-shadow: 0 1px 3px rgba(95, 155, 101, 0.35);
  }

  /* ─────────────────────────────────────────────────────────────
     Section headings (shared)
     ───────────────────────────────────────────────────────────── */
  .section-heading {
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 1.15rem;
    margin: 0 0 14px;
    color: var(--text-primary);
    letter-spacing: -0.005em;
  }
  .section-heading-sm {
    font-size: 0.95rem;
    margin: 0 0 10px;
    color: var(--text-secondary);
    text-transform: none;
    letter-spacing: 0.01em;
  }

  /* ─────────────────────────────────────────────────────────────
     Course-level textbooks — horizontal carousel of book cards
     ───────────────────────────────────────────────────────────── */
  .course-textbooks { margin-bottom: 48px; }
  .textbook-carousel {
    display: flex;
    gap: 14px;
    overflow-x: auto;
    padding: 4px 2px 12px;
    scrollbar-width: thin;
  }
  .textbook-carousel::-webkit-scrollbar { height: 6px; }
  .textbook-carousel::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
  .textbook-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 10px;
  }

  /* Book card — large (course-level) and compact (term-level) variants */
  .book-card {
    display: flex;
    text-decoration: none;
    color: inherit;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
  }
  .book-card:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.06);
    transform: translateY(-2px);
    text-decoration: none;
  }
  .book-card-large {
    flex-direction: column;
    width: 168px;
    flex-shrink: 0;
    padding: 12px;
    gap: 10px;
  }
  .book-card-large .book-card-cover {
    width: 100%;
    height: 200px;
    border-radius: 4px;
  }
  .book-card-compact {
    flex-direction: row;
    gap: 10px;
    padding: 8px;
    align-items: stretch;
  }
  .book-card-compact .book-card-cover {
    width: 42px;
    height: 58px;
    flex-shrink: 0;
    border-radius: 3px;
  }
  .book-card-cover {
    object-fit: cover;
    background: var(--bg-page);
    border: 1px solid var(--border);
  }
  .book-card-cover-placeholder {
    background: linear-gradient(135deg, rgba(95,155,101,0.06), rgba(95,155,101,0.02));
  }
  .book-card-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }
  .book-card-title {
    font-family: var(--font-serif);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .book-card-authors {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.35;
  }
  .book-card-role {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-hint);
    margin-top: 4px;
  }

  /* ─────────────────────────────────────────────────────────────
     Selected-term block
     ───────────────────────────────────────────────────────────── */
  .term-block { margin-bottom: 48px; }

  /* "You're viewing" callout */
  .term-meta-callout {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    background: rgba(95, 155, 101, 0.05);
    border: 1px solid rgba(95, 155, 101, 0.15);
    border-radius: 10px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .meta-left { flex-shrink: 0; }
  .meta-center {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .meta-right {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .meta-chip {
    padding: 4px 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .meta-semester {
    background: var(--accent);
    color: white;
  }
  .meta-instructor a {
    color: var(--text-primary);
    text-decoration: none;
    font-weight: 500;
    border-bottom: 1px dotted transparent;
    transition: border-color 0.15s, color 0.15s;
  }
  .meta-instructor a:hover {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .meta-source {
    color: var(--accent);
    text-decoration: none;
    font-size: 13px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .meta-source:hover { text-decoration: underline; }
  .ext-arrow { font-size: 11px; }
  .edit-btn {
    font-size: 12px;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    text-decoration: none;
    background: var(--bg-white);
    transition: border-color 0.15s, color 0.15s;
  }
  .edit-btn:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }

  /* Rating + status row */
  .rating-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 24px;
    padding: 0 4px;
    flex-wrap: wrap;
  }
  .rating-stars-display { display: inline-flex; gap: 1px; align-items: center; }
  .rating-value { font-size: 14px; font-weight: 600; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .rating-count { font-size: 12px; color: var(--text-hint); font-variant-numeric: tabular-nums; }
  .my-rating {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding-left: 12px;
    border-left: 1px solid var(--border);
  }
  .my-rating-label { font-size: 12px; color: var(--text-hint); }
  .star-picker { display: inline-flex; gap: 1px; cursor: pointer; align-items: center; }
  .star-svg { display: block; vertical-align: middle; }
  .my-rating-value { font-size: 12px; color: #f59e0b; font-weight: 600; font-variant-numeric: tabular-nums; }
  .clear-rating { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 4px; line-height: 1; }
  .clear-rating:hover { color: #c00; }
  .status-actions {
    display: inline-flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-left: auto;
  }
  .status-btn {
    padding: 5px 12px;
    border: 1px solid var(--border);
    background: var(--bg-white);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .status-btn:hover { border-color: var(--accent); color: var(--accent); }
  .status-btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    box-shadow: 0 1px 3px rgba(95, 155, 101, 0.35);
  }

  /* ─────────────────────────────────────────────────────────────
     Schedule table
     ───────────────────────────────────────────────────────────── */
  .term-body { display: block; }
  .body-full { width: 100%; }
  .schedule { margin-bottom: 32px; }
  .schedule-heading {
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 1.25rem;
    margin: 0 0 16px;
    color: var(--text-primary);
  }
  .schedule-table-wrap {
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-white);
  }
  .schedule-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
    /* Critical: ensure cells participate in table-row layout. */
    table-layout: auto;
  }
  .schedule-table th {
    text-align: left;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-hint);
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-page);
    vertical-align: middle;
  }
  .th-num { width: 56px; }
  .th-topic { width: 32%; min-width: 200px; }
  .schedule-table td {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
    /* Explicit display:table-cell guards against accidental override
       by a more-specific rule and keeps the cell in its row. */
    display: table-cell;
  }
  .schedule-table tbody tr:last-child td { border-bottom: none; }
  .schedule-table tbody tr.session-row:hover {
    background: rgba(95, 155, 101, 0.04);
  }
  .session-exam {
    background: rgba(220, 38, 38, 0.03);
  }
  .session-exam:hover {
    background: rgba(220, 38, 38, 0.06) !important;
  }

  /* Lecture # column — bigger, monospace, accent color. */
  .session-num {
    width: 56px;
    white-space: nowrap;
  }
  .session-num-text {
    font-family: var(--font-mono, ui-monospace, "SF Mono", Menlo, monospace);
    font-weight: 600;
    font-size: 14px;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  /* Topic column — serif title, sans-serif tags below. */
  .session-topic {
    color: var(--text-primary);
  }
  .session-topic-text {
    font-family: var(--font-serif);
    font-size: 14px;
    font-weight: 500;
    line-height: 1.4;
    color: var(--text-primary);
  }
  .session-topic-exam strong {
    font-family: var(--font-serif);
    font-weight: 600;
    color: #c53030;
    font-size: 14px;
  }

  /* Attachment cells — chips wrap inside an inner div, NOT on the td.
     Putting display:flex on a <td> ejects it from the table-row layout
     (it stops being a table-cell), which causes the "row break" bug. */
  .session-cell { vertical-align: top; }
  .cell-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: flex-start;
  }

  /* Resource chips — consistent height & padding across kinds. */
  .res-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    font-size: 11px;
    font-weight: 500;
    padding: 0 9px;
    border-radius: 6px;
    text-decoration: none;
    white-space: nowrap;
    background: var(--bg-hover);
    color: var(--text-primary);
    transition: opacity 0.15s, transform 0.05s, box-shadow 0.15s;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    border: 1px solid transparent;
  }
  .res-chip:hover {
    text-decoration: none;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }
  .res-chip:active { transform: translateY(1px); }
  .chip-icon {
    font-size: 10px;
    line-height: 1;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
  }
  .chip-label { line-height: 1; overflow: hidden; text-overflow: ellipsis; }

  /* Per-kind tinting */
  .res-chip-video { background: rgba(220, 38, 38, 0.10); color: #dc2626; }
  .res-chip-notes { background: rgba(59, 130, 246, 0.10); color: #2563eb; }
  .res-chip-slides { background: rgba(245, 158, 11, 0.10); color: #b45309; }
  .res-chip-recitation { background: rgba(20, 184, 166, 0.10); color: #0f766e; }
  .res-chip-reading { background: rgba(139, 92, 246, 0.10); color: #6d28d9; }
  .res-chip-outline { background: rgba(100, 116, 139, 0.10); color: #475569; }
  .res-chip-summary { background: rgba(34, 197, 94, 0.10); color: #15803d; }
  .res-chip-homework { background: rgba(16, 185, 129, 0.10); color: #047857; }
  .res-chip-discussion { background: rgba(168, 85, 247, 0.10); color: #7c3aed; }
  .res-chip-code { background: rgba(75, 85, 99, 0.10); color: #374151; }
  .res-chip-other { background: var(--bg-hover); color: var(--text-secondary); }

  /* Optional: dimmer + italic */
  .res-chip-optional {
    opacity: 0.6;
    font-style: italic;
  }
  .res-chip-optional:hover { opacity: 0.85; }

  /* URL-less reading chips — visually distinct from clickable chips. */
  .res-text {
    display: inline-flex;
    align-items: center;
    height: 22px;
    font-size: 11px;
    padding: 0 4px;
    color: var(--text-secondary);
    white-space: nowrap;
    border-bottom: 1px dotted var(--text-hint);
  }
  .res-text-optional {
    opacity: 0.65;
    font-style: italic;
  }

  /* Section divider rows — pronounced colored band. */
  .session-section td {
    background: linear-gradient(
      to right,
      rgba(95, 155, 101, 0.04),
      rgba(95, 155, 101, 0.14),
      rgba(95, 155, 101, 0.04)
    );
    color: var(--text-primary);
    font-weight: 600;
    font-family: var(--font-serif);
    text-align: center;
    padding: 12px 16px;
    border-top: 1px solid rgba(95, 155, 101, 0.30);
    border-bottom: 1px solid rgba(95, 155, 101, 0.30);
    letter-spacing: 0.02em;
  }
  .section-icon {
    color: var(--accent);
    margin: 0 12px;
    font-weight: 400;
    opacity: 0.7;
  }
  .section-label {
    font-size: 14px;
  }

  /* Session tags */
  .session-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
  .session-tag {
    font-family: var(--font-sans);
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(95, 155, 101, 0.08);
    color: var(--accent);
    text-decoration: none;
    transition: background 0.15s;
  }
  .session-tag:hover { background: rgba(95, 155, 101, 0.18); text-decoration: none; }

  /* Session checkbox */
  .session-check {
    width: 14px;
    height: 14px;
    border: 1.5px solid var(--border-strong);
    border-radius: 3px;
    background: transparent;
    cursor: pointer;
    margin-left: 8px;
    padding: 0;
    vertical-align: middle;
    position: relative;
    transition: border-color 0.15s, background 0.15s;
  }
  .session-check:hover { border-color: var(--accent); }
  .session-check.done { background: var(--accent); border-color: var(--accent); }
  .session-check.done::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 0px;
    width: 4px;
    height: 8px;
    border: solid white;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  .session-actions { white-space: nowrap; text-align: right; }
  .session-action-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-hint);
    padding: 2px 6px;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }
  .session-action-btn:hover {
    color: var(--accent);
    background: rgba(95, 155, 101, 0.08);
  }
  .session-actions-col { width: 1%; }

  .session-add-row td {
    padding: 8px;
    border: none;
    background: transparent;
  }
  .session-add-row:hover { background: transparent !important; }
  .session-add-btn {
    background: none;
    border: 1px dashed var(--border-strong);
    color: var(--text-hint);
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    width: 100%;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }
  .session-add-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: rgba(95, 155, 101, 0.04);
  }

  /* ─────────────────────────────────────────────────────────────
     Term-level materials (textbooks + resources)
     ───────────────────────────────────────────────────────────── */
  .term-materials {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 24px;
    margin: 32px 0 0;
  }
  .textbooks, .term-resources { margin: 0; }

  .resource-list { display: flex; flex-direction: column; gap: 6px; }
  .resource-link {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-white);
    text-decoration: none;
    color: var(--text-primary);
    font-size: 13px;
    transition: border-color 0.15s, background 0.15s;
  }
  .resource-link:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: rgba(95, 155, 101, 0.03);
    text-decoration: none;
  }
  .resource-icon {
    font-size: 13px;
    flex-shrink: 0;
    width: 18px;
    text-align: center;
  }
  .resource-kind {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-hint);
    background: var(--bg-hover);
    padding: 2px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .resource-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .resource-arrow {
    color: var(--text-hint);
    font-size: 12px;
    flex-shrink: 0;
  }
  .resource-link:hover .resource-arrow { color: var(--accent); }

  /* ─────────────────────────────────────────────────────────────
     Course tabs (discussion / reviews / notes)
     ───────────────────────────────────────────────────────────── */
  .course-tabs {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin: 56px 0 16px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .tab-strip {
    display: flex;
    gap: 2px;
    align-items: flex-end;
  }
  .tab-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    text-decoration: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
    border-radius: 6px 6px 0 0;
  }
  .tab-link:hover {
    color: var(--accent);
    border-bottom-color: var(--accent);
    background: rgba(95, 155, 101, 0.04);
    text-decoration: none;
  }
  .tab-label { line-height: 1; }
  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    background: var(--bg-hover);
    color: var(--text-hint);
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .tab-link:hover .tab-count {
    background: rgba(95, 155, 101, 0.15);
    color: var(--accent);
  }
  .tab-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
  }
  .tab-action {
    padding: 6px 14px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    text-decoration: none;
    transition: background 0.15s, transform 0.05s;
    box-shadow: 0 1px 3px rgba(95, 155, 101, 0.3);
  }
  .tab-action:hover {
    background: var(--accent-hover);
    color: white;
    text-decoration: none;
  }
  .tab-action:active { transform: translateY(1px); }

  /* ─────────────────────────────────────────────────────────────
     Discussion card
     ───────────────────────────────────────────────────────────── */
  .discussion-card {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 24px;
    margin-bottom: 32px;
  }
  .discussion-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }
  .discussion-heading {
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 1.2rem;
    margin: 0;
    color: var(--text-primary);
  }
  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 20px;
    padding: 0 8px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .discussion-body { /* CommentThread renders inside */ }

  /* ─────────────────────────────────────────────────────────────
     Misc states
     ───────────────────────────────────────────────────────────── */
  .empty { color: var(--text-secondary); font-style: italic; padding: 12px 0; }
  .meta { color: var(--text-secondary); padding: 24px; }
  .error { color: red; padding: 24px; }

  /* ─────────────────────────────────────────────────────────────
     Modal
     ───────────────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.5); z-index: 1000;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--bg-white, var(--bg-page, #fff));
    border-radius: 10px; padding: 24px;
    width: 90%; max-width: 420px; max-height: 90vh; overflow-y: auto;
    box-shadow: 0 12px 40px rgba(0,0,0,0.25);
  }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); font-weight: 500; font-size: 1.15rem; }
  .modal .form-group { margin-bottom: 12px; }
  .modal .form-group label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px; }
  .modal input {
    width: 100%; padding: 8px; border: 1px solid var(--border);
    border-radius: 6px; font-size: 14px; background: var(--bg-page, #fff);
    color: var(--text-primary, #333); box-sizing: border-box;
  }
  .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
  .btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; cursor: pointer; }
  .btn-primary { background: var(--accent); color: white; border: none; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary { background: var(--bg-white); color: var(--text-primary); border: 1px solid var(--border); }
  .error-msg { color: #c33; font-size: 13px; margin: 0 0 12px; }

  /* ─────────────────────────────────────────────────────────────
     Responsive
     ───────────────────────────────────────────────────────────── */
  @media (max-width: 960px) {
    .course-title { font-size: 1.7rem; }
    .term-meta-callout {
      flex-direction: column;
      align-items: stretch;
      gap: 10px;
    }
    .meta-right { justify-content: flex-end; flex-wrap: wrap; }
    .rating-row { gap: 8px; }
    .my-rating { padding-left: 0; border-left: none; }
    .status-actions { margin-left: 0; width: 100%; }

    /* Hide less-critical schedule columns on small screens */
    .schedule-table th.th-col-outline,
    .schedule-table td.session-cell-outline,
    .schedule-table th.th-col-summary,
    .schedule-table td.session-cell-summary,
    .schedule-table th.th-col-misc,
    .schedule-table td.session-cell-misc {
      display: none;
    }
  }

  @media (max-width: 640px) {
    .term-switcher-bar { gap: 8px; padding: 8px 12px; margin: 0 -12px 24px; }
    .switcher-label { display: none; }
    .switcher-hint { display: none; }
    .pill-row { overflow-x: auto; }

    .schedule-table th, .schedule-table td { padding: 8px 10px; }
    .schedule-table { font-size: 13px; }

    .schedule-table th.th-col-recitation,
    .schedule-table td.session-cell-recitation,
    .schedule-table th.th-col-discussion,
    .schedule-table td.session-cell-discussion {
      display: none;
    }

    .course-tabs { gap: 8px; }
    .tab-link { padding: 8px 10px; font-size: 12px; }

    .discussion-card { padding: 16px; }
  }
</style>
