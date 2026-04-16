<script lang="ts">
  import { getBook, updateBook, getBookEditHistory, rateBook, setReadingStatus, removeReadingStatus, setChapterProgress, createChapter, deleteChapter, updateChapterTags, searchTags, getQuestionsByBook, createQuestion, setPreferredEdition } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { BookDetail, BookEdition, BookChapter, ChapterPrereqEntry } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let locale = $derived(getLocale());

  /** Resolve a localized field (Record<string, string>) to the current locale with fallback. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  let detail = $state<BookDetail | null>(null);
  let loading = $state(true);

  // Rating state
  let hoverRating = $state(0);
  let myRating = $state(0);
  let avgRating = $state(0);
  let ratingCount = $state(0);

  // Reading status state
  let readingStatus = $state('');
  let readingProgress = $state(0);

  // Chapter progress (reactive, keyed by chapter_id)
  let chapterDone = $state(new Map<string, boolean>());

  let selectedEdition = $state('');
  // Auto-select: admin default > first edition
  $effect(() => {
    if (detail && detail.editions.length > 0 && !selectedEdition) {
      const defaultId = detail.book.default_edition_id;
      if (defaultId && detail.editions.some(e => e.id === defaultId)) {
        selectedEdition = defaultId;
      } else {
        selectedEdition = detail.editions[0].id;
      }
    }
  });

  // Q&A
  import { authorName } from '../lib/display';
  import type { Article, ContentFormat } from '../lib/types';
  let bookQuestions = $state<Article[]>([]);
  let showAskForm = $state(false);
  let askTitle = $state('');
  let askContent = $state('');
  let askFormat = $state<ContentFormat>('markdown');
  let askSubmitting = $state(false);

  async function toggleChapter(chapterId: string) {
    const next = !chapterDone.get(chapterId);
    chapterDone.set(chapterId, next);
    chapterDone = new Map(chapterDone); // trigger reactivity
    try { await setChapterProgress(id, chapterId, next); } catch { /* revert on error */ chapterDone.set(chapterId, !next); chapterDone = new Map(chapterDone); }
  }

  // Edit history
  interface EditLog { id: string; editor_did: string; editor_handle: string | null; summary: string; created_at: string; }
  let editHistory = $state<EditLog[]>([]);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      detail = await getBook(id);
      document.title = `${loc(detail.book.title)} — NightBoat`;
      avgRating = detail.rating.avg_rating;
      ratingCount = detail.rating.rating_count;
      myRating = detail.my_rating || 0;
      readingStatus = detail.my_reading_status?.status || '';
      readingProgress = detail.my_reading_status?.progress || 0;
      chapterDone = new Map(detail.my_chapter_progress.map(p => [p.chapter_id, p.completed]));
      getBookEditHistory(id).then(h => { editHistory = h; }).catch(() => {});
      getQuestionsByBook(id).then(qs => { bookQuestions = qs; }).catch(() => {});
    } catch { /* */ }
    loading = false;
  }

  function langLabel(lang: string): string {
    const map: Record<string, string> = {
      zh: '中文', en: 'English', ja: '日本語', ko: '한국어',
      fr: 'Français', de: 'Deutsch', es: 'Español', ru: 'Русский',
    };
    return map[lang] || lang;
  }

  function formatRating(val: number): string {
    return (val / 2).toFixed(1);
  }

  async function submitRating(r: number) {
    if (!getAuth()) return;
    myRating = r;
    try {
      const stats = await rateBook(id, r);
      avgRating = stats.avg_rating;
      ratingCount = stats.rating_count;
    } catch { /* */ }
  }

  async function setStatus(status: string) {
    if (!getAuth()) return;
    if (readingStatus === status) {
      // Toggle off
      readingStatus = '';
      readingProgress = 0;
      try { await removeReadingStatus(id); } catch { /* */ }
    } else {
      readingStatus = status;
      if (status === 'finished') readingProgress = 100;
      else if (status === 'want_to_read') readingProgress = 0;
      try { await setReadingStatus(id, status, readingProgress, selectedEdition || undefined); } catch { /* */ }
    }
  }

  async function updateProgress() {
    if (!getAuth() || readingStatus !== 'reading') return;
    try { await setReadingStatus(id, 'reading', readingProgress); } catch { /* */ }
  }

  // Edit modal state
  let showEdit = $state(false);
  let editTitles = $state<Record<string, string>>({});
  let editDescs = $state<Record<string, string>>({});
  let editSummary = $state('');
  let editSaving = $state(false);
  let editError = $state('');
  let editLang = $state('en');

  const EDIT_LANGS = [
    { code: 'en', label: 'English' },
    { code: 'zh', label: '中文' },
    { code: 'fr', label: 'Français' },
  ];

  function openEdit() {
    if (!detail) return;
    editTitles = { ...detail.book.title };
    editDescs = { ...(detail.book.description || {}) };
    editLang = getLocale();
    editSummary = '';
    editError = '';
    showEdit = true;
  }

  async function saveEdit() {
    // At least one language must have a title
    const hasTitle = Object.values(editTitles).some(v => v.trim());
    if (!hasTitle) { editError = t('books.editTitleRequired'); return; }
    editSaving = true;
    editError = '';
    try {
      // Clean empty entries
      const title = Object.fromEntries(Object.entries(editTitles).filter(([_, v]) => v.trim()));
      const description = Object.fromEntries(Object.entries(editDescs).filter(([_, v]) => v.trim()));
      await updateBook(id, {
        title,
        description,
        edit_summary: editSummary.trim() || undefined,
      });
      showEdit = false;
      await load();
    } catch (e: any) {
      editError = e.message;
    } finally {
      editSaving = false;
    }
  }

  // ---- Chapter management ----
  let showChapterForm = $state(false);
  let newChapterTitle = $state('');
  let newChapterParentId = $state('');
  let newChapterArticleUri = $state('');
  let newChapterTeaches = $state<string[]>([]);
  let newChapterPrereqs = $state<ChapterPrereqEntry[]>([]);
  let newChapterTagInput = $state('');
  let newChapterTagSuggestions = $state<{id:string,name:string}[]>([]);
  let newChapterPrereqInput = $state('');
  let newChapterPrereqSuggestions = $state<{id:string,name:string}[]>([]);
  let newChapterPrereqType = $state<'required'|'recommended'>('required');
  let chapterSaving = $state(false);

  // Tag autocomplete for new chapter form
  let tagSearchTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(tagSearchTimeout);
    const q = newChapterTagInput.trim();
    if (!q) { newChapterTagSuggestions = []; return; }
    tagSearchTimeout = setTimeout(async () => {
      newChapterTagSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });

  let prereqSearchTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(prereqSearchTimeout);
    const q = newChapterPrereqInput.trim();
    if (!q) { newChapterPrereqSuggestions = []; return; }
    prereqSearchTimeout = setTimeout(async () => {
      newChapterPrereqSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });

  function addChapterTeachTag(tagId: string) {
    if (!newChapterTeaches.includes(tagId)) newChapterTeaches = [...newChapterTeaches, tagId];
    newChapterTagInput = '';
    newChapterTagSuggestions = [];
  }
  function removeChapterTeachTag(tagId: string) {
    newChapterTeaches = newChapterTeaches.filter(t => t !== tagId);
  }
  function addChapterPrereq(tagId: string) {
    if (!newChapterPrereqs.find(p => p.tag_id === tagId)) {
      newChapterPrereqs = [...newChapterPrereqs, { tag_id: tagId, prereq_type: newChapterPrereqType }];
    }
    newChapterPrereqInput = '';
    newChapterPrereqSuggestions = [];
  }
  function removeChapterPrereq(tagId: string) {
    newChapterPrereqs = newChapterPrereqs.filter(p => p.tag_id !== tagId);
  }

  async function submitNewChapter() {
    if (!newChapterTitle.trim()) return;
    chapterSaving = true;
    try {
      const rootChapters = detail?.chapters.filter(c => !c.parent_id) ?? [];
      await createChapter(id, {
        title: newChapterTitle.trim(),
        parent_id: newChapterParentId || undefined,
        order_index: rootChapters.length,
        article_uri: newChapterArticleUri.trim() || undefined,
        teaches: newChapterTeaches,
        prereqs: newChapterPrereqs,
      });
      newChapterTitle = '';
      newChapterParentId = '';
      newChapterArticleUri = '';
      newChapterTeaches = [];
      newChapterPrereqs = [];
      showChapterForm = false;
      await load();
    } catch { /* */ } finally {
      chapterSaving = false;
    }
  }

  async function removeChapter(chapterId: string) {
    await deleteChapter(id, chapterId).catch(() => {});
    await load();
  }

  // Per-chapter tag editing
  let editingChapterId = $state<string | null>(null);
  let editChapterTeaches = $state<string[]>([]);
  let editChapterPrereqs = $state<ChapterPrereqEntry[]>([]);
  let editChapterTagInput = $state('');
  let editChapterTagSuggestions = $state<{id:string,name:string}[]>([]);
  let editChapterPrereqInput = $state('');
  let editChapterPrereqSuggestions = $state<{id:string,name:string}[]>([]);
  let editChapterPrereqType = $state<'required'|'recommended'>('required');

  let editTagSearchTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(editTagSearchTimeout);
    const q = editChapterTagInput.trim();
    if (!q) { editChapterTagSuggestions = []; return; }
    editTagSearchTimeout = setTimeout(async () => {
      editChapterTagSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });

  let editPrereqSearchTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(editPrereqSearchTimeout);
    const q = editChapterPrereqInput.trim();
    if (!q) { editChapterPrereqSuggestions = []; return; }
    editPrereqSearchTimeout = setTimeout(async () => {
      editChapterPrereqSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });

  function openChapterTagEdit(ch: BookChapter) {
    editingChapterId = ch.id;
    editChapterTeaches = [...ch.teaches];
    editChapterPrereqs = ch.prereqs.map(p => ({ ...p }));
    editChapterTagInput = '';
    editChapterPrereqInput = '';
  }
  function closeChapterTagEdit() { editingChapterId = null; }

  function addEditTeachTag(tagId: string) {
    if (!editChapterTeaches.includes(tagId)) editChapterTeaches = [...editChapterTeaches, tagId];
    editChapterTagInput = '';
    editChapterTagSuggestions = [];
  }
  function removeEditTeachTag(tagId: string) {
    editChapterTeaches = editChapterTeaches.filter(t => t !== tagId);
  }
  function addEditPrereq(tagId: string) {
    if (!editChapterPrereqs.find(p => p.tag_id === tagId)) {
      editChapterPrereqs = [...editChapterPrereqs, { tag_id: tagId, prereq_type: editChapterPrereqType }];
    }
    editChapterPrereqInput = '';
    editChapterPrereqSuggestions = [];
  }
  function removeEditPrereq(tagId: string) {
    editChapterPrereqs = editChapterPrereqs.filter(p => p.tag_id !== tagId);
  }

  async function saveChapterTags() {
    if (!editingChapterId) return;
    await updateChapterTags(id, editingChapterId, editChapterTeaches, editChapterPrereqs).catch(() => {});
    await load();
    editingChapterId = null;
  }
