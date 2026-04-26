<script lang="ts">
  // Course-level Q&A. Comments are anchored to contentUri = "course:{id}"
  // which is shared across every iteration of the course — discussion
  // does not splinter by semester.
  //
  // We don't expose a session-picker here (those vary per iteration);
  // questions tied to a specific lecture should be filed on the term
  // page once the iteration-scoped UI exists. For now this is the
  // catch-all course discussion thread.
  import { listComments, createComment, getCourseDetail } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { Comment } from '../lib/types';

  let { id } = $props<{ id: string }>();
  let comments = $state<Comment[]>([]);
  let courseTitle = $state('');
  let loading = $state(true);
  let error = $state('');

  let newTitle = $state('');
  let newBody = $state('');
  let newReplyText = $state('');
  let replyingTo = $state<string | null>(null);
  let posting = $state(false);
  let expandedId = $state<string | null>(null);

  let rootComments = $derived(comments.filter(c => !c.parent_id));
  function getReplies(parentId: string) {
    return comments.filter(c => c.parent_id === parentId);
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [cs, detail] = await Promise.all([
        listComments(`course:${id}`),
        courseTitle ? Promise.resolve(null) : getCourseDetail(id),
      ]);
      comments = cs;
      if (detail) {
        courseTitle = detail.course.title;
        document.title = `${t('course.discussion')} — ${courseTitle}`;
      }
    } catch (e: any) {
      error = e.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { if (id) load(); });

  async function postNew() {
    const body = newBody.trim();
    const title = newTitle.trim();
    if (!body) return;
    posting = true;
    try {
      await createComment(`course:${id}`, body, undefined, undefined, undefined, title || undefined);
      newTitle = '';
      newBody = '';
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
      comments = [...comments, c];
      newReplyText = '';
      replyingTo = null;
    } catch (e: any) {
      error = e.message ?? String(e);
    }
  }

  function toggleThread(threadId: string) {
    expandedId = expandedId === threadId ? null : threadId;
  }
</script>

<div class="page">
  <a class="back" href="/course?id={encodeURIComponent(id)}">← {courseTitle}</a>
  <header>
    <h1>{t('course.discussion')} <span class="count">({comments.length})</span></h1>
  </header>

  {#if error}<p class="err">{error}</p>{/if}

  {#if getAuth()}
    <section class="new-form" id="new">
      <h2>{t('course.askQuestion')}</h2>
      <input class="title-input" bind:value={newTitle} placeholder={t('term.titlePlaceholder')} maxlength="200" />
      <textarea bind:value={newBody} placeholder={t('term.bodyPlaceholder')} rows="4"></textarea>
      <div class="form-actions">
        <button class="btn-primary" onclick={postNew} disabled={posting || !newBody.trim()}>
          {posting ? t('term.post') + '…' : t('term.post')}
        </button>
      </div>
    </section>
  {/if}

  {#if loading && comments.length === 0}
    <p class="meta">{t('common.loading')}</p>
  {:else if rootComments.length === 0}
    <p class="meta">{t('course.noDiscussions')}</p>
  {:else}
    {#each rootComments as c}
      <article class="thread" id="c-{c.id}">
        <header class="t-hdr">
          <a href="/profile?did={encodeURIComponent(c.did)}" class="author">{c.author_handle || c.did.slice(0, 16)}</a>
          <span class="date">{new Date(c.created_at).toLocaleDateString()}</span>
        </header>
        {#if c.title}<h3 class="t-title">{c.title}</h3>{/if}
        <p class="body">{c.body}</p>
        <div class="t-actions">
          <span class="score">▲ {c.vote_score}</span>
          <button class="ghost" onclick={() => toggleThread(c.id)}>
            {expandedId === c.id ? t('term.hideReplies') : t('term.showReplies')}
          </button>
          {#if getAuth()}
            <button class="ghost" onclick={() => replyingTo = replyingTo === c.id ? null : c.id}>{t('term.reply')}</button>
          {/if}
        </div>

        {#if replyingTo === c.id}
          <div class="reply-form">
            <textarea bind:value={newReplyText} placeholder={t('term.reply')} rows="2"></textarea>
            <button class="btn-primary" onclick={() => postReply(c.id)} disabled={!newReplyText.trim()}>{t('term.post')}</button>
          </div>
        {/if}

        {#if expandedId === c.id && getReplies(c.id).length > 0}
          <div class="replies">
            {#each getReplies(c.id) as r}
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
  .form-actions { display: flex; gap: 8px; align-items: center; margin-top: 8px; justify-content: flex-end; }
  .btn-primary { padding: 6px 16px; background: var(--accent); color: white; border: none; border-radius: 4px; font-size: 13px; cursor: pointer; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .thread { padding: 14px 16px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 12px; background: var(--bg-white); }
  .t-hdr { display: flex; gap: 10px; align-items: center; font-size: 12px; color: var(--text-hint); border: none; padding: 0; margin: 0 0 4px; }
  .author { color: var(--text-primary); font-weight: 500; text-decoration: none; }
  .author:hover { color: var(--accent); }
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
</style>
