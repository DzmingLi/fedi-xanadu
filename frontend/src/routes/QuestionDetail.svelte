<script lang="ts">
  import { getQuestionDetail, postAnswer, castVote, getMyVote, getArticleContent } from '../lib/api';
  import { authorName } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import type { ArticleContent, QuestionDetail, ContentFormat } from '../lib/types';
  import { toast } from '../lib/components/Toast.svelte';

  let { uri } = $props<{ uri: string }>();

  let locale = $derived(getLocale());

  let detail = $state<QuestionDetail | null>(null);
  let questionContent = $state<ArticleContent | null>(null);
  let answerContents = $state(new Map<string, ArticleContent>());
  let loading = $state(true);
  let error = $state('');

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

      // Load vote
      if (getAuth()) {
        contentPromises.push(
          getMyVote(uri).then(v => { myVote = v.value; }).catch(() => {})
        );
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

  <!-- Question -->
  <div class="question-section">
    <div class="q-header">
      <span class="q-badge">{t('qa.questionBadge')}</span>
      <h1 class="q-title">{q.title}</h1>
    </div>

    <div class="q-meta">
      <a href="#/profile?did={encodeURIComponent(q.did)}" class="author">{authorName(q)}</a>
      <span>&middot;</span>
      <span>{q.created_at.split(' ')[0]}</span>
      <span>&middot;</span>
      <span>{t('qa.answerCount', q.answer_count)}</span>
    </div>

    {#if questionContent}
      <div class="content-body rendered-html">
        {@html questionContent.html}
      </div>
    {/if}

    <div class="q-actions">
      <button class="vote-btn" class:active={myVote === 1} onclick={() => vote(1)}>&#9650; {t('article.upvote')}</button>
      <span class="vote-score">{q.vote_score}</span>
      <button class="vote-btn" class:active={myVote === -1} onclick={() => vote(-1)}>&#9660; {t('article.downvote')}</button>

      {#if getAuth() && q.did === getAuth()?.did}
        <a href="#/new?edit={encodeURIComponent(q.at_uri)}" class="edit-link">{t('common.edit')}</a>
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
            <a href="#/profile?did={encodeURIComponent(answer.did)}" class="author">{authorName(answer)}</a>
            <span>&middot;</span>
            <span>{answer.created_at.split(' ')[0]}</span>
          </div>
          {#if content}
            <div class="content-body rendered-html">
              {@html content.html}
            </div>
          {:else}
            <p class="meta">{t('common.loading')}</p>
          {/if}
          <div class="answer-actions">
            <span class="vote-score">&#9650; {answer.vote_score}</span>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Comments on the question -->
  <div class="comments-section">
    <CommentThread contentUri={uri} />
  </div>
{/if}

<style>
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

  .content-body {
    font-size: 15px;
    line-height: 1.7;
    color: var(--text-primary);
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
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px 20px;
    margin-bottom: 12px;
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

  .comments-section {
    margin-top: 24px;
  }

  .empty { color: var(--text-hint); font-size: 14px; }
  .error { color: #dc2626; }
</style>
