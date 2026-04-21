<script lang="ts">
  import { getBook, updateBook, updateBookEdition, uploadEditionCover, getBookEditHistory, rateBook, unrateBook, setReadingStatus, removeReadingStatus, setChapterProgress, createChapter, deleteChapter, updateChapterTags, searchTags, lookupTag, getQuestionsByBook, createQuestion, setPreferredEdition, listBookResources, addBookResource, deleteBookResource, listBookShortReviews, upsertBookShortReview, deleteBookShortReview } from '../lib/api';
  import type { BookResource } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';
  import PostCard from '../lib/components/PostCard.svelte';
  import ShortReviewComposer from '../lib/components/ShortReviewComposer.svelte';
  import ShortReviewCard from '../lib/components/ShortReviewCard.svelte';
  import type { BookDetail, BookEdition, BookChapter, ChapterPrereqEntry, BookShortReview } from '../lib/types';

  let { id } = $props<{ id: string }>();

  let locale = $derived(getLocale());

  /** Resolve a localized field (Record<string, string>) to the current locale with fallback. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const l = getLocale();
    return field[l] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  let detail = $state<BookDetail | null>(null);
  let resources = $state<BookResource[]>([]);
  let showAddResource = $state(false);
  let newResKind = $state('other');
  let newResLabel = $state('');
  let newResUrl = $state('');

  async function submitResource() {
    if (!newResLabel.trim() || !newResUrl.trim()) return;
    await addBookResource(id, { kind: newResKind, label: newResLabel.trim(), url: newResUrl.trim() });
    newResLabel = ''; newResUrl = ''; showAddResource = false;
    resources = await listBookResources(id);
  }

  async function removeResource(rid: string) {
    await deleteBookResource(id, rid);
    resources = await listBookResources(id);
  }

  /** Resolve a localized field for a specific language, with fallback to en then any. */
  function locFor(field: Record<string, string> | null | undefined, lang: string): string {
    if (!field) return '';
    if (lang in field) return field[lang];
    return field['en'] || Object.values(field).find(v => v) || '';
  }

  /** Build edition card title. */
  function editionFullTitle(ed: BookEdition): string {
    const t = ed.title;
    const st = ed.subtitle || '';
    const full = st ? `${t}: ${st}` : t;
    return ed.edition_name ? `${full} (${ed.edition_name})` : full;
  }

  const RESOURCE_KINDS = ['solutions', 'exercises', 'video', 'slides', 'errata', 'code', 'other'];

  function resourceKindLabel(kind: string): string {
    return t(`books.resourceKind.${kind}`) || kind;
  }

  let bookLevelResources = $derived(resources.filter(r => !r.edition_id));
  let groupedResources = $derived(
    bookLevelResources.reduce((acc, r) => { (acc[r.kind] = acc[r.kind] || []).push(r); return acc; }, {} as Record<string, BookResource[]>)
  );
  let loading = $state(true);

  // Rating state
  let hoverRating = $state(0);
  let myRating = $state(0);
  let avgRating = $state(0);
  let ratingCount = $state(0);

  // Short reviews
  let bookShortReviews = $state<BookShortReview[]>([]);
  let showBookShortReviewComposer = $state(false);

  // Reading status state
  let readingStatus = $state('');
  let readingProgress = $state(0);

  // Chapter progress (reactive, keyed by chapter_id)
  let chapterDone = $state(new Map<string, boolean>());

  let totalChapters = $derived(detail?.chapters?.length ?? 0);
  let doneChapters = $derived(Array.from(chapterDone.values()).filter(v => v).length);

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
  import { authorName, authorDisplayName } from '../lib/display';
  import { tagStore } from '../lib/tagStore.svelte';

  $effect(() => { tagStore.ensure(); });
  const localTag = (id: string) => tagStore.localize(id);
  import type { Article, ContentFormat } from '../lib/types';
  let bookQuestions = $state<Article[]>([]);
  let showAskForm = $state(false);
  let askTitle = $state('');
  let askContent = $state('');
  let askFormat = $state<ContentFormat>('markdown');
  let askSubmitting = $state(false);

  async function toggleChapter(chapterId: string) {
    if (!detail) return;
    const next = !chapterDone.get(chapterId);

    // Collect every descendant id so the UI mirrors the backend cascade.
    const descendantIds: string[] = [];
    const walk = (parent: string) => {
      for (const c of detail!.chapters) {
        if (c.parent_id === parent) {
          descendantIds.push(c.id);
          walk(c.id);
        }
      }
    };
    walk(chapterId);
    const touched = [chapterId, ...descendantIds];
    const prior = new Map(touched.map(id => [id, chapterDone.get(id) ?? false]));

    for (const cid of touched) chapterDone.set(cid, next);
    chapterDone = new Map(chapterDone);

    try {
      const status = await setChapterProgress(id, chapterId, next);
      if (status) {
        readingStatus = status.status;
        readingProgress = status.progress;
      }
    } catch {
      for (const [cid, was] of prior) chapterDone.set(cid, was);
      chapterDone = new Map(chapterDone);
    }
  }

  // Edit history
  interface EditLog { id: string; editor_did: string; editor_handle: string | null; old_data: Record<string, any>; new_data: Record<string, any>; summary: string; created_at: string; }
  let editHistory = $state<EditLog[]>([]);
  let selectedLog = $state<EditLog | null>(null);

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
      bookShortReviews = detail.recent_short_reviews;
      getBookEditHistory(id).then(h => { editHistory = h; }).catch(() => {});
      getQuestionsByBook(id).then(qs => { bookQuestions = qs; }).catch(() => {});
      listBookResources(id).then(r => { resources = r; }).catch(() => {});
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
    return val.toFixed(1);
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

  async function clearRating() {
    if (!getAuth()) return;
    try {
      const stats = await unrateBook(id);
      myRating = 0;
      avgRating = stats.avg_rating;
      ratingCount = stats.rating_count;
    } catch { /* */ }
  }

  let myBookShortReview = $derived(
    getAuth() ? bookShortReviews.find(r => r.did === getAuth()!.did) || null : null
  );

  async function handleBookShortReviewSubmit(body: string) {
    await upsertBookShortReview(id, { body });
    bookShortReviews = await listBookShortReviews(id);
    showBookShortReviewComposer = false;
  }

  async function handleDeleteBookShortReview() {
    await deleteBookShortReview(id);
    bookShortReviews = await listBookShortReviews(id);
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

  // Edit modal state
  let showEdit = $state(false);
  let editTitles = $state<Record<string, string>>({});
  let editAbbreviation = $state('');
  let editDescs = $state<Record<string, string>>({});
  let editSubtitles = $state<Record<string, string>>({});
  let editSummary = $state('');
  let editSaving = $state(false);
  let editError = $state('');
  let editLang = $state('en');
  let editAuthorsInput = $state('');
  let editBookTeaches = $state<string[]>([]);
  let editBookPrereqs = $state<string[]>([]);
  let editBookTopics = $state<string[]>([]);
  let editBookTagInput = $state('');
  let editBookTagSuggestions = $state<{id:string,name:string}[]>([]);
  let editBookPrereqInput = $state('');
  let editBookPrereqSuggestions = $state<{id:string,name:string}[]>([]);
  let editBookTopicInput = $state('');
  let editBookTopicSuggestions = $state<{id:string,name:string}[]>([]);

  let editBookTagTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(editBookTagTimeout);
    const q = editBookTagInput.trim();
    if (!q) { editBookTagSuggestions = []; return; }
    editBookTagTimeout = setTimeout(async () => {
      editBookTagSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });
  let editBookPrereqTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(editBookPrereqTimeout);
    const q = editBookPrereqInput.trim();
    if (!q) { editBookPrereqSuggestions = []; return; }
    editBookPrereqTimeout = setTimeout(async () => {
      editBookPrereqSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });
  /**
   * Append a tag_id to the given slot. Input may already be a tag_id
   * (from a suggestion's `tag_id` field) or a label the user typed.
   * For typed labels we LOOK UP only — never silently create — so an
   * unknown name surfaces an error nudging the user to the hierarchy
   * page to mint the concept with a proper parent first. Editor
   * state only ever contains tag_ids.
   */
  async function appendTagId(slot: 'teaches' | 'prereqs' | 'topics', input: string) {
    const s = input.trim();
    if (!s) return;
    let tagId: string;
    if (s.startsWith('tg-')) {
      tagId = s;
    } else {
      try {
        const res = await lookupTag(s);
        tagId = res.tag_id;
      } catch {
        editError = t('books.tagNotFound').replace('{name}', s);
        return;
      }
    }
    editError = '';
    if (slot === 'teaches') {
      if (!editBookTeaches.includes(tagId)) editBookTeaches = [...editBookTeaches, tagId];
      editBookTagInput = ''; editBookTagSuggestions = [];
    } else if (slot === 'prereqs') {
      if (!editBookPrereqs.includes(tagId)) editBookPrereqs = [...editBookPrereqs, tagId];
      editBookPrereqInput = ''; editBookPrereqSuggestions = [];
    } else {
      if (!editBookTopics.includes(tagId)) editBookTopics = [...editBookTopics, tagId];
      editBookTopicInput = ''; editBookTopicSuggestions = [];
    }
  }
  function removeBookTag(tagId: string) {
    editBookTeaches = editBookTeaches.filter(t => t !== tagId);
  }
  function removeBookPrereq(tagId: string) {
    editBookPrereqs = editBookPrereqs.filter(t => t !== tagId);
  }

  let editBookTopicTimeout: ReturnType<typeof setTimeout>;
  $effect(() => {
    clearTimeout(editBookTopicTimeout);
    const q = editBookTopicInput.trim();
    if (!q) { editBookTopicSuggestions = []; return; }
    editBookTopicTimeout = setTimeout(async () => {
      editBookTopicSuggestions = await searchTags(q).catch(() => []);
    }, 150);
  });
  function removeBookTopic(tagId: string) {
    editBookTopics = editBookTopics.filter(t => t !== tagId);
  }

  // Edition edit modal state
  let showEditionEdit = $state(false);
  let editionEditId = $state('');
  let editionEditName = $state('');
  let editionEditTitle = $state('');
  let editionEditSubtitle = $state('');
  let editionEditLang = $state('en');
  let editionEditIsbn = $state('');
  let editionEditPublisher = $state('');
  let editionEditYear = $state('');
  let editionEditTranslators = $state('');
  let editionEditLinks = $state('');
  let editionEditSaving = $state(false);
  let editionEditError = $state('');
  let uploadingCoverId = $state('');
  let coverErrorId = $state('');
  let coverErrorMsg = $state('');

  async function handleEditionCoverUpload(editionId: string, file: File) {
    coverErrorId = '';
    coverErrorMsg = '';
    if (file.size > 5 * 1024 * 1024) {
      coverErrorId = editionId; coverErrorMsg = t('books.coverTooLarge'); return;
    }
    if (!/\.(jpe?g|png|webp)$/i.test(file.name)) {
      coverErrorId = editionId; coverErrorMsg = t('books.coverWrongType'); return;
    }
    uploadingCoverId = editionId;
    try {
      await uploadEditionCover(id, editionId, file);
      await load();
    } catch (e: any) {
      coverErrorId = editionId;
      coverErrorMsg = e.message || 'Upload failed';
    } finally {
      uploadingCoverId = '';
    }
  }

  function openEditionEdit(ed: BookEdition) {
    editionEditId = ed.id;
    editionEditName = ed.edition_name || '';
    editionEditTitle = ed.title || '';
    editionEditSubtitle = ed.subtitle || '';
    editionEditLang = ed.lang;
    editionEditIsbn = ed.isbn || '';
    editionEditPublisher = ed.publisher || '';
    editionEditYear = ed.year || '';
    editionEditTranslators = ed.translators.join(', ');
    editionEditLinks = (ed.purchase_links as any[]).map((l: any) => `${l.label}:${l.url}`).join('\n');
    editionEditError = '';
    showEditionEdit = true;
  }

  async function saveEditionEdit() {
    if (!editionEditTitle.trim()) { editionEditError = 'Title is required'; return; }
    editionEditSaving = true;
    editionEditError = '';
    try {
      const translators = editionEditTranslators.trim() ? editionEditTranslators.split(',').map(s => s.trim()).filter(Boolean) : [];
      const purchase_links = editionEditLinks.trim() ? editionEditLinks.split('\n').map(line => {
        const [label, ...rest] = line.split(':');
        return { label: label.trim(), url: rest.join(':').trim() };
      }).filter(l => l.label && l.url) : [];
      await updateBookEdition(id, editionEditId, {
        edition_name: editionEditName.trim() || undefined,
        title: editionEditTitle.trim(),
        subtitle: editionEditSubtitle || undefined,
        lang: editionEditLang,
        isbn: editionEditIsbn || undefined,
        publisher: editionEditPublisher || undefined,
        year: editionEditYear || undefined,
        translators,
        purchase_links,
      });
      showEditionEdit = false;
      await load();
    } catch (e: any) {
      editionEditError = e.message || 'Save failed';
    } finally {
      editionEditSaving = false;
    }
  }

  const EDIT_LANGS = [
    { code: 'en', label: 'English' },
    { code: 'zh', label: '中文' },
    { code: 'fr', label: 'Français' },
  ];

  function openEdit() {
    if (!detail) return;
    // Ensure every language has an entry (even if empty) so bind:value works
    const titles: Record<string, string> = {};
    const subs: Record<string, string> = {};
    const descs: Record<string, string> = {};
    for (const lang of EDIT_LANGS) {
      titles[lang.code] = (detail.book.title as Record<string, string>)[lang.code] || '';
      subs[lang.code] = (detail.book.subtitle as Record<string, string> | null | undefined || {})[lang.code] || '';
      descs[lang.code] = (detail.book.description as Record<string, string> || {})[lang.code] || '';
    }
    editTitles = titles;
    editSubtitles = subs;
    editDescs = descs;
    editAbbreviation = detail.book.abbreviation || '';
    editLang = getLocale();
    editSummary = '';
    editError = '';
    editAuthorsInput = (detail.linked_authors.length > 0
      ? detail.linked_authors.map(a => a.name)
      : detail.book.authors).join(', ');
    // Edit state holds canonical tag_ids throughout; display chips use
    // tagStore.localize() to show the user-locale label.
    editBookTeaches = [...detail.tags];
    editBookPrereqs = [...detail.prereqs];
    // detail.topics = derived ∪ explicit (display set); the editor
    // only manages the explicit ones — derived topics auto-appear
    // from teach-tag ancestors and shouldn't be persisted as rows.
    editBookTopics = [...detail.explicit_topics];
    editBookTagInput = '';
    editBookPrereqInput = '';
    editBookTopicInput = '';
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
      const subtitle = Object.fromEntries(Object.entries(editSubtitles).filter(([_, v]) => v.trim()));
      const description = Object.fromEntries(Object.entries(editDescs).filter(([_, v]) => v.trim()));
      const authors = editAuthorsInput.split(',').map(s => s.trim()).filter(Boolean);
      await updateBook(id, {
        title,
        subtitle,
        description,
        abbreviation: editAbbreviation.trim(),
        authors,
        tags: editBookTeaches,
        prereqs: editBookPrereqs,
        topics: editBookTopics,
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
  let newChapterTitles = $state<Record<string, string>>({});
  let chapterTitleLang = $state('en');
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
    // Use first non-empty title as the primary title
    const primaryTitle = newChapterTitles[chapterTitleLang]?.trim() || Object.values(newChapterTitles).find(v => v.trim()) || '';
    if (!primaryTitle) return;
    chapterSaving = true;
    // Build title_i18n: only include non-empty entries
    const titleI18n: Record<string, string> = {};
    for (const [lang, val] of Object.entries(newChapterTitles)) {
      if (val.trim()) titleI18n[lang] = val.trim();
    }
    try {
      const rootChapters = detail?.chapters.filter(c => !c.parent_id) ?? [];
      await createChapter(id, {
        title: primaryTitle,
        title_i18n: titleI18n,
        parent_id: newChapterParentId || undefined,
        order_index: rootChapters.length,
        article_uri: newChapterArticleUri.trim() || undefined,
        teaches: newChapterTeaches,
        prereqs: newChapterPrereqs,
      } as any);
      newChapterTitles = {};
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
          <h1>
            {loc(detail.book.title)}
            {#if detail.book.abbreviation}<span class="book-abbr" title={t('books.abbreviationLabel')}>{detail.book.abbreviation}</span>{/if}
          </h1>
          {#if loc(detail.book.subtitle)}
            <p class="book-subtitle">{loc(detail.book.subtitle)}</p>
          {/if}
          <p class="authors">
            {#if detail.linked_authors.length > 0}
              {#each detail.linked_authors as a, i}
                <a href="/author?id={encodeURIComponent(a.id)}">{authorDisplayName(a)}</a>{#if i < detail.linked_authors.length - 1}, {/if}
              {/each}
            {:else}
              {detail.book.authors.join(', ')}
            {/if}
          </p>
          {#if loc(detail.book.description)}
            <p class="description">{loc(detail.book.description)}</p>
          {/if}

          <!-- Tags -->
          {#if detail.topics.length > 0}
            <div class="book-tag-row">
              <span class="book-tag-row-label" title={t('books.topicTooltip')}>{t('books.topicRowLabel')}</span>
              {#each detail.topics as topic}
                <a href="/tag?id={encodeURIComponent(topic)}" class="tag-badge topic">{localTag(topic)}</a>
              {/each}
            </div>
          {/if}
          {#if detail.tags.length > 0}
            <div class="book-tag-row">
              <span class="book-tag-row-label">{t('books.teachesRowLabel')}</span>
              {#each detail.tags as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches">{localTag(tag)}</a>
              {/each}
            </div>
          {/if}
          {#if detail.prereqs.length > 0}
            <div class="book-tag-row">
              <span class="book-tag-row-label">{t('books.prereqRowLabel')}</span>
              {#each detail.prereqs as prereq}
                <a href="/tag?id={encodeURIComponent(prereq)}" class="tag-badge prereq">{localTag(prereq)}</a>
              {/each}
            </div>
          {/if}
          {#if detail.series_badges.length > 0}
            <div class="book-tag-row">
              <span class="book-tag-row-label">{t('books.seriesBadges')}</span>
              {#each detail.series_badges as badge}
                <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                <span class="tag-badge series-badge" onclick={() => navigate(`/book-series-detail?id=${encodeURIComponent(badge.id)}`)}>{loc(badge.title)} #{badge.position}</span>
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
                <button class="clear-rating" onclick={clearRating} title={t('books.clearRating')}>×</button>
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
                    <option value={ed.id}>{editionFullTitle(ed)} ({ed.lang}{ed.year ? `, ${ed.year}` : ''})</option>
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

          <!-- Chapter-based progress readout for in-progress / dropped states -->
          {#if (readingStatus === 'reading' || readingStatus === 'dropped') && totalChapters > 0}
            <div class="progress-section">
              <div class="progress-readout">
                <span>{t('books.progress')}: {doneChapters} / {totalChapters} {t('books.chaptersDone')}</span>
                <span class="progress-pct">{Math.round((doneChapters / totalChapters) * 100)}%</span>
              </div>
              <div class="progress-bar">
                <div class="progress-fill" style="width: {(doneChapters / totalChapters) * 100}%"></div>
              </div>
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
            <div class="lang-tabs">
              {#each EDIT_LANGS as lang}
                <button class="lang-tab" class:active={chapterTitleLang === lang.code} onclick={() => chapterTitleLang = lang.code}>
                  {lang.label}
                  {#if newChapterTitles[lang.code]?.trim()}<span class="lang-dot"></span>{/if}
                </button>
              {/each}
            </div>
            <input class="chapter-input" bind:value={newChapterTitles[chapterTitleLang]} placeholder={chapterTitleLang === 'zh' ? '章节标题' : 'Chapter title'} />
            <input class="chapter-input" bind:value={newChapterArticleUri} placeholder="关联文章 URI（可选）" />
            <select class="chapter-input" bind:value={newChapterParentId}>
              <option value="">顶层章节</option>
              {#each (detail?.chapters ?? []).filter(c => !c.parent_id) as ch}
                <option value={ch.id}>{loc(ch.title_i18n) || ch.title}</option>
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

            <button class="action-btn primary" onclick={submitNewChapter} disabled={chapterSaving || !Object.values(newChapterTitles).some(v => v.trim())}>
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
                    <a href="/article?uri={encodeURIComponent(ch.article_uri)}" class="chapter-title">{loc(ch.title_i18n) || ch.title}</a>
                  {:else}
                    <span class="chapter-title">{loc(ch.title_i18n) || ch.title}</span>
                  {/if}
                  <!-- Chapter tags display -->
                  {#if ch.teaches.length > 0 || ch.prereqs.length > 0}
                    <div class="chapter-tag-row">
                      {#each ch.teaches as tag}
                        <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches sm">{localTag(tag)}</a>
                      {/each}
                      {#each ch.prereqs as p}
                        <span class="tag-badge prereq sm" title="{p.prereq_type === 'required' ? '必须前置' : '推荐前置'}">{localTag(p.tag_id)}</span>
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
                              <a href="/tag?id={encodeURIComponent(tag)}" class="tag-badge teaches sm">{localTag(tag)}</a>
                            {/each}
                            {#each sub.prereqs as p}
                              <span class="tag-badge prereq sm">{localTag(p.tag_id)}</span>
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

      <!-- Short reviews -->
      <div class="reviews-section">
        <div class="section-header">
          <h2>{t('books.shortReviews')} ({bookShortReviews.length})</h2>
          {#if getAuth() && !myBookShortReview}
            <button class="write-btn" onclick={() => { showBookShortReviewComposer = !showBookShortReviewComposer; }}>
              {t('books.writeShortReview')}
            </button>
          {/if}
        </div>
        {#if showBookShortReviewComposer}
          <div class="composer-wrap">
            <ShortReviewComposer
              onSubmit={handleBookShortReviewSubmit}
              initialBody={myBookShortReview?.body || ''}
              placeholder={t('books.shortReviewPlaceholder')}
            />
          </div>
        {/if}
        {#if myBookShortReview}
          <div class="my-review-section">
            <ShortReviewCard review={myBookShortReview} onDelete={handleDeleteBookShortReview} />
            <button class="edit-btn" onclick={() => { showBookShortReviewComposer = !showBookShortReviewComposer; }}>
              {t('books.editShortReview')}
            </button>
          </div>
        {/if}
        {#each bookShortReviews.filter(r => !myBookShortReview || r.id !== myBookShortReview.id) as review}
          <ShortReviewCard {review} />
        {:else}
          {#if !myBookShortReview}<p class="empty">{t('books.noShortReviews')}</p>{/if}
        {/each}
      </div>

      <!-- Reviews (opinions on the book) -->
      <div class="reviews-section">
        <h2>{t('books.reviews')}</h2>
        {#if detail.reviews.length === 0}
          <p class="empty">{t('books.noReviews')}</p>
        {:else}
          {#each detail.reviews as review}
            <div class="review-wrapper">
              {#if review.book_chapter_id}
                {@const ch = detail.chapters.find(c => c.id === review.book_chapter_id)}
                {#if ch}<span class="review-edition">{t('books.onChapter') || 'Chapter'}: {ch.title}</span>{/if}
              {:else if review.edition_id}
                {@const ed = detail.editions.find(e => e.id === review.edition_id)}
                {#if ed}<span class="review-edition">{editionFullTitle(ed)}</span>{/if}
              {/if}
              <PostCard article={review} articleTeaches={[]} />
            </div>
          {/each}
        {/if}
      </div>

      <!-- Notes (reader thoughts / knowledge supplements) -->
      <div class="reviews-section">
        <h2>{t('books.notes') || 'Notes'}</h2>
        {#if detail.notes.length === 0}
          <p class="empty">{t('books.noNotes') || 'No notes yet.'}</p>
        {:else}
          {#each detail.notes as note}
            <div class="review-wrapper">
              {#if note.book_chapter_id}
                {@const ch = detail.chapters.find(c => c.id === note.book_chapter_id)}
                {#if ch}<span class="review-edition">{t('books.onChapter') || 'Chapter'}: {ch.title}</span>{/if}
              {/if}
              <PostCard article={note} articleTeaches={[]} />
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
            <img
              src={ed.cover_url}
              alt=""
              class="edition-cover"
              onerror={(e) => {
                const img = e.currentTarget as HTMLImageElement;
                img.classList.add('broken');
                // Swap into an explicit error slot so the problem is visible
                // instead of showing a stray alt string in the empty image box.
                const slot = img.nextElementSibling as HTMLElement | null;
                if (slot?.classList.contains('edition-cover-error')) slot.hidden = false;
                img.hidden = true;
                console.error('edition cover failed to load', { edition: ed.id, url: ed.cover_url });
              }}
            />
            <div class="edition-cover-error" hidden>{t('books.coverLoadError') || 'Cover failed to load'}: {ed.cover_url}</div>
          {/if}
          <div class="edition-top">
            <strong>{editionFullTitle(ed)}</strong>
            <span class="edition-lang">{langLabel(ed.lang)}</span>
            {#if (ed as any).status === 'draft'}
              <span class="edition-draft-badge">{t('books.editionDraft')}</span>
            {/if}
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
          {#each resources.filter(r => r.edition_id === ed.id) as r}
            <div class="resource-row">
              <a href={r.url} target="_blank" rel="noopener" class="purchase-link">{r.label}</a>
              {#if getAuth()}
                <button class="resource-del" onclick={() => removeResource(r.id)} title="Delete">×</button>
              {/if}
            </div>
          {/each}
          {#if getAuth()}
            <div class="edition-actions">
              <button class="set-cover-btn" onclick={() => openEditionEdit(ed)}>✎</button>
              <label class="set-cover-btn" class:disabled={uploadingCoverId === ed.id}>
                {uploadingCoverId === ed.id ? t('books.uploadingCover') : t('books.uploadCover')}
                <input type="file" accept="image/jpeg,image/png,image/webp" hidden onchange={async (e) => {
                  const input = e.target as HTMLInputElement;
                  const file = input.files?.[0];
                  if (file) await handleEditionCoverUpload(ed.id, file);
                  input.value = '';
                }} />
              </label>
              {#if ed.cover_url}
                <button class="set-cover-btn" onclick={async () => {
                  await setPreferredEdition(id, ed.id);
                  await load();
                }}>{t('books.setAsCover') || 'Use this cover'}</button>
              {/if}
            </div>
            {#if coverErrorId === ed.id}
              <p class="error-msg">{coverErrorMsg}</p>
            {/if}
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

      <!-- Supplementary Resources -->
      {#if bookLevelResources.length > 0 || getAuth()}
        <div class="resources-section">
          <h3>{t('books.resources')}</h3>
          {#each Object.entries(groupedResources) as [kind, items]}
            <div class="resource-group">
              <h4 class="resource-kind">{resourceKindLabel(kind)}</h4>
              {#each items as r}
                <div class="resource-row">
                  <a href={r.url} target="_blank" rel="noopener" class="resource-link">{r.label}</a>
                  {#if getAuth()}
                    <button class="resource-del" onclick={() => removeResource(r.id)} title="Delete">×</button>
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
          {#if getAuth()}
            {#if showAddResource}
              <div class="resource-form">
                <select bind:value={newResKind} class="res-input">
                  {#each RESOURCE_KINDS as k}
                    <option value={k}>{resourceKindLabel(k)}</option>
                  {/each}
                </select>
                <input class="res-input" bind:value={newResLabel} placeholder="Label" />
                <input class="res-input" bind:value={newResUrl} placeholder="URL" />
                <div class="res-form-actions">
                  <button class="btn btn-primary btn-sm" onclick={submitResource} disabled={!newResLabel.trim() || !newResUrl.trim()}>OK</button>
                  <button class="btn btn-secondary btn-sm" onclick={() => showAddResource = false}>{t('books.cancel')}</button>
                </div>
              </div>
            {:else}
              <button class="add-resource-btn" onclick={() => showAddResource = true}>{t('books.addResource')}</button>
            {/if}
          {/if}
        </div>
      {/if}

      <!-- Edit history -->
      <div class="edit-history">
        <h3>{t('books.editHistory')}</h3>
        {#if editHistory.length === 0}
          <p class="empty-hint">{t('books.noEditHistory')}</p>
        {:else}
          {#each editHistory.slice(0, 10) as log}
            <button class="edit-log" onclick={() => selectedLog = log}>
              <span class="edit-log-who">{log.editor_handle ? `@${log.editor_handle}` : log.editor_did.slice(0, 20)}</span>
              <span class="edit-log-summary">{log.summary || '—'}</span>
              <span class="edit-log-time">{new Date(log.created_at).toLocaleDateString()}</span>
            </button>
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
          <label>{t('books.subtitleLabel')} ({editLang})</label>
          <input bind:value={editSubtitles[editLang]} placeholder={editSubtitles['en'] || ''} />
        </div>
        <div class="form-group">
          <label>{t('books.descriptionLabel')} ({editLang})</label>
          <textarea bind:value={editDescs[editLang]} rows="3" placeholder={editDescs['en'] || ''}></textarea>
        </div>
        <div class="form-group">
          <label>{t('books.abbreviationLabel')}</label>
          <input bind:value={editAbbreviation} placeholder="e.g. CLRS, SICP, LADR" maxlength="50" />
        </div>

        <div class="form-group">
          <label>{t('books.authorsLabel')}</label>
          <input bind:value={editAuthorsInput} placeholder={t('books.authorsPlaceholder')} />
        </div>

        <div class="form-group">
          <label>{t('books.tagsLabel')}</label>
          <div class="tag-chip-row">
            {#each editBookTeaches as tag}
              <span class="tag-chip">{localTag(tag)} <button type="button" onclick={() => removeBookTag(tag)}>×</button></span>
            {/each}
          </div>
          <div class="tag-input-wrap">
            <input class="tag-input" bind:value={editBookTagInput} placeholder={t('books.tagsPlaceholder')} onkeydown={(e) => { if (e.key === 'Enter' && editBookTagInput.trim()) { e.preventDefault(); appendTagId('teaches', editBookTagInput); } }} />
            {#if editBookTagSuggestions.length > 0}
              <ul class="tag-suggestions">
                {#each editBookTagSuggestions as s}
                  <li><button type="button" onclick={() => appendTagId('teaches', s.tag_id)}>{s.name || s.id}</button></li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>

        <div class="form-group">
          <label>{t('books.topicsLabel')}</label>
          <div class="form-hint">{t('books.topicsHint')}</div>
          <div class="tag-chip-row">
            {#each editBookTopics as tag}
              <span class="tag-chip topic">{localTag(tag)} <button type="button" onclick={() => removeBookTopic(tag)}>×</button></span>
            {/each}
          </div>
          <div class="tag-input-wrap">
            <input class="tag-input" bind:value={editBookTopicInput} placeholder={t('books.topicsPlaceholder')} onkeydown={(e) => { if (e.key === 'Enter' && editBookTopicInput.trim()) { e.preventDefault(); appendTagId('topics', editBookTopicInput); } }} />
            {#if editBookTopicSuggestions.length > 0}
              <ul class="tag-suggestions">
                {#each editBookTopicSuggestions as s}
                  <li><button type="button" onclick={() => appendTagId('topics', s.tag_id)}>{s.name || s.id}</button></li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>

        <div class="form-group">
          <label>{t('books.prereqsLabel')}</label>
          <div class="tag-chip-row">
            {#each editBookPrereqs as tag}
              <span class="tag-chip prereq">{localTag(tag)} <button type="button" onclick={() => removeBookPrereq(tag)}>×</button></span>
            {/each}
          </div>
          <div class="tag-input-wrap">
            <input class="tag-input" bind:value={editBookPrereqInput} placeholder={t('books.prereqsPlaceholder')} onkeydown={(e) => { if (e.key === 'Enter' && editBookPrereqInput.trim()) { e.preventDefault(); appendTagId('prereqs', editBookPrereqInput); } }} />
            {#if editBookPrereqSuggestions.length > 0}
              <ul class="tag-suggestions">
                {#each editBookPrereqSuggestions as s}
                  <li><button type="button" onclick={() => appendTagId('prereqs', s.tag_id)}>{s.name || s.id}</button></li>
                {/each}
              </ul>
            {/if}
          </div>
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

  <!-- Edition edit modal -->
  {#if showEditionEdit}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={() => showEditionEdit = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('books.editEdition') || 'Edit Edition'}</h3>
        {#if editionEditError}<p class="error-msg">{editionEditError}</p>{/if}
        <div class="form-group">
          <label>{t('books.editionName') || 'Edition Name'}</label>
          <input bind:value={editionEditName} placeholder="e.g. 3rd Edition, 中文版" />
        </div>
        <div class="form-group">
          <label>{t('books.editionTitle') || 'Title (optional)'}</label>
          <input bind:value={editionEditTitle} placeholder="e.g. 计算理论导引" />
        </div>
        <div class="form-group">
          <label>{t('books.editionSubtitle') || 'Subtitle (optional)'}</label>
          <input bind:value={editionEditSubtitle} placeholder="e.g. A Compact History of Infinity" />
        </div>
        <div class="form-group">
          <label>Language</label>
          <input bind:value={editionEditLang} placeholder="en" />
        </div>
        <div class="form-group">
          <label>ISBN</label>
          <input bind:value={editionEditIsbn} placeholder="978-..." />
        </div>
        <div class="form-group">
          <label>Publisher</label>
          <input bind:value={editionEditPublisher} />
        </div>
        <div class="form-group">
          <label>Year</label>
          <input bind:value={editionEditYear} placeholder="2024" />
        </div>
        <div class="form-group">
          <label>Translators (comma-separated)</label>
          <input bind:value={editionEditTranslators} placeholder="Name 1, Name 2" />
        </div>
        <div class="form-group">
          <label>Purchase links (one per line, label:url)</label>
          <textarea bind:value={editionEditLinks} rows="3" placeholder="Amazon:https://..."></textarea>
        </div>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => showEditionEdit = false}>Cancel</button>
          <button class="btn btn-primary" onclick={saveEditionEdit} disabled={editionEditSaving}>
            {editionEditSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Edit history diff modal -->
  {#if selectedLog}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={() => selectedLog = null}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('books.editHistory')}</h3>
        <div class="diff-meta">
          <span class="edit-log-who">{selectedLog.editor_handle ? `@${selectedLog.editor_handle}` : selectedLog.editor_did.slice(0, 20)}</span>
          <span class="edit-log-time">{new Date(selectedLog.created_at).toLocaleDateString()}</span>
          {#if selectedLog.summary}<p class="edit-log-summary">{selectedLog.summary}</p>{/if}
        </div>
        <div class="edit-diff">
          {#each Object.keys({ ...selectedLog.old_data, ...selectedLog.new_data }) as key}
            {@const oldVal = typeof selectedLog.old_data[key] === 'object' ? JSON.stringify(selectedLog.old_data[key], null, 2) : String(selectedLog.old_data[key] ?? '')}
            {@const newVal = typeof selectedLog.new_data[key] === 'object' ? JSON.stringify(selectedLog.new_data[key], null, 2) : String(selectedLog.new_data[key] ?? '')}
            {#if oldVal !== newVal}
              <div class="diff-field">
                <span class="diff-key">{key}</span>
                {#if oldVal}<div class="diff-old">- {oldVal}</div>{/if}
                {#if newVal}<div class="diff-new">+ {newVal}</div>{/if}
              </div>
            {/if}
          {/each}
        </div>
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => selectedLog = null}>{t('books.cancel') || 'Close'}</button>
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
  .book-subtitle { font-size: 1.1rem; color: var(--text-secondary); margin: -4px 0 4px; font-style: italic; }
  .book-abbr {
    display: inline-block;
    margin-left: 8px;
    padding: 2px 8px;
    font-size: 0.7em;
    font-weight: 500;
    font-family: var(--font-mono, monospace);
    color: var(--text-secondary);
    background: var(--bg-subtle, rgba(0,0,0,0.04));
    border: 1px solid var(--border);
    border-radius: 3px;
    vertical-align: middle;
    letter-spacing: 0.03em;
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
  .clear-rating {
    background: none;
    border: none;
    color: var(--text-hint);
    cursor: pointer;
    font-size: 14px;
    padding: 0 4px;
    line-height: 1;
  }
  .clear-rating:hover {
    color: #c00;
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
  .progress-readout {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }
  .progress-pct {
    color: var(--accent);
    font-weight: 500;
  }
  .progress-bar {
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
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
  .edition-cover-error {
    padding: 8px 10px;
    margin-bottom: 8px;
    border: 1px dashed var(--danger, #c0392b);
    border-radius: 3px;
    font-size: 12px;
    color: var(--danger, #c0392b);
    word-break: break-all;
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
  .edition-draft-badge {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--warning-bg, #fff4d9);
    color: var(--warning-fg, #8a5a00);
    border: 1px solid var(--warning-border, #e6c77a);
    letter-spacing: 0.04em;
    text-transform: uppercase;
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
  .resources-section { margin-top: 16px; }
  .resources-section h3 { font-size: 14px; font-weight: 600; margin-bottom: 8px; }
  .resource-group { margin-bottom: 10px; }
  .resource-kind { font-size: 12px; color: var(--text-hint); text-transform: capitalize; margin: 0 0 4px; font-weight: 500; }
  .resource-row { display: flex; align-items: center; gap: 4px; }
  .resource-link { flex: 1; font-size: 13px; color: var(--accent); text-decoration: none; padding: 2px 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .resource-link:hover { text-decoration: underline; }
  .resource-del { background: none; border: none; color: var(--text-hint); cursor: pointer; font-size: 14px; padding: 0 2px; line-height: 1; }
  .resource-del:hover { color: #c62828; }
  .resource-form { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; }
  .res-input { font-size: 12px; padding: 4px 6px; border: 1px solid var(--border); border-radius: 3px; }
  .res-form-actions { display: flex; gap: 6px; }
  .btn-sm { font-size: 12px; padding: 3px 10px; }
  .add-resource-btn { font-size: 13px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 4px 0; }
  .add-resource-btn:hover { text-decoration: underline; }
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
    font-size: 12px; cursor: pointer; background: none; border-left: none; border-right: none; border-top: none;
    width: 100%; text-align: left;
  }
  .edit-log:hover { background: var(--bg-secondary); }
  .edit-log-who { color: var(--accent); font-weight: 500; }
  .edit-log-summary { color: var(--text-secondary); }
  .edit-log-time { color: var(--text-hint); font-size: 11px; }
  .edit-diff {
    padding: 6px 8px; margin-bottom: 6px; background: var(--bg-secondary);
    border-radius: 4px; font-size: 12px; font-family: monospace;
  }
  .diff-field { margin-bottom: 6px; }
  .diff-key { font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 2px; }
  .diff-old { color: #c62828; white-space: pre-wrap; word-break: break-all; }
  .diff-new { color: #2e7d32; white-space: pre-wrap; word-break: break-all; }
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
  .book-tag-row {
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
    margin: 4px 0;
  }
  .book-tag-row-label {
    font-size: 12px; color: var(--text-secondary);
    min-width: 3.5em; margin-right: 2px;
  }
  .tag-badge {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 8px; border-radius: 12px; font-size: 12px;
    text-decoration: none; transition: opacity 0.15s;
  }
  .tag-badge.teaches { background: var(--accent-light, #e8f4fd); color: var(--accent); }
  .tag-badge.prereq { background: #fdf2e8; color: #b06000; }
  .tag-badge.topic { background: var(--bg-hover, #f5f5f5); color: var(--text-secondary); border: 1px dashed var(--border-strong); }
  .tag-badge.series-badge { background: #f0edf9; color: #5b21b6; cursor: pointer; }
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

  /* Short reviews */
  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
  .section-header h2 { margin: 0; }
  .write-btn {
    padding: 5px 12px;
    background: var(--accent, #6366f1);
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.82rem;
  }
  .edit-btn {
    margin-top: 6px;
    padding: 4px 10px;
    background: none;
    color: var(--accent, #6366f1);
    border: 1px solid var(--accent, #6366f1);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .composer-wrap { margin-bottom: 1.2rem; }
  .my-review-section { margin-bottom: 1.2rem; }

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
  .set-cover-btn.disabled { opacity: 0.6; pointer-events: none; }

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

  .tag-chip-row {
    display: flex; flex-wrap: wrap; gap: 6px;
    margin-bottom: 6px;
  }
  .tag-chip {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 8px; border-radius: 12px;
    background: var(--bg-hover, #f5f5f5);
    border: 1px solid var(--border);
    font-size: 12px; color: var(--text-primary);
  }
  .tag-chip.prereq { border-color: var(--warn, #d97706); color: var(--warn, #d97706); }
  .tag-chip.topic  { border-color: var(--accent, #6b7280); color: var(--accent, #6b7280); }
  .form-hint { font-size: 12px; color: var(--text-muted, #888); margin-bottom: 6px; }
  .tag-chip button {
    background: none; border: none; cursor: pointer; color: inherit;
    font-size: 14px; padding: 0; line-height: 1;
  }
  .tag-suggest {
    display: inline-block;
    margin: 2px 4px 0 0;
    padding: 3px 8px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    cursor: pointer;
    color: var(--text-primary);
  }
  .tag-suggest:hover { background: var(--bg-hover); }
</style>
