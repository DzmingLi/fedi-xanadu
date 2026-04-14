<script lang="ts">
  import { getQuestionDetail, postAnswer, castVote, getMyVote, getArticleContent, addBookmark, removeBookmark, getRelatedQuestions } from '../lib/api';
  import { authorName } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import type { ArticleContent, QuestionDetail, ContentFormat } from '../lib/types';
  import { toast } from '../lib/components/Toast.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';

  let expandedComments = $state(new Set<string>());
  let answerVotes = $state(new Map<string, number>());
  let answerBookmarks = $state(new Set<string>());

  let { uri } = $props<{ uri: string }>();

  let locale = $derived(getLocale());

  let detail = $state<QuestionDetail | null>(null);
  let questionContent = $state<ArticleContent | null>(null);
  let answerContents = $state(new Map<string, ArticleContent>());
  let loading = $state(true);
  let error = $state('');
  let relatedQuestions = $state<import('../lib/types').Article[]>([]);

  // Answer form
  let showAnswerForm = $state(false);
  let answerContent = $state('');
  let answerFormat = $state<ContentFormat>('markdown');
  let answerSubmitting = $state(false);

  // Votes
  let myVote = $state(0);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      const d = await getQuestionDetail(uri);
      detail = d;
      document.title = `${d.question.title} — NightBoat`;

      // Load related questions (non-blocking)
      getRelatedQuestions(uri).then(rq => { relatedQuestions = rq; }).catch(() => {});

      // Load question content and all answer contents in parallel
      const contentPromises: Promise<void>[] = [];
      contentPromises.push(
        getArticleContent(uri).then(c => { questionContent = c; })
      );
      for (const a of d.answers) {
        contentPromises.push(
          getArticleContent(a.at_uri).then(c => {
            answerContents.set(a.at_uri, c);
            answerContents = new Map(answerContents);
          })
        );
      }

      // Load votes for question and answers
      if (getAuth()) {
        contentPromises.push(
          getMyVote(uri).then(v => { myVote = v.value; }).catch(() => {})
        );
        for (const a of d.answers) {
          contentPromises.push(
            getMyVote(a.at_uri).then(v => {
              answerVotes.set(a.at_uri, v.value);
              answerVotes = new Map(answerVotes);
            }).catch(() => {})
          );
        }
      }

      await Promise.all(contentPromises);
    } catch (e: any) {
      error = e.message || 'Failed to load';
    }
    loading = false;
  }

  async function vote(value: number) {
    if (!getAuth()) return;
    const v = myVote === value ? 0 : value;
    try {
      const result = await castVote(uri, v);
      myVote = v;
      if (detail) {
        detail.question.vote_score = result.score;
        detail = { ...detail };
      }
    } catch { /* */ }
  }

  async function voteAnswer(answerUri: string, value: number) {
    if (!getAuth()) return;
    const current = answerVotes.get(answerUri) || 0;
    const v = current === value ? 0 : value;
    try {
      const result = await castVote(answerUri, v);
      answerVotes.set(answerUri, v);
      answerVotes = new Map(answerVotes);
      if (detail) {
        const a = detail.answers.find(a => a.at_uri === answerUri);
        if (a) a.vote_score = result.score;
        detail = { ...detail };
      }
    } catch { /* */ }
  }

  async function bookmarkAnswer(answerUri: string) {
    if (!getAuth()) return;
    try {
      if (answerBookmarks.has(answerUri)) {
        await removeBookmark(answerUri);
        answerBookmarks.delete(answerUri);
      } else {
        await addBookmark(answerUri);
        answerBookmarks.add(answerUri);
      }
      answerBookmarks = new Set(answerBookmarks);
    } catch { /* */ }
  }

  async function submitAnswer() {
    if (!answerContent.trim()) return;
    answerSubmitting = true;
    try {
      await postAnswer({
        title: `Re: ${detail?.question.title || ''}`,
        content: answerContent,
        content_format: answerFormat,
        translation_of: uri, // question_uri passed as translation_of
        tags: [],
        prereqs: [],
      });
      showAnswerForm = false;
      answerContent = '';
      // Reload to get new answer
      await load();
    } catch (e: any) {
      toast(e.message || 'Failed to post answer', 'error');
    }
    answerSubmitting = false;
  }
</script>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if error}
  <p class="error">{error}</p>
{:else if detail}
  {@const q = detail.question}
