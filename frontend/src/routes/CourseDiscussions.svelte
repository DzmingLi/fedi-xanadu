<script lang="ts">
  import { listCourseDiscussions, getCourseDetail, listComments, createComment } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { Comment, CourseSession } from '../lib/types';

  let { id } = $props<{ id: string }>();
  let items = $state<Comment[]>([]);
  let total = $state(0);
  let sessions = $state<CourseSession[]>([]);
  let courseTitle = $state('');
  let page = $state(0);
  const pageSize = 20;
  let loading = $state(true);
  let error = $state('');

  // Expanded replies per-thread
  let expandedId = $state<string | null>(null);
  let replies = $state<Record<string, Comment[]>>({});

  // New discussion form state
  let newTitle = $state('');
  let newBody = $state('');
  let newSection = $state<string | null>(null);
  let newReplyText = $state('');
  let replyingTo = $state<string | null>(null);
  let posting = $state(false);

  let totalPages = $derived(Math.max(1, Math.ceil(total / pageSize)));

  async function load() {
    loading = true;
    error = '';
    try {
      const [resp, detail] = await Promise.all([
        listCourseDiscussions(id, pageSize, page * pageSize),
        sessions.length === 0 ? getCourseDetail(id) : Promise.resolve(null),
      ]);
      items = resp.items;
      total = resp.total;
      if (detail) {
        sessions = detail.sessions;
        courseTitle = detail.course.title;
        document.title = `${t('course.qa')} — ${courseTitle}`;
      }
    } catch (e: any) {
      error = e.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { if (id) load(); });

  function gotoPage(p: number) {
    page = Math.max(0, Math.min(p, totalPages - 1));
    load();
    window.scrollTo({ top: 0 });
  }

  async function toggleThread(threadId: string) {
    if (expandedId === threadId) { expandedId = null; return; }
    expandedId = threadId;
    if (!(threadId in replies)) {
      const all = await listComments(`course:${id}`).catch(() => []);
      replies[threadId] = all.filter(c => c.parent_id === threadId);
    }
  }

  async function postNew() {
    const body = newBody.trim();
    const title = newTitle.trim();
    if (!body) return;
    posting = true;
    try {
      await createComment(`course:${id}`, body, undefined, undefined, newSection ?? undefined, title || undefined);
      newTitle = '';
      newBody = '';
      newSection = null;
      page = 0;
      await load();
    } catch (e: any) {
      error = e.message ?? String(e);
    } finally {
      posting = false;
    }
  }

  async function postReply(parentId: string) {
    const body = newReplyText.trim();
    if (!body) return;
    try {
      const c = await createComment(`course:${id}`, body, parentId);
      replies[parentId] = [...(replies[parentId] ?? []), c];
      newReplyText = '';
      replyingTo = null;
    } catch (e: any) {
      error = e.message ?? String(e);
    }
  }
</script>

<div class="page">
  <a class="back" href="/course?id={encodeURIComponent(id)}">← {courseTitle}</a>
  <header>
    <h1>{t('course.qa')} <span class="count">({total})</span></h1>
  </header>

  {#if error}<p class="err">{error}</p>{/if}

  {#if getAuth()}
    <section class="new-form" id="new">
      <h2>{t('course.askQuestion')}</h2>
      <input class="title-input" bind:value={newTitle} placeholder={t('course.titlePlaceholder')} maxlength="200" />
      <textarea bind:value={newBody} placeholder={t('course.bodyPlaceholder')} rows="4"></textarea>
      <div class="form-actions">
        <select bind:value={newSection}>
          <option value={null}>{t('course.allSessions')}</option>
          {#each sessions as s}
            <option value={s.id}>{s.sort_order}. {s.topic || ''}</option>
          {/each}
        </select>
        <button class="btn-primary" onclick={postNew} disabled={posting || !newBody.trim()}>
          {posting ? t('course.post') + '…' : t('course.post')}
        </button>
      </div>
    </section>
  {/if}

  {#if loading && items.length === 0}
    <p class="meta">{t('common.loading')}</p>
  {:else if items.length === 0}
    <p class="meta">{t('course.noDiscussions')}</p>
  {:else}
    {#each items as c}
      <article class="thread" id="c-{c.id}">
        <header class="t-hdr">
          <a href="/profile?did={encodeURIComponent(c.did)}" class="author">{c.author_handle || c.did.slice(0, 16)}</a>
          <span class="date">{new Date(c.created_at).toLocaleDateString()}</span>
          {#if c.section_ref}
            {@const lec = sessions.find(s => s.id === c.section_ref)}
            {#if lec}<span class="session">{t('course.onLecture')} {lec.sort_order}: {lec.topic}</span>{/if}
          {/if}
        </header>
        {#if c.title}<h3 class="t-title">{c.title}</h3>{/if}
        <p class="body">{c.body}</p>
        <div class="t-actions">
          <span class="score">▲ {c.vote_score}</span>
          <button class="ghost" onclick={() => toggleThread(c.id)}>{expandedId === c.id ? t('course.hideReplies') : t('course.showReplies')}</button>
          {#if getAuth()}
            <button class="ghost" onclick={() => replyingTo = replyingTo === c.id ? null : c.id}>{t('course.reply')}</button>
          {/if}
        </div>

        {#if replyingTo === c.id}
          <div class="reply-form">
            <textarea bind:value={newReplyText} placeholder={t('course.reply')} rows="2"></textarea>
            <button class="btn-primary" onclick={() => postReply(c.id)} disabled={!newReplyText.trim()}>{t('course.post')}</button>
          </div>
        {/if}

        {#if expandedId === c.id && (replies[c.id] ?? []).length > 0}
          <div class="replies">
            {#each replies[c.id] as r}
              <div class="reply">
                <header class="t-hdr">
                  <a href="/profile?did={encodeURIComponent(r.did)}" class="author">{r.author_handle || r.did.slice(0, 16)}</a>
                  <span class="date">{new Date(r.created_at).toLocaleDateString()}</span>
                </header>
                <p class="body">{r.body}</p>
              </div>
            {/each}
          </div>
        {/if}
      </article>
    {/each}

    {#if totalPages > 1}
      <div class="pager">
        <button onclick={() => gotoPage(page - 1)} disabled={page === 0}>← {t('common.prev')}</button>
        <span>{page + 1} / {totalPages}</span>
        <button onclick={() => gotoPage(page + 1)} disabled={page >= totalPages - 1}>{t('common.next')} →</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page { max-width: 840px; margin: 0 auto; padding: 24px 16px; }
  .back { display: inline-block; margin-bottom: 12px; font-size: 13px; color: var(--text-secondary); text-decoration: none; }
  .back:hover { color: var(--accent); }
  header { margin-bottom: 20px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.6rem; margin: 0; }
  .count { color: var(--text-hint); font-size: 0.85em; }
  .meta { color: var(--text-hint); }
  .err { background: #fee; color: #c00; padding: 8px 12px; border-radius: 4px; font-size: 13px; }

  .new-form { padding: 16px; background: var(--bg-white); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 20px; }
  .new-form h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.1rem; margin: 0 0 10px; }
  .title-input { width: 100%; padding: 8px 10px; border: 1px solid var(--border); border-radius: 4px; font-size: 14px; background: var(--bg-page); color: var(--text-primary); box-sizing: border-box; margin-bottom: 8px; }
  .new-form textarea { width: 100%; padding: 10px; border: 1px solid var(--border); border-radius: 4px; font-size: 14px; font-family: inherit; resize: vertical; background: var(--bg-page); color: var(--text-primary); box-sizing: border-box; }
  .form-actions { display: flex; gap: 8px; align-items: center; margin-top: 8px; }
  .form-actions select { padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-white); font-size: 13px; flex: 1; }
  .btn-primary { padding: 6px 16px; background: var(--accent); color: white; border: none; border-radius: 4px; font-size: 13px; cursor: pointer; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .thread { padding: 14px 16px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 12px; background: var(--bg-white); }
  .t-hdr { display: flex; gap: 10px; align-items: center; font-size: 12px; color: var(--text-hint); border: none; padding: 0; margin: 0 0 4px; }
  .author { color: var(--text-primary); font-weight: 500; text-decoration: none; }
  .author:hover { color: var(--accent); }
  .session { padding: 1px 6px; background: var(--bg-hover, #f5f5f5); border-radius: 3px; font-size: 11px; }
  .t-title { font-family: var(--font-serif); font-size: 17px; margin: 4px 0; color: var(--text-primary); }
  .body { font-size: 14px; color: var(--text-primary); line-height: 1.6; margin: 4px 0; white-space: pre-wrap; }
  .t-actions { display: flex; gap: 14px; align-items: center; font-size: 12px; margin-top: 8px; }
  .score { color: var(--text-hint); }
  .ghost { background: none; border: none; color: var(--accent); cursor: pointer; padding: 0; font-size: 12px; }
  .ghost:hover { text-decoration: underline; }
  .reply-form { margin-top: 10px; }
  .reply-form textarea { width: 100%; padding: 8px; border: 1px solid var(--border); border-radius: 4px; font-size: 13px; font-family: inherit; resize: vertical; background: var(--bg-page); color: var(--text-primary); box-sizing: border-box; }
  .reply-form .btn-primary { margin-top: 6px; }
  .replies { margin-top: 10px; padding-left: 14px; border-left: 2px solid var(--border); }
  .reply { padding: 8px 0; }

  .pager { display: flex; justify-content: center; align-items: center; gap: 12px; margin-top: 24px; font-size: 13px; }
  .pager button { padding: 4px 12px; border: 1px solid var(--border); background: var(--bg-white); border-radius: 4px; cursor: pointer; }
  .pager button:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .pager button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
