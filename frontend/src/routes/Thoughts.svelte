<script lang="ts">
  import { listThoughts, createThought, castVote, getArticleContent } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { authorName, fmtRep } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Article, ArticleContent, ContentFormat } from '../lib/types';
  import CommentThread from '../lib/components/CommentThread.svelte';

  let thoughts = $state<Article[]>([]);
  let loading = $state(true);
  let isLoggedIn = $derived(!!getAuth());

  // Compose
  let composeOpen = $state(false);
  let composeContent = $state('');
  let composeFormat = $state<ContentFormat>('markdown');
  let composeTags = $state('');
  let submitting = $state(false);

  // Expanded content cache
  let contentCache = $state(new Map<string, ArticleContent>());
  let expandedComments = $state(new Set<string>());

  $effect(() => {
    document.title = `${t('thoughts.title')} — NightBoat`;
    load();
  });

  async function load() {
    loading = true;
    try { thoughts = await listThoughts(100, 0); } catch { /* */ }
    loading = false;
  }

  async function submit() {
    if (!composeContent.trim()) return;
    submitting = true;
    try {
      const tags = composeTags.split(',').map(s => s.trim()).filter(Boolean);
      await createThought({
        title: '',
        content: composeContent.trim(),
        content_format: composeFormat,
        tags,
        prereqs: [],
      } as any);
      composeContent = '';
      composeTags = '';
      composeOpen = false;
      await load();
    } catch { /* */ }
    submitting = false;
  }

  async function loadContent(uri: string) {
    if (contentCache.has(uri)) return;
    try {
      const c = await getArticleContent(uri);
      contentCache.set(uri, c);
      contentCache = new Map(contentCache);
    } catch { /* */ }
  }

  // Auto-load content for all thoughts
  $effect(() => {
    for (const th of thoughts) {
      loadContent(th.at_uri);
    }
  });

  function toggleComments(uri: string) {
    if (expandedComments.has(uri)) expandedComments.delete(uri);
    else expandedComments.add(uri);
    expandedComments = new Set(expandedComments);
  }
</script>