<div class="q-layout">
  <main class="q-main">

  <!-- Question -->
  <div class="question-section">
    <div class="q-header">
      <span class="q-badge">{t('qa.questionBadge')}</span>
      <h1 class="q-title">{q.title}</h1>
    </div>

    <div class="q-meta">
      <a href="/profile?did={encodeURIComponent(q.did)}" class="author">{authorName(q)}</a>
      <span>&middot;</span>
      <span>{q.created_at.split(' ')[0]}</span>
      <span>&middot;</span>
      <span>{t('qa.answerCount', q.answer_count)}</span>
    </div>

    {#if questionContent}
      <div class="content">
        {@html questionContent.html}
      </div>
    {/if}

    <div class="q-actions">
      <button class="vote-btn" class:active={myVote === 1} onclick={() => vote(1)}>&#9650; {t('article.upvote')}</button>
      <span class="vote-score">{q.vote_score}</span>
      <button class="vote-btn" class:active={myVote === -1} onclick={() => vote(-1)}>&#9660; {t('article.downvote')}</button>

      {#if getAuth() && q.did === getAuth()?.did}
        <a href="/new?edit={encodeURIComponent(q.at_uri)}" class="edit-link">{t('common.edit')}</a>
      {/if}
    </div>
  </div>

  <!-- Answers -->
  <div class="answers-section">
    <div class="answers-header">
      <h2>{t('qa.answerCount', detail.answers.length)}</h2>
      {#if getAuth()}
        <button class="btn-answer" onclick={() => { showAnswerForm = !showAnswerForm; }}>
          {t('qa.writeAnswer')}
        </button>
      {/if}
    </div>

    {#if showAnswerForm}
      <div class="answer-form">
        <div class="format-row">
          <select bind:value={answerFormat}>
            <option value="markdown">Markdown</option>
            <option value="typst">Typst</option>
            <option value="html">HTML</option>
          </select>
        </div>
        <textarea
          bind:value={answerContent}
          placeholder={t('qa.answerPlaceholder')}
          rows="8"
        ></textarea>
        <div class="form-actions">
          <button class="btn-cancel" onclick={() => { showAnswerForm = false; }}>{t('common.cancel')}</button>
          <button class="btn-submit" onclick={submitAnswer} disabled={answerSubmitting || !answerContent.trim()}>
            {answerSubmitting ? t('common.loading') : t('qa.postAnswer')}
          </button>
        </div>
      </div>
    {/if}

    {#if detail.answers.length === 0}
      <p class="empty">{t('qa.noAnswers')}</p>
    {:else}
      {#each detail.answers as answer}
        {@const content = answerContents.get(answer.at_uri)}
        <div class="answer-card">
          <div class="answer-meta">
            <a href="/profile?did={encodeURIComponent(answer.did)}" class="author">{authorName(answer)}</a>
            <span>&middot;</span>
            <span>{answer.created_at.split(' ')[0]}</span>
          </div>
          {#if content}
            <div class="content">
              {@html content.html}
            </div>
          {:else}
            <p class="meta">{t('common.loading')}</p>
          {/if}
          <div class="answer-actions">
            <button class="vote-btn" class:active={(answerVotes.get(answer.at_uri) || 0) > 0} onclick={() => voteAnswer(answer.at_uri, 1)}>&#9650;</button>
            <span class="vote-score">{answer.vote_score}</span>
            <button class="vote-btn" class:active={(answerVotes.get(answer.at_uri) || 0) < 0} onclick={() => voteAnswer(answer.at_uri, -1)}>&#9660;</button>
            <button class="bookmark-btn" class:active={answerBookmarks.has(answer.at_uri)} onclick={() => bookmarkAnswer(answer.at_uri)} title={t('article.bookmark')}>&#9733;</button>
            <button class="comment-toggle" onclick={() => {
              const s = new Set(expandedComments);
              if (s.has(answer.at_uri)) s.delete(answer.at_uri); else s.add(answer.at_uri);
              expandedComments = s;
            }}>
              {expandedComments.has(answer.at_uri) ? t('qa.hideComments') : t('qa.showComments')}
            </button>
          </div>
          {#if expandedComments.has(answer.at_uri)}
            <div class="answer-comments">
              <CommentThread contentUri={answer.at_uri} />
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  </main>

  {#if relatedQuestions.length > 0}
    <aside class="q-sidebar">
      <div class="sidebar-heading">{t('qa.relatedQuestions') || 'Related Questions'}</div>
      <div class="related-list">
        {#each relatedQuestions as rq}
          <a href="/question?uri={encodeURIComponent(rq.at_uri)}" class="related-card">
            <span class="related-title">{rq.title}</span>
            <span class="related-meta">
              {rq.answer_count} {t('qa.answers') || 'answers'}
              {#if rq.vote_score > 0}&middot; &#9650;{rq.vote_score}{/if}
            </span>
          </a>
        {/each}
      </div>
    </aside>
  {/if}
</div>

{/if}

<style>
  .q-layout {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
  }
  .q-main {
    flex: 1;
    min-width: 0;
  }
  .q-sidebar {
    width: 220px;
    flex-shrink: 0;
    position: sticky;
    top: 4rem;
    align-self: flex-start;
  }
  .sidebar-heading {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 8px;
  }
  .related-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .related-card {
    display: block;
    padding: 6px 8px;
    border-left: 2px solid var(--border);
    text-decoration: none;
    border-radius: 0 3px 3px 0;
    transition: all 0.1s;
  }
  .related-card:hover {
    border-left-color: var(--accent);
    background: var(--bg-hover);
    text-decoration: none;
  }
  .related-title {
    display: block;
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .related-card:hover .related-title {
    color: var(--accent);
  }
  .related-meta {
    display: block;
    font-size: 11px;
    color: var(--text-hint);
    margin-top: 2px;
  }

  @media (max-width: 800px) {
    .q-layout { flex-direction: column; }
    .q-sidebar { width: 100%; position: static; border-top: 1px solid var(--border); padding-top: 1rem; }
  }

  .question-section {
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border);
  }
  .q-header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 8px;
  }
  .q-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    padding: 3px 10px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
    margin-top: 6px;
  }
  .q-title {
    font-family: var(--font-serif);
    font-size: 1.6rem;
    font-weight: 400;
    margin: 0;
    line-height: 1.35;
  }
  .q-meta {
    display: flex;
    gap: 6px;
    font-size: 13px;
    color: var(--text-hint);
    margin-bottom: 16px;
  }
  .author {
    color: var(--text-secondary);
    text-decoration: none;
  }
  .author:hover {
    color: var(--accent);
    text-decoration: none;
  }

  .q-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
  }
  .vote-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 3px 10px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .vote-btn:hover { border-color: var(--accent); color: var(--accent); }
  .vote-btn.active { background: var(--accent); color: white; border-color: var(--accent); }
  .vote-score {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    min-width: 24px;
    text-align: center;
  }
  .edit-link {
    font-size: 13px;
    color: var(--text-hint);
    margin-left: auto;
  }

  /* Answers */
  .answers-section {
    margin-bottom: 32px;
  }
  .answers-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .answers-header h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.1rem;
    margin: 0;
    color: var(--text-secondary);
  }
  .btn-answer {
    font-size: 13px;
    padding: 5px 14px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--accent);
    background: none;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-answer:hover { background: var(--accent); color: white; }

  .answer-form {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px;
    margin-bottom: 16px;
  }
  .format-row {
    margin-bottom: 8px;
  }
  .format-row select {
    font-size: 13px;
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-sans);
  }
  .answer-form textarea {
    width: 100%;
    padding: 10px;
    font-size: 14px;
    font-family: var(--font-mono, monospace);
    border: 1px solid var(--border);
    border-radius: 4px;
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
  .btn-cancel {
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .btn-submit {
    padding: 5px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .btn-submit:disabled { opacity: 0.5; cursor: not-allowed; }

  .answer-card {
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    padding: 20px 0;
    margin-bottom: 0;
  }
  .answer-card:last-child {
    border-bottom: none;
  }
  .answer-meta {
    display: flex;
    gap: 6px;
    font-size: 13px;
    color: var(--text-hint);
    margin-bottom: 12px;
  }
  .answer-actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .bookmark-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 3px 8px;
    font-size: 13px;
    color: var(--text-hint);
    cursor: pointer;
    transition: all 0.15s;
    margin-left: 4px;
  }
  .bookmark-btn:hover { border-color: #d4a017; color: #d4a017; }
  .bookmark-btn.active { background: #d4a017; color: white; border-color: #d4a017; }
  .comment-toggle {
    background: none;
    border: none;
    font-size: 12px;
    color: var(--text-hint);
    cursor: pointer;
    margin-left: auto;
  }
  .comment-toggle:hover { color: var(--accent); }
  .answer-comments {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .empty { color: var(--text-hint); font-size: 14px; }
  .error { color: #dc2626; }
</style>
