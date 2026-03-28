<script lang="ts">
  import { listComments, createComment, updateComment, deleteComment, voteComment, getMyCommentVotes } from '../api';
  import { getAuth } from '../auth';
  import { t } from '../i18n';
  import { isBlocked } from '../blocklist';
  import type { Comment } from '../types';

  let {
    contentUri,
    contentEl = undefined,
  }: {
    contentUri: string;
    contentEl?: HTMLDivElement;
  } = $props();

  let comments = $state<Comment[]>([]);
  let commentBody = $state('');
  let submittingComment = $state(false);
  let editingCommentId = $state<string | null>(null);
  let editingCommentBody = $state('');
  let replyingToId = $state<string | null>(null);
  let replyBody = $state('');
  let quoteText = $state<string | null>(null);
  let commentError = $state('');
  let myCommentVotes = $state<Record<string, number>>({});

  let isLoggedIn = $derived(!!getAuth());
  let visibleComments = $derived(comments.filter(c => !isBlocked(c.did)));
  let rootComments = $derived(visibleComments.filter(c => !c.parent_id));
  function getReplies(parentId: string) { return visibleComments.filter(c => c.parent_id === parentId); }

  // Exposed for parent to set quote text from text selection
  export function setQuoteText(text: string) {
    quoteText = text;
  }

  export async function loadComments() {
    try {
      comments = await listComments(contentUri);
      if (getAuth()) {
        getMyCommentVotes(contentUri).then(votes => {
          const map: Record<string, number> = {};
          for (const v of votes) map[v.comment_id] = v.value;
          myCommentVotes = map;
        }).catch(() => {});
      }
    } catch { /* ignore */ }
  }

  export function getCommentCount() {
    return comments.length;
  }

  async function submitComment() {
    if (!commentBody.trim() || submittingComment) return;
    submittingComment = true;
    commentError = '';
    try {
      const c = await createComment(contentUri, commentBody.trim(), undefined, quoteText ?? undefined);
      comments = [...comments, c];
      commentBody = '';
      quoteText = null;
    } catch (e: any) {
      const msg = e.message || '';
      if (msg.includes('401') || msg.includes('Unauthorized')) {
        commentError = t('article.sessionExpired');
      } else {
        commentError = msg || t('article.postFailed');
      }
    } finally {
      submittingComment = false;
    }
  }

  async function doUpdateComment(id: string) {
    if (!editingCommentBody.trim()) return;
    const updated = await updateComment(id, editingCommentBody.trim());
    comments = comments.map(c => c.id === id ? updated : c);
    editingCommentId = null;
    editingCommentBody = '';
  }

  async function doDeleteComment(id: string) {
    if (!confirm(t('comments.deleteConfirm'))) return;
    try {
      await deleteComment(id);
      comments = comments.filter(c => c.id !== id && c.parent_id !== id);
    } catch (e: any) {
      commentError = e.message || t('comments.deleteFailed');
    }
  }

  async function submitReply(parentId: string) {
    if (!replyBody.trim()) return;
    commentError = '';
    try {
      const c = await createComment(contentUri, replyBody.trim(), parentId);
      comments = [...comments, c];
      replyBody = '';
      replyingToId = null;
    } catch (e: any) {
      commentError = e.message;
    }
  }

  async function doVoteComment(commentId: string, value: number) {
    const current = myCommentVotes[commentId] || 0;
    const newValue = current === value ? 0 : value;
    try {
      const result = await voteComment(commentId, newValue);
      myCommentVotes = { ...myCommentVotes, [commentId]: result.my_vote };
      comments = comments.map(c =>
        c.id === commentId ? { ...c, vote_score: result.score } : c
      );
    } catch { /* ignore */ }
  }

  function scrollToQuote(text: string) {
    if (!contentEl) return;
    const walker = document.createTreeWalker(contentEl, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode as Text;
      const idx = node.textContent?.indexOf(text) ?? -1;
      if (idx >= 0) {
        const range = document.createRange();
        range.setStart(node, idx);
        range.setEnd(node, idx + text.length);
        const sel = window.getSelection();
        sel?.removeAllRanges();
        sel?.addRange(range);
        const rect = range.getBoundingClientRect();
        window.scrollTo({ top: window.scrollY + rect.top - 120, behavior: 'smooth' });
        const mark = document.createElement('mark');
        mark.className = 'quote-highlight';
        range.surroundContents(mark);
        setTimeout(() => {
          const parent = mark.parentNode;
          if (parent) {
            parent.replaceChild(document.createTextNode(mark.textContent || ''), mark);
            parent.normalize();
          }
        }, 3000);
        return;
      }
    }
  }