<div class="thoughts-page">
  <div class="page-header">
    <div>
      <h1>{t('thoughts.title')}</h1>
      <p class="subtitle">{t('thoughts.subtitle')}</p>
    </div>
  </div>

  <!-- Compose box -->
  {#if isLoggedIn}
    <div class="compose-box">
      {#if !composeOpen}
        <button class="compose-trigger" onclick={() => composeOpen = true}>
          {t('thoughts.placeholder')}
        </button>
      {:else}
        <textarea
          bind:value={composeContent}
          rows="4"
          placeholder={t('thoughts.placeholder')}
          class="compose-input"
        ></textarea>
        <div class="compose-bar">
          <select bind:value={composeFormat} class="format-select">
            <option value="markdown">Markdown</option>
            <option value="typst">Typst</option>
          </select>
          <input type="text" bind:value={composeTags} placeholder="Tags (comma separated)" class="tags-input" />
          <div class="compose-actions">
            <button class="btn-cancel" onclick={() => { composeOpen = false; composeContent = ''; }}>{t('common.cancel')}</button>
            <button class="btn-submit" onclick={submit} disabled={submitting || !composeContent.trim()}>
              {submitting ? t('common.loading') : t('thoughts.new')}
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Timeline -->
  {#if loading}
    <p class="meta">{t('common.loading')}</p>
  {:else if thoughts.length === 0}
    <p class="empty">{t('thoughts.empty')}</p>
  {:else}
    <div class="timeline">
      {#each thoughts as th (th.at_uri)}
        <div class="thought-card">
          <div class="thought-header">
            <a href="/profile?did={encodeURIComponent(th.did)}" class="author">
              {authorName(th)}
            </a>
            {#if th.author_reputation > 0}
              <span class="rep">{fmtRep(th.author_reputation)}</span>
            {/if}
            <span class="dot">&middot;</span>
            <span class="date">{new Date(th.created_at).toLocaleDateString()}</span>
            {#if th.title}
              <span class="dot">&middot;</span>
              <span class="thought-title">{th.title}</span>
            {/if}
          </div>

          <div class="thought-body">
            {#if contentCache.has(th.at_uri)}
              {@html contentCache.get(th.at_uri)!.html}
            {:else}
              <p class="raw-desc">{th.summary || '...'}</p>
            {/if}
          </div>

          <div class="thought-footer">
            <button class="action-btn" onclick={() => castVote(th.at_uri, 1)}>
              &#9650; {th.vote_score}
            </button>
            <button class="action-btn" onclick={() => toggleComments(th.at_uri)}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
              {t('qa.showComments')}
            </button>
            <a href="/article?uri={encodeURIComponent(th.at_uri)}" class="action-btn permalink" title={t('qa.permalink')} aria-label={t('qa.permalink')}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
            </a>
          </div>

          {#if expandedComments.has(th.at_uri)}
            <div class="thought-comments">
              <CommentThread contentUri={th.at_uri} />
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .thoughts-page { max-width: 640px; margin: 0 auto; }
  .page-header { margin-bottom: 1rem; }
  .page-header h1 { font-family: var(--font-serif); font-weight: 400; margin: 0; }
  .subtitle { color: var(--text-secondary); font-size: 14px; margin: 4px 0 0; }

  /* Compose */
  .compose-box { margin-bottom: 1.5rem; }
  .compose-trigger {
    width: 100%; padding: 12px 14px; text-align: left;
    border: 1px solid var(--border); border-radius: 8px; background: var(--bg-white);
    color: var(--text-hint); font-size: 14px; cursor: pointer; transition: border-color 0.15s;
  }
  .compose-trigger:hover { border-color: var(--accent); }
  .compose-input {
    width: 100%; padding: 12px 14px; border: 1px solid var(--accent); border-radius: 8px 8px 0 0;
    font-size: 14px; font-family: var(--font-sans); resize: vertical; min-height: 80px;
    background: var(--bg-white); border-bottom: none;
  }
  .compose-input:focus { outline: none; }
  .compose-bar {
    display: flex; flex-wrap: wrap; gap: 8px; align-items: center;
    padding: 8px 12px; border: 1px solid var(--accent); border-top: 1px solid var(--border);
    border-radius: 0 0 8px 8px; background: var(--bg-page);
  }
  .format-select { padding: 4px 8px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white); }
  .tags-input { flex: 1; min-width: 120px; padding: 4px 8px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; }
  .compose-actions { display: flex; gap: 6px; margin-left: auto; }
  .btn-cancel { padding: 4px 12px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .btn-submit { padding: 4px 14px; font-size: 12px; border: none; border-radius: 3px; background: var(--accent); color: white; cursor: pointer; }
  .btn-submit:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Timeline */
  .timeline { display: flex; flex-direction: column; gap: 0; }
  .thought-card {
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
  }
  .thought-card:first-child { padding-top: 0; }

  .thought-header {
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
    font-size: 13px; color: var(--text-secondary); margin-bottom: 8px;
  }
  .author { color: var(--text-primary); font-weight: 500; text-decoration: none; }
  .author:hover { color: var(--accent); }
  .rep { font-size: 11px; font-weight: 600; color: var(--text-secondary); background: var(--bg-page); border: 1px solid var(--border); border-radius: 3px; padding: 0 3px; }
  .dot { color: var(--text-hint); }
  .date { color: var(--text-hint); }
  .thought-title { font-weight: 500; color: var(--text-primary); }

  .thought-body {
    font-size: 15px; line-height: 1.6; color: var(--text-primary);
    overflow: hidden;
  }
  .thought-body :global(p) { margin: 0 0 0.5em; }
  .thought-body :global(p:last-child) { margin-bottom: 0; }
  .thought-body :global(pre) { font-size: 13px; padding: 8px; background: var(--bg-page); border-radius: 4px; overflow-x: auto; }
  .thought-body :global(img) { max-width: 100%; border-radius: 6px; margin: 8px 0; }
  .raw-desc { margin: 0; color: var(--text-secondary); }

  .thought-footer {
    display: flex; align-items: center; gap: 12px; margin-top: 10px;
  }
  .action-btn {
    display: flex; align-items: center; gap: 4px;
    padding: 4px 8px; font-size: 12px;
    border: none; background: none; color: var(--text-hint); cursor: pointer;
    border-radius: 3px; transition: all 0.1s;
  }
  .action-btn:hover { color: var(--accent); background: rgba(95,155,101,0.06); }
  .permalink { text-decoration: none; margin-left: auto; }

  .thought-comments { margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border); }

  .empty { color: var(--text-hint); font-size: 14px; text-align: center; padding: 2rem; }
</style>
