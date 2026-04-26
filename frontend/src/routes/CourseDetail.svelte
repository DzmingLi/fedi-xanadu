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
      {#if detail.terms.length > 0}
        <div class="term-switcher" aria-label={t('courses.iterations')}>
          <span class="switcher-label">{t('courses.iterations')}</span>
          <div class="pill-row">
            {#each detail.terms as term (term.id)}
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
        </div>
      {/if}
    </header>

    <!-- Course-level textbooks — apply across every iteration. -->
    {#if detail.textbooks.length > 0}
      <section class="course-textbooks">
        <h3>{t('courses.textbooks')}</h3>
        <div class="textbook-row">
          {#each detail.textbooks as tb}
            <a href="/book?id={encodeURIComponent(tb.book_id)}" class="textbook-card">
              {#if tb.cover_url}
                <img src={tb.cover_url} alt="" class="textbook-cover" />
              {/if}
              <div class="textbook-info">
                <span class="textbook-title">{loc(tb.title)}</span>
                <span class="textbook-authors">{tb.authors.join(', ')}</span>
                {#if tb.role !== 'required'}
                  <span class="textbook-role">{tb.role}</span>
                {/if}
              </div>
            </a>
          {/each}
        </div>
      </section>
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
        <div class="term-meta-row">
          <div class="meta-main">
            {#if td.term.semester}
              <span class="meta-chip meta-semester">{td.term.semester}</span>
            {/if}
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
            {#if td.term.source_url}
              <a href={td.term.source_url} target="_blank" rel="noopener" class="meta-source">
                {t('term.source')} ↗
              </a>
            {/if}
          </div>
          <div class="meta-side">
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
                <h2>{t('term.calendar')}</h2>
                <table class="schedule-table">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>{t('term.topic')}</th>
                      {#each visibleColumns as col}
                        <th>{t(col.labelKey)}</th>
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
                              title={sessionDone.get(s.id) ? t('term.markUndone') : t('term.markDone')}
                              aria-label={sessionDone.get(s.id) ? t('term.markUndone') : t('term.markDone')}
                            ></button>
                          {/if}
                        </td>
                        {#if isExam}
                          <td class="session-topic" colspan={colCount - 1 - (isTermOwner ? 1 : 0)}>
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
              </section>
            {/if}
          </div>

        </div>

        <!-- Term-level materials (textbooks, extra resources). Below the
             calendar so the schedule gets full page width. -->
        {#snippet bookCard(tb: NonNullable<typeof td>['textbooks'][number])}
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

        {#if td.textbooks.length > 0 || td.resources.length > 0}
          <div class="term-materials">
            {#if td.textbooks.some(t => t.role === 'required')}
              <section class="textbooks">
                <h3>{t('term.textbooks')}</h3>
                <div class="textbook-row">
                  {#each td.textbooks.filter(t => t.role === 'required') as tb}
                    {@render bookCard(tb)}
                  {/each}
                </div>
              </section>
            {/if}
            {#if td.textbooks.some(t => t.role !== 'required')}
              <section class="textbooks">
                <h3>{t('term.recommendedReading')}</h3>
                <div class="textbook-row">
                  {#each td.textbooks.filter(t => t.role !== 'required') as tb}
                    {@render bookCard(tb)}
                  {/each}
                </div>
              </section>
            {/if}
            {#if td.resources.length > 0}
              <section class="term-resources">
                <h3>{t('term.resources')}</h3>
                {#each td.resources as r}
                  <a href={r.url} target="_blank" rel="noopener" class="resource-link">
                    {#if r.kind}<span class="resource-kind">{r.kind}</span>{/if}
                    <span class="resource-label">{r.label}</span>
                  </a>
                {/each}
              </section>
            {/if}
          </div>
        {/if}
      </section>
    {/if}

    <!-- Course-level discussion / reviews / notes — anchored to the
         course, not the iteration, so they survive term switches. -->
    <nav class="course-tabs">
      <a class="tab-link" href="/course-discussions?id={encodeURIComponent(detail.course.id)}">
        {t('course.discussion')} <span class="tab-count">{detail.discussion_count}</span>
      </a>
      <a class="tab-link" href="/course-reviews?id={encodeURIComponent(detail.course.id)}">
        {t('course.reviews')}
      </a>
      <a class="tab-link" href="/course-notes?id={encodeURIComponent(detail.course.id)}">
        {t('course.notes')}
      </a>
      {#if getAuth()}
        <span class="tab-spacer"></span>
        <a class="tab-action" href="/new?category=review&course_id={encodeURIComponent(detail.course.id)}">{t('course.writeReview')}</a>
        <a class="tab-action" href="/new?category=note&course_id={encodeURIComponent(detail.course.id)}">{t('course.writeNote')}</a>
      {/if}
    </nav>

    <section class="discussion">
      <h2>{t('course.discussion')}  <span class="count">{detail.discussion_count}</span></h2>
      <CommentThread contentUri={`course:${detail.course.id}`} />
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
  .course-page { max-width: 100%; padding: 12px 0; }
  .course-header {
    display: flex; gap: 24px; margin-bottom: 24px; padding-bottom: 16px;
    border-bottom: 1px solid var(--border); align-items: flex-start;
  }
  .header-main { flex: 1; min-width: 0; }
  .term-switcher {
    flex-shrink: 0; max-width: 360px; display: flex; flex-direction: column; gap: 6px;
  }
  .switcher-label {
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em;
    color: var(--text-hint); text-align: right;
  }
  .pill-row { display: flex; flex-wrap: wrap; gap: 6px; justify-content: flex-end; }
  .term-pill {
    padding: 4px 12px; border: 1px solid var(--border); border-radius: 999px;
    background: var(--bg-white); color: var(--text-secondary); font-size: 12px;
    cursor: pointer; white-space: nowrap; transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .term-pill:hover { border-color: var(--accent); color: var(--accent); }
  .term-pill.active { background: var(--accent); border-color: var(--accent); color: white; }

  .course-code {
    display: inline-block; font-size: 12px; padding: 2px 10px;
    background: rgba(95,155,101,0.10); color: var(--accent); border-radius: 3px;
    font-family: var(--font-mono, monospace); margin-bottom: 8px;
  }
  .course-title { font-family: var(--font-serif); font-weight: 500; margin: 4px 0 8px; font-size: 1.6rem; }
  .course-institution { color: var(--text-secondary); margin: 0 0 12px; }
  .course-desc { color: var(--text-secondary); line-height: 1.6; margin: 0; }
  .course-actions { margin-top: 16px; }
  .danger {
    background: transparent; border: 1px solid #c00; color: #c00;
    padding: 4px 12px; border-radius: 3px; cursor: pointer; font-size: 13px;
  }
  .danger:hover { background: #c00; color: white; }

  /* ── Selected-term block ── */
  .term-block { margin-bottom: 32px; }
  .term-meta-row { display: flex; gap: 16px; align-items: flex-start; margin-bottom: 12px; }
  .meta-main { flex: 1; min-width: 0; display: flex; gap: 14px; flex-wrap: wrap; align-items: center; font-size: 13px; }
  .meta-side { flex-shrink: 0; }
  .meta-chip { padding: 3px 10px; border-radius: 3px; font-size: 12px; font-weight: 500; }
  .meta-semester { background: rgba(95,155,101,0.10); color: var(--accent); }
  .meta-instructor { color: var(--text-secondary); }
  .meta-instructor a { color: var(--text-primary); text-decoration: none; }
  .meta-instructor a:hover { color: var(--accent); }
  .meta-source { color: var(--accent); text-decoration: none; font-size: 12px; }
  .meta-source:hover { text-decoration: underline; }
  .edit-btn { font-size: 13px; padding: 4px 12px; border: 1px solid var(--border); border-radius: 4px; color: var(--text-secondary); text-decoration: none; }
  .edit-btn:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }

  .rating-row { display: flex; align-items: center; gap: 10px; margin: 8px 0 16px; flex-wrap: wrap; }
  .rating-stars-display { display: flex; gap: 1px; }
  .rating-value { font-size: 14px; font-weight: 600; color: var(--text-primary); }
  .rating-count { font-size: 12px; color: var(--text-hint); }
  .my-rating { display: inline-flex; align-items: center; gap: 6px; }
  .my-rating-label { font-size: 12px; color: var(--text-hint); }
  .star-picker { display: flex; gap: 1px; cursor: pointer; }
  .star-svg { display: block; }
  .my-rating-value { font-size: 12px; color: #f59e0b; font-weight: 600; }
  .clear-rating { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 4px; line-height: 1; }
  .clear-rating:hover { color: #c00; }
  .status-actions { display: inline-flex; gap: 6px; flex-wrap: wrap; }
  .status-btn { padding: 3px 10px; border: 1px solid var(--border); background: var(--bg-white); border-radius: 4px; font-size: 12px; cursor: pointer; color: var(--text-secondary); transition: all 0.15s; }
  .status-btn:hover { border-color: var(--accent); color: var(--accent); }
  .status-btn.active { background: var(--accent); color: white; border-color: var(--accent); }

  .term-body { display: block; }
  .body-full { width: 100%; }
  .term-materials {
    display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 24px; margin: 32px 0 0;
  }
  .textbook-row { display: flex; flex-wrap: wrap; gap: 12px; }

  .schedule h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.2rem; margin: 0 0 12px; }
  .schedule { margin-bottom: 24px; }
  .schedule-table { width: 100%; border-collapse: collapse; font-size: 14px; }
  .schedule-table th { text-align: left; font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.03em; color: var(--text-hint); padding: 8px 12px; border-bottom: 2px solid var(--border); }
  .schedule-table td { padding: 8px 12px; border-bottom: 1px solid var(--border); vertical-align: top; }
  .schedule-table tbody tr:hover { background: var(--bg-hover, rgba(0,0,0,0.02)); }
  .session-exam { background: var(--bg-hover, rgba(0,0,0,0.02)); }
  .session-num { font-weight: 600; color: var(--text-hint); width: 40px; }
  .session-topic { color: var(--text-primary); width: 40%; }
  .session-cell-video,
  .session-cell-notes,
  .session-cell-slides,
  .session-cell-recitation,
  .session-cell-reading,
  .session-cell-outline,
  .session-cell-summary,
  .session-cell-homework,
  .session-cell-discussion,
  .session-cell-misc {
    max-width: 280px; display: flex; flex-wrap: wrap; gap: 4px; vertical-align: top;
  }
  .res-chip {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: 11px; padding: 2px 8px; border-radius: 3px;
    text-decoration: none; white-space: nowrap;
    background: var(--bg-hover, #f5f5f5); color: var(--text-primary);
    transition: opacity 0.15s; max-width: 100%; overflow: hidden; text-overflow: ellipsis;
  }
  .res-chip:hover { opacity: 0.85; text-decoration: none; }
  .chip-icon { font-size: 11px; flex-shrink: 0; }
  .res-chip-video { background: rgba(220,38,38,0.10); color: #dc2626; }
  .res-chip-homework { background: rgba(16,185,129,0.10); color: #059669; }
  .res-chip-discussion { background: rgba(168,85,247,0.10); color: #7c3aed; }
  .res-chip-optional { opacity: 0.65; font-style: italic; }
  .res-text { display: inline-block; font-size: 11px; padding: 2px 0; color: var(--text-secondary, #555); white-space: nowrap; }
  .res-text-optional { opacity: 0.65; font-style: italic; }
  .session-section td {
    background: rgba(95,155,101,0.10); color: var(--text-primary);
    font-weight: 600; font-family: var(--font-serif);
    text-align: center; padding: 6px 12px;
    border-top: 1px solid rgba(95,155,101,0.35);
    border-bottom: 1px solid rgba(95,155,101,0.35);
  }
  .session-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
  .session-tag { font-size: 10px; padding: 1px 6px; border-radius: 3px; background: rgba(95,155,101,0.08); color: var(--accent); text-decoration: none; }
  .session-tag:hover { background: rgba(95,155,101,0.18); text-decoration: none; }
  .session-check { width: 14px; height: 14px; border: 1.5px solid var(--border); border-radius: 3px; background: transparent; cursor: pointer; margin-left: 6px; padding: 0; vertical-align: middle; position: relative; }
  .session-check:hover { border-color: var(--accent); }
  .session-check.done { background: var(--accent); border-color: var(--accent); }
  .session-check.done::after { content: ''; position: absolute; left: 3px; top: 0px; width: 4px; height: 8px; border: solid white; border-width: 0 2px 2px 0; transform: rotate(45deg); }
  .session-actions { white-space: nowrap; text-align: right; }
  .session-action-btn { background: none; border: none; cursor: pointer; font-size: 13px; color: var(--text-hint); padding: 2px 6px; }
  .session-action-btn:hover { color: var(--accent); }
  .session-actions-col { width: 1%; }
  .session-add-row td { padding: 4px 0; border: none; }
  .session-add-btn { background: none; border: 1px dashed var(--border); color: var(--text-hint); padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 13px; width: 100%; }
  .session-add-btn:hover { border-color: var(--accent); color: var(--accent); }

  .textbooks, .term-resources { margin-bottom: 20px; }
  .textbooks h3, .term-resources h3 { font-family: var(--font-serif); font-weight: 400; font-size: 0.95rem; margin: 0 0 8px; color: var(--text-secondary); }
  .textbook-card { display: flex; gap: 12px; padding: 10px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 8px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .textbook-card:hover { border-color: var(--accent); text-decoration: none; }
  .textbook-cover { width: 48px; height: 64px; object-fit: cover; border-radius: 3px; flex-shrink: 0; background: var(--bg-page); }
  .textbook-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .textbook-title { font-size: 13px; font-weight: 500; color: var(--text-primary); line-height: 1.3; }
  .textbook-authors { font-size: 12px; color: var(--text-secondary); }
  .resource-link { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 5px; text-decoration: none; color: var(--text-primary); font-size: 13px; transition: border-color 0.15s; }
  .resource-link:hover { border-color: var(--accent); color: var(--accent); text-decoration: none; }
  .resource-kind { font-size: 10px; text-transform: uppercase; color: var(--text-hint); background: var(--bg-hover, #f5f5f5); padding: 1px 6px; border-radius: 3px; flex-shrink: 0; }
  .resource-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .course-tabs { display: flex; gap: 4px; margin: 32px 0 12px; border-bottom: 1px solid var(--border); align-items: center; }
  .tab-spacer { flex: 1; }
  .tab-action { padding: 4px 12px; margin-bottom: 6px; background: var(--accent); color: white; border-radius: 4px; font-size: 12px; text-decoration: none; }
  .tab-action:hover { opacity: 0.9; text-decoration: none; }
  .tab-action + .tab-action { margin-left: 6px; }
  .tab-link {
    padding: 8px 14px; font-size: 13px; color: var(--text-secondary);
    text-decoration: none; border-bottom: 2px solid transparent; margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab-link:hover { color: var(--accent); border-bottom-color: var(--accent); text-decoration: none; }
  .tab-count { color: var(--text-hint); font-size: 12px; margin-left: 4px; }

  .discussion { margin-bottom: 32px; }
  .discussion h2 {
    font-family: var(--font-serif); font-weight: 400; font-size: 1.2rem;
    margin: 0 0 12px; color: var(--text-primary);
  }
  .count { color: var(--text-secondary); font-size: 0.9rem; font-family: inherit; margin-left: 4px; }

  .empty { color: var(--text-secondary); font-style: italic; }
  .meta { color: var(--text-secondary); padding: 24px; }
  .error { color: red; padding: 24px; }

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
  .btn { padding: 6px 14px; border-radius: 4px; font-size: 13px; cursor: pointer; }
  .btn-primary { background: var(--accent); color: white; border: none; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary { background: var(--bg-white); color: var(--text-primary); border: 1px solid var(--border); }
  .error-msg { color: #c33; font-size: 13px; margin: 0 0 12px; }

  @media (max-width: 768px) {
    .course-header { flex-direction: column; }
    .term-switcher { max-width: 100%; }
    .pill-row { justify-content: flex-start; }
    .switcher-label { text-align: left; }
  }
</style>