</script>

<div class="comments-section">
  <h3 class="comments-title">{t('article.comments')} ({comments.length})</h3>

  {#if isLoggedIn}
    <div class="comment-form">
      {#if quoteText}
        <div class="quote-preview">
          <blockquote>{quoteText}</blockquote>
          <button class="quote-remove" onclick={() => { quoteText = null; }} title={t('common.remove')}>×</button>
        </div>
      {/if}
      <textarea
        bind:value={commentBody}
        placeholder={t('article.writeComment')}
        rows="3"
      ></textarea>
      <button class="comment-submit" onclick={submitComment} disabled={submittingComment || !commentBody.trim()}>
        {t('article.submit')}
      </button>
      {#if commentError}
        <p class="error-msg" style="margin-top:6px;font-size:13px">{commentError}</p>
      {/if}
    </div>
  {:else}
    <p class="meta">{t('article.loginToComment')}</p>
  {/if}

  {#snippet commentNode(c: Comment, depth: number)}
    <div class="comment-item" style:margin-left="{depth * 24}px">
      <div class="comment-header">
        <a href="#/profile?did={encodeURIComponent(c.did)}" class="comment-author">
          {c.author_handle ? `@${c.author_handle}` : c.did.slice(0, 20) + '…'}
        </a>
        <span class="comment-date">{c.created_at.split('T')[0]}</span>
        {#if getAuth()?.did === c.did}
          <button class="comment-action" title={t('comments.edit')} onclick={() => { editingCommentId = c.id; editingCommentBody = c.body; }}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button class="comment-action danger" title={t('comments.delete')} onclick={() => doDeleteComment(c.id)}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        {/if}
      </div>
      {#if editingCommentId === c.id}
        <div class="comment-edit">
          <textarea bind:value={editingCommentBody} rows="3"></textarea>
          <div class="comment-edit-actions">
            <button class="comment-submit" onclick={() => doUpdateComment(c.id)}>{t('comments.save')}</button>
            <button class="comment-cancel" onclick={() => { editingCommentId = null; }}>{t('comments.cancel')}</button>
          </div>
        </div>
      {:else}
        {#if c.quote_text}
          <blockquote class="comment-quote" role="button" tabindex="0" onclick={() => scrollToQuote(c.quote_text!)} onkeydown={(e) => { if (e.key === 'Enter') scrollToQuote(c.quote_text!); }}>
            {c.quote_text}
          </blockquote>
        {/if}
        <div class="comment-body">{c.body}</div>
      {/if}
      <div class="comment-footer">
        <div class="comment-vote-btns">
          <button class="vote-btn" class:active={myCommentVotes[c.id] === 1} onclick={() => doVoteComment(c.id, 1)} title={t('common.upvote')}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill={myCommentVotes[c.id] === 1 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14z"/><path d="M7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3"/></svg>
          </button>
          <span class="vote-count" class:positive={c.vote_score > 0} class:negative={c.vote_score < 0}>{c.vote_score}</span>
          <button class="vote-btn" class:active={myCommentVotes[c.id] === -1} onclick={() => doVoteComment(c.id, -1)} title={t('common.downvote')}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill={myCommentVotes[c.id] === -1 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M10 15v4a3 3 0 003 3l4-9V2H5.72a2 2 0 00-2 1.7l-1.38 9a2 2 0 002 2.3H10z"/><path d="M17 2h3a2 2 0 012 2v7a2 2 0 01-2 2h-3"/></svg>
          </button>
        </div>
        {#if isLoggedIn && depth < 3}
          <button class="reply-btn" onclick={() => { replyingToId = replyingToId === c.id ? null : c.id; replyBody = ''; }}>
            {t('common.reply')}
          </button>
        {/if}
      </div>
      {#if replyingToId === c.id}
        <div class="reply-form">
          <textarea bind:value={replyBody} rows="2" placeholder={t('article.writeReply')}></textarea>
          <div class="reply-actions">
            <button class="comment-submit" onclick={() => submitReply(c.id)} disabled={!replyBody.trim()}>
              {t('common.send')}
            </button>
            <button class="comment-cancel" onclick={() => { replyingToId = null; }}>
              {t('common.cancel')}
            </button>
          </div>
        </div>
      {/if}
      {#each getReplies(c.id) as reply}
        {@render commentNode(reply, depth + 1)}
      {/each}
    </div>
  {/snippet}

  {#if comments.length === 0}
    <p class="meta comment-empty">{t('article.noComments')}</p>
  {:else}
    <div class="comment-list">
      {#each rootComments as c}
        {@render commentNode(c, 0)}
      {/each}
    </div>
  {/if}
</div>

<style>
  .comments-section {
    margin-top: 2rem;
  }
  .comments-title {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.2rem;
    margin-bottom: 1rem;
  }
  .comment-form {
    margin-bottom: 1.5rem;
  }
  .comment-form textarea {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
    font-size: 14px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .comment-form textarea:focus {
    outline: none;
    border-color: var(--accent);
  }
  .comment-submit {
    margin-top: 6px;
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .comment-submit:hover:not(:disabled) { opacity: 0.85; }
  .comment-submit:disabled { opacity: 0.5; cursor: not-allowed; }
  .comment-cancel {
    margin-top: 6px;
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .comment-empty {
    padding: 1rem 0;
  }
  .comment-list {
    display: flex;
    flex-direction: column;
  }
  .comment-item {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .comment-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .comment-author {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    text-decoration: none;
  }
  .comment-author:hover { color: var(--accent); }
  .comment-date {
    font-size: 12px;
    color: var(--text-hint);
  }
  .comment-action {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px;
    color: var(--text-hint);
    display: flex;
    transition: color 0.15s;
    margin-left: auto;
  }
  .comment-action + .comment-action { margin-left: 0; }
  .comment-action:hover { color: var(--accent); }
  .comment-action.danger:hover { color: #c44; }
  .comment-body {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-primary);
    white-space: pre-wrap;
  }
  .comment-edit textarea {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
    font-size: 14px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .comment-edit-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
  .comment-footer {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }
  .comment-vote-btns {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .vote-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    color: var(--text-hint);
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }
  .vote-btn:hover { color: var(--accent); }
  .vote-btn.active { color: var(--accent); }
  .vote-count {
    font-size: 12px;
    color: var(--text-hint);
    min-width: 16px;
    text-align: center;
  }
  .vote-count.positive { color: var(--accent); }
  .vote-count.negative { color: #c44; }
  .reply-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-hint);
    padding: 0;
    transition: color 0.15s;
  }
  .reply-btn:hover { color: var(--accent); }
  .reply-form {
    margin-top: 8px;
    padding-left: 0;
  }
  .reply-form textarea {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 13px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .reply-form textarea:focus { outline: none; border-color: var(--accent); }
  .reply-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  /* Quote preview in comment form */
  .quote-preview {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 8px;
  }
  .quote-preview blockquote {
    flex: 1;
    margin: 0;
    padding: 8px 12px;
    border-left: 3px solid var(--accent);
    background: rgba(95, 155, 101, 0.06);
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
    border-radius: 0 4px 4px 0;
  }
  .quote-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 18px;
    color: var(--text-hint);
    padding: 4px;
    line-height: 1;
  }
  .quote-remove:hover { color: var(--text-primary); }

  /* Quote in comment display */
  .comment-quote {
    margin: 4px 0;
    padding: 6px 10px;
    border-left: 3px solid var(--accent);
    background: rgba(95, 155, 101, 0.06);
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 0 4px 4px 0;
    transition: background 0.15s;
  }
  .comment-quote:hover {
    background: rgba(95, 155, 101, 0.12);
  }
</style>