</script>

{#if loading}
  <p class="meta">Loading...</p>
{:else if detail}
  <div class="book-layout">
    <div class="book-main">
      <!-- Header -->
      <div class="book-header">
        {#if detail.book.cover_url}
          <img src={detail.book.cover_url} alt={loc(detail.book.title)} class="cover" />
        {:else}
          <div class="cover placeholder">
            <span>{loc(detail.book.title).charAt(0)}</span>
          </div>
        {/if}
        <div class="book-meta">
          <h1>{loc(detail.book.title)}</h1>
          <p class="authors">{detail.book.authors.join(', ')}</p>
          {#if loc(detail.book.description)}
            <p class="description">{loc(detail.book.description)}</p>
          {/if}

          <!-- Tags -->
          {#if detail.tags.length > 0 || detail.prereqs.length > 0}
            <div class="book-tags">
              {#each detail.tags as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches">{tag}</a>
              {/each}
              {#each detail.prereqs as prereq}
                <a href="/tag?id={encodeURIComponent(prereq)}" class="tag-badge prereq">{prereq}</a>
              {/each}
            </div>
          {/if}

          <!-- Rating display -->
          <div class="rating-row">
            <span class="rating-stars-display">
              {#each [1,2,3,4,5] as star}
                {@const val = avgRating / 2}
                {@const filled = val >= star}
                {@const half = !filled && val >= star - 0.5}
                <svg class="star-svg" viewBox="0 0 24 24" width="28" height="28">
                  <defs>
                    <clipPath id="star-left-{star}"><rect x="0" y="0" width="12" height="24"/></clipPath>
                    <clipPath id="star-right-{star}"><rect x="12" y="0" width="12" height="24"/></clipPath>
                  </defs>
                  {#if filled}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                  {:else if half}
                    <path clip-path="url(#star-left-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="#f59e0b" stroke="#f59e0b" stroke-width="1"/>
                    <path clip-path="url(#star-right-{star})" d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {:else}
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="none" stroke="#ccc" stroke-width="1.5"/>
                  {/if}
                </svg>
              {/each}
            </span>
            <span class="rating-value">{formatRating(avgRating)}</span>
            <span class="rating-count">({ratingCount})</span>
          </div>

          <!-- User rating -->
          {#if getAuth()}
            <div class="my-rating">
              <span class="my-rating-label">{t('books.myRating')}:</span>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <span class="star-picker" onmouseleave={() => { hoverRating = 0; }}>
                {#each [1,2,3,4,5] as star}
                  {@const activeVal = hoverRating || myRating}
                  {@const leftActive = activeVal >= star * 2 - 1}
                  {@const rightActive = activeVal >= star * 2}
                  <svg class="star-svg" viewBox="0 0 24 24" width="24" height="24">
                    <!-- Left half (odd value) -->
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <g clip-path="inset(0 50% 0 0)"
                       onmouseenter={() => { hoverRating = star * 2 - 1; }}
                       onclick={() => submitRating(star * 2 - 1)}
                       role="button" tabindex="-1">
                      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                            fill={leftActive ? '#f59e0b' : 'none'}
                            stroke={leftActive ? '#f59e0b' : '#ccc'}
                            stroke-width="1.5"/>
                    </g>
                    <!-- Right half (even value) -->
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <g clip-path="inset(0 0 0 50%)"
                       onmouseenter={() => { hoverRating = star * 2; }}
                       onclick={() => submitRating(star * 2)}
                       role="button" tabindex="-1">
                      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                            fill={rightActive ? '#f59e0b' : 'none'}
                            stroke={rightActive ? '#f59e0b' : '#ccc'}
                            stroke-width="1.5"/>
                    </g>
                  </svg>
                {/each}
              </span>
              {#if myRating > 0}
                <span class="my-rating-value">{formatRating(myRating)}</span>
              {/if}
            </div>
          {/if}

          <!-- Reading status + actions -->
          <div class="actions">
            {#if getAuth()}
              <button class="action-btn" class:active={readingStatus === 'want_to_read'} onclick={() => setStatus('want_to_read')}>
                {t('books.wantToRead')}
              </button>
              <button class="action-btn" class:active={readingStatus === 'reading'} onclick={() => setStatus('reading')}>
                {t('books.reading')}
              </button>
              <button class="action-btn" class:active={readingStatus === 'finished'} onclick={() => setStatus('finished')}>
                {t('books.finished')}
              </button>
              <button class="action-btn" class:active={readingStatus === 'dropped'} onclick={() => setStatus('dropped')}>
                {t('books.dropped')}
              </button>
              {#if detail.editions.length > 0}
                <select class="edition-select" bind:value={selectedEdition} title="Edition">
                  {#each detail.editions as ed}
                    <option value={ed.id}>{ed.title} ({ed.lang}{ed.year ? `, ${ed.year}` : ''})</option>
                  {/each}
                </select>
              {/if}
            {/if}
            {#if getAuth()}
              <a href="/new?category=review&book_id={encodeURIComponent(id)}" class="action-btn primary">
                {t('books.writeReview')}
              </a>
            {/if}
            <button class="action-btn" onclick={openEdit}>
              {t('books.editInfo')}
            </button>
          </div>

          <!-- Progress bar for "reading" -->
          {#if readingStatus === 'reading'}
            <div class="progress-section">
              <label class="progress-label">
                {t('books.progress')}: {readingProgress}%
                <input type="range" min="0" max="100" bind:value={readingProgress} onchange={updateProgress} class="progress-slider" />
              </label>
            </div>
          {/if}
        </div>
      </div>

      <!-- Chapters / Table of Contents -->
      <div class="chapters-section">
        <div class="chapters-header">
          <h2>{t('books.tableOfContents')}</h2>
          {#if getAuth()}
            <button class="add-chapter-btn" onclick={() => { showChapterForm = !showChapterForm; }}>
              {showChapterForm ? '取消' : '+ 添加章节'}
            </button>
          {/if}
        </div>

        <!-- Add chapter form -->
        {#if showChapterForm}
          <div class="chapter-form">
            <input class="chapter-input" bind:value={newChapterTitle} placeholder="章节标题" />
            <input class="chapter-input" bind:value={newChapterArticleUri} placeholder="关联文章 URI（可选）" />
            <select class="chapter-input" bind:value={newChapterParentId}>
              <option value="">顶层章节</option>
              {#each (detail?.chapters ?? []).filter(c => !c.parent_id) as ch}
                <option value={ch.id}>{ch.title}</option>
              {/each}
            </select>

            <!-- Teaches tags -->
            <div class="tag-editor">
              <div class="tag-editor-label">教授知识点</div>
              <div class="tag-list">
                {#each newChapterTeaches as tag}
                  <span class="tag-badge teaches">
                    {tag}
                    <button class="tag-remove" onclick={() => removeChapterTeachTag(tag)}>×</button>
                  </span>
                {/each}
              </div>
              <div class="tag-input-wrap">
                <input class="tag-input" bind:value={newChapterTagInput} placeholder="输入 tag..." />
                {#if newChapterTagSuggestions.length > 0}
                  <ul class="tag-suggestions">
                    {#each newChapterTagSuggestions as s}
                      <li><button onclick={() => addChapterTeachTag(s.id)}>{s.name || s.id}</button></li>
                    {/each}
                  </ul>
                {/if}
              </div>
              {#if newChapterTagInput.trim()}
                <button class="tag-add-btn" onclick={() => addChapterTeachTag(newChapterTagInput.trim())}>添加</button>
              {/if}
            </div>

            <!-- Prereq tags -->
            <div class="tag-editor">
              <div class="tag-editor-label">前置知识</div>
              <div class="tag-list">
                {#each newChapterPrereqs as p}
                  <span class="tag-badge prereq">
                    {p.tag_id}
                    <span class="prereq-type-label">{p.prereq_type === 'required' ? '必须' : '推荐'}</span>
                    <button class="tag-remove" onclick={() => removeChapterPrereq(p.tag_id)}>×</button>
                  </span>
                {/each}
              </div>
              <div class="tag-input-wrap">
                <select class="prereq-type-select" bind:value={newChapterPrereqType}>
                  <option value="required">必须</option>
                  <option value="recommended">推荐</option>
                </select>
                <input class="tag-input" bind:value={newChapterPrereqInput} placeholder="输入前置 tag..." />
                {#if newChapterPrereqSuggestions.length > 0}
                  <ul class="tag-suggestions">
                    {#each newChapterPrereqSuggestions as s}
                      <li><button onclick={() => addChapterPrereq(s.id)}>{s.name || s.id}</button></li>
                    {/each}
                  </ul>
                {/if}
              </div>
              {#if newChapterPrereqInput.trim()}
                <button class="tag-add-btn" onclick={() => addChapterPrereq(newChapterPrereqInput.trim())}>添加</button>
              {/if}
            </div>

            <button class="action-btn primary" onclick={submitNewChapter} disabled={chapterSaving || !newChapterTitle.trim()}>
              {chapterSaving ? '保存中…' : '保存章节'}
            </button>
          </div>
        {/if}

        {#if detail.chapters.length > 0}
          {@const rootChapters = detail.chapters.filter(c => !c.parent_id)}
          {#each rootChapters as ch}
            {@const children = detail.chapters.filter(c => c.parent_id === ch.id)}
            <div class="chapter-item">
              <div class="chapter-row">
                {#if getAuth()}
                  <button
                    class="chapter-check"
                    class:done={chapterDone.get(ch.id)}
                    onclick={() => toggleChapter(ch.id)}
                    title={chapterDone.get(ch.id) ? '标为未读' : '标为已读'}
                  ></button>
                {/if}
                <div class="chapter-content">
                  {#if ch.article_uri}
                    <a href="/article?uri={encodeURIComponent(ch.article_uri)}" class="chapter-title">{ch.title}</a>
                  {:else}
                    <span class="chapter-title">{ch.title}</span>
                  {/if}
                  <!-- Chapter tags display -->
                  {#if ch.teaches.length > 0 || ch.prereqs.length > 0}
                    <div class="chapter-tag-row">
                      {#each ch.teaches as tag}
                        <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches sm">{tag}</a>
                      {/each}
                      {#each ch.prereqs as p}
                        <span class="tag-badge prereq sm" title="{p.prereq_type === 'required' ? '必须前置' : '推荐前置'}">{p.tag_id}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
                {#if getAuth()}
                  <button class="chapter-edit-tags-btn" onclick={() => editingChapterId === ch.id ? closeChapterTagEdit() : openChapterTagEdit(ch)}>
                    {editingChapterId === ch.id ? '收起' : '编辑 tag'}
                  </button>
                  <button class="chapter-delete-btn" onclick={() => removeChapter(ch.id)}>删除</button>
                {/if}
              </div>

              <!-- Inline tag editor for this chapter -->
              {#if editingChapterId === ch.id}
                <div class="chapter-tag-editor">
                  <div class="tag-editor">
                    <div class="tag-editor-label">教授知识点</div>
                    <div class="tag-list">
                      {#each editChapterTeaches as tag}
                        <span class="tag-badge teaches">
                          {tag} <button class="tag-remove" onclick={() => removeEditTeachTag(tag)}>×</button>
                        </span>
                      {/each}
                    </div>
                    <div class="tag-input-wrap">
                      <input class="tag-input" bind:value={editChapterTagInput} placeholder="输入 tag..." />
                      {#if editChapterTagSuggestions.length > 0}
                        <ul class="tag-suggestions">
                          {#each editChapterTagSuggestions as s}
                            <li><button onclick={() => addEditTeachTag(s.id)}>{s.name || s.id}</button></li>
                          {/each}
                        </ul>
                      {/if}
                    </div>
                    {#if editChapterTagInput.trim()}
                      <button class="tag-add-btn" onclick={() => addEditTeachTag(editChapterTagInput.trim())}>添加</button>
                    {/if}
                  </div>
                  <div class="tag-editor">
                    <div class="tag-editor-label">前置知识</div>
                    <div class="tag-list">
                      {#each editChapterPrereqs as p}
                        <span class="tag-badge prereq">
                          {p.tag_id} <span class="prereq-type-label">{p.prereq_type === 'required' ? '必须' : '推荐'}</span>
                          <button class="tag-remove" onclick={() => removeEditPrereq(p.tag_id)}>×</button>
                        </span>
                      {/each}
                    </div>
                    <div class="tag-input-wrap">
                      <select class="prereq-type-select" bind:value={editChapterPrereqType}>
                        <option value="required">必须</option>
                        <option value="recommended">推荐</option>
                      </select>
                      <input class="tag-input" bind:value={editChapterPrereqInput} placeholder="输入前置 tag..." />
                      {#if editChapterPrereqSuggestions.length > 0}
                        <ul class="tag-suggestions">
                          {#each editChapterPrereqSuggestions as s}
                            <li><button onclick={() => addEditPrereq(s.id)}>{s.name || s.id}</button></li>
                          {/each}
                        </ul>
                      {/if}
                    </div>
                    {#if editChapterPrereqInput.trim()}
                      <button class="tag-add-btn" onclick={() => addEditPrereq(editChapterPrereqInput.trim())}>添加</button>
                    {/if}
                  </div>
                  <div class="chapter-tag-editor-actions">
                    <button class="action-btn primary" onclick={saveChapterTags}>保存</button>
                    <button class="action-btn" onclick={closeChapterTagEdit}>取消</button>
                  </div>
                </div>
              {/if}

              {#if children.length > 0}
                <div class="chapter-children">
                  {#each children as sub}
                    <div class="chapter-row sub">
                      {#if getAuth()}
                        <button
                          class="chapter-check"
                          class:done={chapterDone.get(sub.id)}
                          onclick={() => toggleChapter(sub.id)}
                          title={chapterDone.get(sub.id) ? '标为未读' : '标为已读'}
                        ></button>
                      {/if}
                      <div class="chapter-content">
                        {#if sub.article_uri}
                          <a href="/article?uri={encodeURIComponent(sub.article_uri)}" class="chapter-title">{sub.title}</a>
                        {:else}
                          <span class="chapter-title">{sub.title}</span>
                        {/if}
                        {#if sub.teaches.length > 0 || sub.prereqs.length > 0}
                          <div class="chapter-tag-row">
                            {#each sub.teaches as tag}
                              <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches sm">{tag}</a>
                            {/each}
                            {#each sub.prereqs as p}
                              <span class="tag-badge prereq sm">{p.tag_id}</span>
                            {/each}
                          </div>
                        {/if}
                      </div>
                      {#if getAuth()}
                        <button class="chapter-edit-tags-btn" onclick={() => editingChapterId === sub.id ? closeChapterTagEdit() : openChapterTagEdit(sub)}>
                          {editingChapterId === sub.id ? '收起' : '编辑 tag'}
                        </button>
                        <button class="chapter-delete-btn" onclick={() => removeChapter(sub.id)}>删除</button>
                      {/if}
                    </div>
                    {#if editingChapterId === sub.id}
                      <div class="chapter-tag-editor sub">
                        <div class="tag-editor">
                          <div class="tag-editor-label">教授知识点</div>
                          <div class="tag-list">
                            {#each editChapterTeaches as tag}
                              <span class="tag-badge teaches">
                                {tag} <button class="tag-remove" onclick={() => removeEditTeachTag(tag)}>×</button>
                              </span>
                            {/each}
                          </div>
                          <div class="tag-input-wrap">
                            <input class="tag-input" bind:value={editChapterTagInput} placeholder="输入 tag..." />
                            {#if editChapterTagSuggestions.length > 0}
                              <ul class="tag-suggestions">
                                {#each editChapterTagSuggestions as s}
                                  <li><button onclick={() => addEditTeachTag(s.id)}>{s.name || s.id}</button></li>
                                {/each}
                              </ul>
                            {/if}
                          </div>
                          {#if editChapterTagInput.trim()}
                            <button class="tag-add-btn" onclick={() => addEditTeachTag(editChapterTagInput.trim())}>添加</button>
                          {/if}
                        </div>
                        <div class="tag-editor">
                          <div class="tag-editor-label">前置知识</div>
                          <div class="tag-list">
                            {#each editChapterPrereqs as p}
                              <span class="tag-badge prereq">
                                {p.tag_id} <span class="prereq-type-label">{p.prereq_type === 'required' ? '必须' : '推荐'}</span>
                                <button class="tag-remove" onclick={() => removeEditPrereq(p.tag_id)}>×</button>
                              </span>
                            {/each}
                          </div>
                          <div class="tag-input-wrap">
                            <select class="prereq-type-select" bind:value={editChapterPrereqType}>
                              <option value="required">必须</option>
                              <option value="recommended">推荐</option>
                            </select>
                            <input class="tag-input" bind:value={editChapterPrereqInput} placeholder="输入前置 tag..." />
                            {#if editChapterPrereqSuggestions.length > 0}
                              <ul class="tag-suggestions">
                                {#each editChapterPrereqSuggestions as s}
                                  <li><button onclick={() => addEditPrereq(s.id)}>{s.name || s.id}</button></li>
                                {/each}
                              </ul>
                            {/if}
                          </div>
                          {#if editChapterPrereqInput.trim()}
                            <button class="tag-add-btn" onclick={() => addEditPrereq(editChapterPrereqInput.trim())}>添加</button>
                          {/if}
                        </div>
                        <div class="chapter-tag-editor-actions">
                          <button class="action-btn primary" onclick={saveChapterTags}>保存</button>
                          <button class="action-btn" onclick={closeChapterTagEdit}>取消</button>
                        </div>
                      </div>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        {:else if !showChapterForm}
          <p class="empty">暂无章节目录</p>
        {/if}
      </div>

      <!-- Reviews -->
      <div class="reviews-section">
        <h2>{t('books.reviews')}</h2>
        {#if detail.reviews.length === 0}
          <p class="empty">{t('books.noReviews')}</p>
        {:else}
          {#each detail.reviews as review}
            <div class="review-wrapper">
              {#if review.edition_id}
                {@const ed = detail.editions.find(e => e.id === review.edition_id)}
                {#if ed}
                  <span class="review-edition">{ed.title} ({ed.lang}{ed.year ? `, ${ed.year}` : ''})</span>
                {/if}
              {/if}
              <PostCard article={review} articleTeaches={[]} />
            </div>
          {/each}
        {/if}
      </div>

      <!-- Q&A -->
      <div class="qa-section">
        <div class="qa-header">
          <h2>{t('books.qa') || 'Questions & Answers'}</h2>
          {#if getAuth()}
            <button class="ask-btn" onclick={() => showAskForm = !showAskForm}>
              {t('books.askQuestion') || 'Ask a Question'}
            </button>
          {/if}
        </div>

        {#if showAskForm}
          <div class="ask-form">
            <input type="text" bind:value={askTitle} placeholder={t('books.askTitlePlaceholder') || 'What do you want to know about this book?'} class="ask-title" />
            <textarea bind:value={askContent} rows="3" placeholder={t('books.askContentPlaceholder') || 'Add details (optional)...'} class="ask-body"></textarea>
            <div class="ask-bar">
              <select bind:value={askFormat} class="format-select">
                <option value="markdown">Markdown</option>
                <option value="typst">Typst</option>
              </select>
              <button class="btn-cancel" onclick={() => { showAskForm = false; askTitle = ''; askContent = ''; }}>{t('common.cancel')}</button>
              <button class="btn-submit" disabled={askSubmitting || !askTitle.trim()} onclick={async () => {
                askSubmitting = true;
                try {
                  await createQuestion({ title: askTitle.trim(), content: askContent || ' ', content_format: askFormat, tags: [], prereqs: [], book_id: id } as any);
                  askTitle = ''; askContent = ''; showAskForm = false;
                  bookQuestions = await getQuestionsByBook(id);
                } catch { /* */ }
                askSubmitting = false;
              }}>{askSubmitting ? '...' : t('common.submit') || 'Submit'}</button>
            </div>
          </div>
        {/if}

        {#if bookQuestions.length === 0}
          <p class="empty">{t('books.noQuestions') || 'No questions yet. Be the first to ask!'}</p>
        {:else}
          <div class="question-list">
            {#each bookQuestions as q}
              <a href="/question?uri={encodeURIComponent(q.at_uri)}" class="q-card">
                <div class="q-card-main">
                  <span class="q-card-title">{q.title}</span>
                  <span class="q-card-meta">
                    {authorName(q)} &middot; {q.answer_count} {t('qa.answers') || 'answers'}
                    {#if q.vote_score > 0} &middot; &#9650;{q.vote_score}{/if}
                  </span>
                </div>
                <div class="q-card-answers">
                  <span class="answer-count" class:has-answers={q.answer_count > 0}>{q.answer_count}</span>
                  <span class="answer-label">{t('qa.answers') || 'answers'}</span>
                </div>
              </a>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Right sidebar: Editions -->
    <aside class="book-sidebar">
      <h3>{t('books.editions')}</h3>
      {#each detail.editions as ed}
        <div class="edition-card">
          {#if ed.cover_url}
            <img src={ed.cover_url} alt={ed.title} class="edition-cover" />
          {/if}
          <div class="edition-top">
            <strong>{ed.title}</strong>
            <span class="edition-lang">{langLabel(ed.lang)}</span>
          </div>
          <div class="edition-details">
            {#if ed.isbn}<span>ISBN: {ed.isbn}</span>{/if}
            {#if ed.publisher}<span>{ed.publisher}</span>{/if}
            {#if ed.year}<span>{ed.year}</span>{/if}
            {#if ed.translators.length > 0}
              <span>{t('books.translators')}: {ed.translators.join(', ')}</span>
            {/if}
          </div>
          {#if ed.purchase_links.length > 0}
            <div class="purchase-links">
              {#each ed.purchase_links as link}
                <a href={link.url} target="_blank" rel="noopener" class="purchase-link">{link.label}</a>
              {/each}
            </div>
          {/if}
          {#if getAuth() && ed.cover_url}
            <button class="set-cover-btn" onclick={async () => {
              await setPreferredEdition(id, ed.id);
              await load();
            }}>{t('books.setAsCover') || 'Use this cover'}</button>
          {/if}
        </div>
      {/each}
      {#if getAuth()}
        <a href="/book-edition?book_id={encodeURIComponent(id)}" class="add-edition-btn">
          + {t('books.addEdition')}
        </a>
      {/if}

      <div class="sidebar-stats">
        <span>{detail.editions.length} {t('books.editionCount')}</span>
        <span>{detail.review_count} {t('books.reviewCount')}</span>
      </div>

      <!-- Edit history -->
      <div class="edit-history">
        <h3>{t('books.editHistory')}</h3>
        {#if editHistory.length === 0}
          <p class="empty-hint">{t('books.noEditHistory')}</p>
        {:else}
          {#each editHistory.slice(0, 10) as log}
            <div class="edit-log">
              <span class="edit-log-who">{log.editor_handle ? `@${log.editor_handle}` : log.editor_did.slice(0, 20)}</span>
              <span class="edit-log-summary">{log.summary || '—'}</span>
              <span class="edit-log-time">{new Date(log.created_at).toLocaleDateString()}</span>
            </div>
          {/each}
        {/if}
        {#if getAuth()}
          <button class="report-dispute-btn" onclick={() => {
            const uri = `book:${id}`;
            window.location.href = `/report?kind=book_dispute&target_uri=${encodeURIComponent(uri)}`;
          }}>
            {t('books.reportDispute')}
          </button>
        {/if}
      </div>
    </aside>
  </div>

  <!-- Edit modal -->
  {#if showEdit}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={() => showEdit = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('books.editInfo')}</h3>
        {#if editError}<p class="error-msg">{editError}</p>{/if}

        <!-- Language tabs -->
        <div class="lang-tabs">
          {#each EDIT_LANGS as lang}
            <button class="lang-tab" class:active={editLang === lang.code} onclick={() => editLang = lang.code}>
              {lang.label}
              {#if editTitles[lang.code]}<span class="lang-dot"></span>{/if}
            </button>
          {/each}
        </div>

        <div class="form-group">
          <label>{t('books.titleLabel')} ({editLang})</label>
          <input bind:value={editTitles[editLang]} placeholder={editTitles['en'] || ''} />
        </div>
        <div class="form-group">
          <label>{t('books.descriptionLabel')} ({editLang})</label>
          <textarea bind:value={editDescs[editLang]} rows="3" placeholder={editDescs['en'] || ''}></textarea>
        </div>
        <div class="form-group">
          <label>{t('books.editSummary')}</label>
          <input bind:value={editSummary} placeholder={t('books.editSummaryPlaceholder')} />
        </div>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => showEdit = false}>{t('books.cancel')}</button>
          <button class="btn btn-primary" onclick={saveEdit} disabled={editSaving}>
            {editSaving ? t('books.saving') : t('books.save')}
          </button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.5); z-index: 1000;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--bg-white, var(--bg-page, #fff)); border-radius: 8px; padding: 24px;
    width: 90%; max-width: 480px; max-height: 90vh; overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0,0,0,0.3);
  }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); }
  .modal .form-group { margin-bottom: 12px; }
  .modal .form-group label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px; }
  .modal input, .modal textarea {
    width: 100%; padding: 8px; border: 1px solid var(--border);
    border-radius: 4px; font-size: 14px; background: var(--bg-page, #fff);
    color: var(--text-primary, #333); box-sizing: border-box;
  }
  .modal textarea { resize: vertical; }
  .lang-tabs { display: flex; gap: 4px; margin-bottom: 12px; }
  .lang-tab { font-size: 12px; padding: 4px 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; position: relative; }
  .lang-tab:hover { border-color: var(--accent); color: var(--accent); }
  .lang-tab.active { background: var(--accent); color: white; border-color: var(--accent); }
  .lang-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); position: absolute; top: 2px; right: 2px; }
  .lang-tab.active .lang-dot { background: white; }
  .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
  .error-msg { color: #c33; font-size: 13px; margin: 0 0 12px; }
  .book-layout {
    display: flex;
    gap: 32px;
  }
  .book-main {
    flex: 1;
    min-width: 0;
  }
  .book-sidebar {
    width: 280px;
    flex-shrink: 0;
  }
  .book-sidebar h3 {
    font-family: var(--font-serif);
    font-size: 1rem;
    font-weight: 400;
    margin: 0 0 12px;
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .book-layout {
      flex-direction: column;
    }
    .book-sidebar {
      width: 100%;
    }
  }

  .book-header {
    display: flex;
    gap: 24px;
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border);
  }
  .cover {
    width: 140px;
    height: 200px;
    object-fit: cover;
    border-radius: 6px;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }
  .cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--border);
    color: var(--text-hint);
    font-size: 48px;
    font-family: var(--font-serif);
  }
  .book-meta { flex: 1; }
  .book-meta h1 {
    margin: 0;
    font-family: var(--font-serif);
    font-size: 1.6rem;
  }
  .authors {
    margin: 4px 0 0;
    font-size: 15px;
    color: var(--text-secondary);
  }
  .description {
    margin: 12px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  /* Rating display */
  .rating-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .rating-stars-display {
    display: inline-flex;
    gap: 2px;
    align-items: center;
  }
  .rating-value {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .rating-count {
    font-size: 13px;
    color: var(--text-hint);
  }

  /* User rating picker */
  .my-rating {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .my-rating-label { flex-shrink: 0; }
  .my-rating-value {
    font-weight: 600;
    color: #f59e0b;
  }
  .star-picker {
    display: inline-flex;
    gap: 2px;
    cursor: pointer;
    align-items: center;
  }
  .star-svg {
    display: block;
  }
  .star-svg g {
    cursor: pointer;
  }

  /* Actions */
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    flex-wrap: wrap;
  }
  .action-btn {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    color: var(--text-secondary);
    text-decoration: none;
    cursor: pointer;
    transition: all 0.15s;
  }
  .action-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .action-btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .action-btn.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .action-btn.primary:hover { opacity: 0.9; }

  /* Progress */
  .progress-section {
    margin-top: 10px;
  }
  .progress-label {
    font-size: 13px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .progress-slider {
    flex: 1;
    max-width: 200px;
    accent-color: var(--accent);
  }

  /* Editions (sidebar) */
  .edition-card {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 8px;
    background: var(--bg-white);
  }
  .edition-cover {
    width: 100%;
    max-height: 200px;
    object-fit: contain;
    border-radius: 3px;
    margin-bottom: 8px;
  }
  .edition-top {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .edition-lang {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-dim);
    color: var(--text-hint);
  }
  .edition-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-hint);
  }
  .purchase-links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .purchase-link {
    font-size: 11px;
    padding: 2px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    text-decoration: none;
    transition: all 0.15s;
  }
  .purchase-link:hover {
    background: var(--accent);
    color: white;
    text-decoration: none;
  }
  .add-edition-btn {
    display: inline-block;
    font-size: 12px;
    color: var(--text-hint);
    text-decoration: none;
    padding: 4px 0;
    transition: color 0.15s;
  }
  .add-edition-btn:hover { color: var(--accent); text-decoration: none; }
  .sidebar-stats {
    display: flex;
    gap: 12px;
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-hint);
  }

  /* Edit history */
  .edit-history {
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }
  .edit-history h3 {
    font-size: 13px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.04em; color: var(--text-hint); margin: 0 0 8px;
  }
  .empty-hint { font-size: 12px; color: var(--text-hint); }
  .edit-log {
    display: flex; flex-direction: column; gap: 1px;
    padding: 6px 0; border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .edit-log-who { color: var(--accent); font-weight: 500; }
  .edit-log-summary { color: var(--text-secondary); }
  .edit-log-time { color: var(--text-hint); font-size: 11px; }
  .report-dispute-btn {
    margin-top: 12px; padding: 4px 10px;
    font-size: 12px; color: var(--text-hint);
    border: 1px solid var(--border); border-radius: 4px;
    background: none; cursor: pointer; transition: all 0.15s;
  }
  .report-dispute-btn:hover { color: #c33; border-color: #c33; }

  /* Book tags */
  .book-tags {
    display: flex; flex-wrap: wrap; gap: 6px; margin: 8px 0 4px;
  }
  .tag-badge {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 8px; border-radius: 12px; font-size: 12px;
    text-decoration: none; transition: opacity 0.15s;
  }
  .tag-badge.teaches { background: var(--accent-light, #e8f4fd); color: var(--accent); }
  .tag-badge.prereq { background: #fdf2e8; color: #b06000; }
  .tag-badge.sm { font-size: 11px; padding: 1px 6px; }
  .tag-badge:hover { opacity: 0.8; }
  .tag-remove {
    background: none; border: none; padding: 0; cursor: pointer;
    font-size: 13px; line-height: 1; color: inherit; opacity: 0.6;
  }
  .tag-remove:hover { opacity: 1; }
  .prereq-type-label { font-size: 10px; opacity: 0.75; }

  /* Chapters header row */
  .chapters-header {
    display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;
  }
  .chapters-header h2 { margin: 0; }
  .add-chapter-btn {
    font-size: 12px; padding: 4px 10px;
    border: 1px solid var(--border); border-radius: 4px;
    background: none; cursor: pointer; color: var(--text-secondary);
    transition: all 0.15s;
  }
  .add-chapter-btn:hover { color: var(--accent); border-color: var(--accent); }

  /* Add chapter form */
  .chapter-form {
    padding: 12px; margin-bottom: 12px;
    border: 1px solid var(--border); border-radius: 6px;
    background: var(--bg-card, var(--bg));
  }
  .chapter-form h4 { margin: 0 0 8px; font-size: 14px; }
  .chapter-input {
    width: 100%; padding: 6px 8px; margin-bottom: 8px;
    border: 1px solid var(--border); border-radius: 4px;
    font-size: 13px; background: var(--bg); color: var(--text);
    box-sizing: border-box;
  }
  .chapter-input:focus { outline: none; border-color: var(--accent); }

  /* Tag editor */
  .tag-editor { margin-top: 8px; }
  .tag-editor-label { font-size: 12px; font-weight: 500; margin-bottom: 4px; color: var(--text-secondary); }
  .tag-list { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 4px; }
  .tag-input-wrap { position: relative; display: flex; gap: 4px; align-items: center; }
  .tag-input {
    flex: 1; padding: 4px 6px;
    border: 1px solid var(--border); border-radius: 4px;
    font-size: 12px; background: var(--bg); color: var(--text);
  }
  .tag-suggestions {
    position: absolute; top: 100%; left: 0; z-index: 10; list-style: none;
    margin: 0; padding: 4px 0; min-width: 160px;
    background: var(--bg); border: 1px solid var(--border); border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  }
  .tag-suggestions li button {
    display: block; width: 100%; text-align: left;
    padding: 4px 10px; font-size: 12px;
    background: none; border: none; cursor: pointer; color: var(--text);
  }
  .tag-suggestions li button:hover { background: var(--bg-hover, #f5f5f5); }
  .tag-add-btn {
    font-size: 12px; padding: 3px 8px;
    border: 1px solid var(--accent); border-radius: 4px;
    background: none; cursor: pointer; color: var(--accent); white-space: nowrap;
  }
  .prereq-type-select {
    padding: 4px 6px; font-size: 12px;
    border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg); color: var(--text);
  }

  /* Per-chapter tag editor */
  .chapter-edit-tags-btn, .chapter-delete-btn {
    font-size: 11px; padding: 2px 7px;
    border: 1px solid var(--border); border-radius: 3px;
    background: none; cursor: pointer; color: var(--text-hint);
    transition: all 0.15s; white-space: nowrap;
    opacity: 0;
    flex-shrink: 0;
  }
  .chapter-edit-tags-btn:hover { color: var(--accent); border-color: var(--accent); }
  .chapter-delete-btn:hover { color: #c33; border-color: #c33; }
  .chapter-tag-editor {
    padding: 10px 12px 12px;
    border: 1px solid var(--border); border-radius: 6px;
    background: var(--bg-card, var(--bg));
    margin: 4px 0 8px;
  }
  .chapter-tag-editor.sub { margin-left: 24px; }
  .chapter-tag-editor-actions {
    display: flex; gap: 8px; margin-top: 10px; justify-content: flex-end;
  }

  /* Chapters */
  .chapters-section {
    margin-bottom: 2rem;
  }
  .chapters-section h2 {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 400;
    margin: 0 0 12px;
  }
  .chapter-item {
    border-bottom: 1px solid var(--border);
  }
  .chapter-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    font-size: 14px;
  }
  .chapter-row:hover .chapter-edit-tags-btn,
  .chapter-row:hover .chapter-delete-btn {
    opacity: 1;
  }
  .chapter-row.sub {
    padding-left: 24px;
    font-size: 13px;
  }
  .chapter-check {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1.5px solid var(--border);
    background: none;
    cursor: pointer;
    padding: 0;
    transition: border-color 0.15s, background 0.15s;
    position: relative;
  }
  .chapter-check:hover {
    border-color: var(--accent);
  }
  .chapter-check.done {
    background: var(--accent);
    border-color: var(--accent);
  }
  .chapter-check.done::after {
    content: '';
    position: absolute;
    left: 4px;
    top: 1px;
    width: 5px;
    height: 9px;
    border: 2px solid white;
    border-top: none;
    border-left: none;
    transform: rotate(45deg);
  }
  .chapter-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .chapter-tag-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chapter-title {
    color: var(--text-primary);
    text-decoration: none;
  }
  a.chapter-title:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .chapter-children {
    border-left: 2px solid var(--border);
    margin-left: 8px;
  }

  /* Reviews */
  .reviews-section h2 {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 400;
    margin: 0 0 12px;
  }
  .empty {
    color: var(--text-hint);
    font-size: 14px;
  }
  .review-wrapper {
    position: relative;
  }
  .review-edition {
    position: absolute;
    top: 8px;
    right: 8px;
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-gray, #f5f5f5);
    padding: 2px 8px;
    border-radius: 3px;
    z-index: 1;
  }
  .edition-select { font-size: 12px; padding: 4px 8px; border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white); color: var(--text-secondary); }
  .set-cover-btn {
    font-size: 11px;
    color: var(--accent);
    background: none;
    border: 1px dashed var(--accent);
    border-radius: 3px;
    padding: 2px 8px;
    cursor: pointer;
    margin-top: 4px;
  }
  .set-cover-btn:hover { background: var(--accent); color: white; }

  /* Q&A Section */
  .qa-section { margin-top: 2rem; }
  .qa-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .qa-header h2 { font-family: var(--font-serif); font-size: 1.2rem; font-weight: 400; margin: 0; }
  .ask-btn { padding: 4px 14px; font-size: 13px; border: 1px solid var(--accent); border-radius: 3px; background: none; color: var(--accent); cursor: pointer; }
  .ask-btn:hover { background: var(--accent); color: white; }

  .ask-form { margin-bottom: 16px; }
  .ask-title { display: block; width: 100%; padding: 8px 10px; font-size: 14px; border: 1px solid var(--border); border-radius: 4px 4px 0 0; font-family: var(--font-sans); border-bottom: none; }
  .ask-body { display: block; width: 100%; padding: 8px 10px; font-size: 13px; border: 1px solid var(--border); border-radius: 0; font-family: var(--font-sans); resize: vertical; border-bottom: none; }
  .ask-title:focus, .ask-body:focus { outline: none; border-color: var(--accent); }
  .ask-bar { display: flex; gap: 6px; align-items: center; padding: 6px 8px; border: 1px solid var(--border); border-radius: 0 0 4px 4px; background: var(--bg-page); }
  .format-select { padding: 3px 6px; font-size: 11px; border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white); }
  .btn-cancel { padding: 4px 10px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; margin-left: auto; }
  .btn-submit { padding: 4px 12px; font-size: 12px; border: none; border-radius: 3px; background: var(--accent); color: white; cursor: pointer; }
  .btn-submit:disabled { opacity: 0.5; cursor: not-allowed; }

  .question-list { display: flex; flex-direction: column; gap: 6px; }
  .q-card { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 6px; text-decoration: none; color: inherit; transition: border-color 0.15s; }
  .q-card:hover { border-color: var(--accent); text-decoration: none; }
  .q-card-main { flex: 1; min-width: 0; }
  .q-card-title { display: block; font-size: 14px; color: var(--text-primary); font-family: var(--font-serif); }
  .q-card:hover .q-card-title { color: var(--accent); }
  .q-card-meta { display: block; font-size: 12px; color: var(--text-hint); margin-top: 2px; }
  .q-card-answers { text-align: center; flex-shrink: 0; }
  .answer-count { display: block; font-size: 18px; font-weight: 600; color: var(--text-hint); }
  .answer-count.has-answers { color: var(--accent); }
  .answer-label { display: block; font-size: 10px; color: var(--text-hint); }
</style>
