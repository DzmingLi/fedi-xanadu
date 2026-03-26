<script lang="ts">
  import { getArticle, getArticleContent, getArticlePrereqs, getArticleForks, listBookmarks, addBookmark, removeBookmark, getArticleVotes, getMyVote, castVote, getSeriesContext, forkArticle, getTranslations, listComments, createComment, updateComment, deleteComment, updateArticle, deleteArticle, voteComment, getMyCommentVotes } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { tagName } from '../lib/display';
  import { t, LANG_NAMES } from '../lib/i18n';
  import type { Article, ArticleContent, ArticlePrereqRow, ForkWithTitle, BookmarkWithTitle, VoteSummary, SeriesContextItem, Comment } from '../lib/types';

  let { uri }: { uri: string } = $props();

  let article = $state<Article | null>(null);
  let content = $state<ArticleContent | null>(null);
  let prereqs = $state<ArticlePrereqRow[]>([]);
  let forks = $state<ForkWithTitle[]>([]);
  let translations = $state<Article[]>([]);
  let error = $state('');
  let bookmarks = $state<BookmarkWithTitle[]>([]);
  let isBookmarked = $derived(bookmarks.some(b => b.article_uri === uri));
  let votes = $state<VoteSummary | null>(null);
  let myVote = $state(0);
  let isLoggedIn = $derived(!!getAuth());
  let seriesContext = $state<SeriesContextItem[]>([]);
  let comments = $state<Comment[]>([]);
  let commentBody = $state('');
  let submittingComment = $state(false);
  let editingCommentId = $state<string | null>(null);
  let editingCommentBody = $state('');
  let replyingToId = $state<string | null>(null);
  let replyBody = $state('');
  let quoteText = $state<string | null>(null);
  let quotePopup = $state<{ x: number; y: number; text: string } | null>(null);
  let myCommentVotes = $state<Record<string, number>>({});
  let rootComments = $derived(comments.filter(c => !c.parent_id));
  function getReplies(parentId: string) { return comments.filter(c => c.parent_id === parentId); }
  let isOwner = $derived(!!getAuth() && article?.did === getAuth()?.did);

  interface TocItem { id: string; text: string; level: number; }
  let tocItems = $state<TocItem[]>([]);
  let activeId = $state('');

  let contentEl: HTMLDivElement | undefined = $state();
  let topForks = $derived(forks.slice(0, 3));

  $effect(() => {
    if (!uri) return;
    error = '';
    article = null;
    content = null;
    tocItems = [];
    seriesContext = [];
    translations = [];
    const ac = new AbortController();
    Promise.all([
      getArticle(uri),
      getArticleContent(uri),
      getArticlePrereqs(uri),
      getArticleForks(uri),
      listBookmarks(),
      getArticleVotes(uri),
      getMyVote(uri),
      getSeriesContext(uri),
      getTranslations(uri),
      listComments(uri),
    ]).then(([a, c, p, f, bk, v, mv, sc, tr, cm]) => {
      if (ac.signal.aborted) return;
      article = a;
      content = c;
      prereqs = p;
      forks = f;
      bookmarks = bk;
      votes = v;
      myVote = mv.value;
      seriesContext = sc;
      translations = tr;
      comments = cm;
      // Load my comment votes
      if (getAuth()) {
        getMyCommentVotes(uri).then(votes => {
          const map: Record<string, number> = {};
          for (const v of votes) map[v.comment_id] = v.value;
          myCommentVotes = map;
        }).catch(() => {});
      }
    }).catch(e => {
      if (ac.signal.aborted) return;
      error = e.message;
    });
    return () => ac.abort();
  });

  // After content renders, extract headings for TOC and convert footnotes to sidenotes
  $effect(() => {
    if (!contentEl || !content) return;

    // Generate IDs for headings and build TOC
    const headings = contentEl.querySelectorAll('h2, h3, h4');
    const items: TocItem[] = [];
    const usedIds = new Set<string>();
    headings.forEach(h => {
      let id = h.id || h.textContent!.trim()
        .toLowerCase()
        .replace(/[^\w\u4e00-\u9fff]+/g, '-')
        .replace(/^-|-$/g, '');
      // Deduplicate
      let finalId = id;
      let n = 1;
      while (usedIds.has(finalId)) { finalId = `${id}-${n++}`; }
      usedIds.add(finalId);
      h.id = finalId;
      items.push({
        id: finalId,
        text: h.textContent!.trim(),
        level: parseInt(h.tagName[1]),
      });
    });
    tocItems = items;

    // Convert footnotes to sidenotes
    convertFootnotesToSidenotes(contentEl);

    // Render KaTeX math for markdown articles
    if (article?.content_format === 'markdown') {
      renderKatex(contentEl);
    }

    // Intersection observer for active heading
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            activeId = entry.target.id;
          }
        }
      },
      { rootMargin: '-80px 0px -70% 0px' }
    );
    headings.forEach(h => observer.observe(h));

    return () => observer.disconnect();
  });

  async function doVote(value: number) {
    if (!isLoggedIn) return;
    const newValue = myVote === value ? 0 : value; // toggle off if same
    votes = await castVote(uri, newValue);
    myVote = newValue;
  }

  async function toggleBookmark() {
    if (isBookmarked) {
      await removeBookmark(uri);
    } else {
      await addBookmark(uri);
    }
    bookmarks = await listBookmarks();
  }

  function doFork() {
    if (!article) return;
    window.location.hash = `#/new?fork_of=${encodeURIComponent(article.at_uri)}`;
  }

  let commentError = $state('');

  async function submitComment() {
    if (!commentBody.trim() || submittingComment) return;
    submittingComment = true;
    commentError = '';
    try {
      const c = await createComment(uri, commentBody.trim(), undefined, quoteText ?? undefined);
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
    await deleteComment(id);
    comments = comments.filter(c => c.id !== id && c.parent_id !== id);
  }

  async function submitReply(parentId: string) {
    if (!replyBody.trim()) return;
    commentError = '';
    try {
      const c = await createComment(uri, replyBody.trim(), parentId);
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

  function doEdit() {
    if (!article) return;
    window.location.hash = `#/new?edit=${encodeURIComponent(article.at_uri)}`;
  }

  async function doDelete() {
    if (!article || !confirm(t('article.deleteConfirm'))) return;
    await deleteArticle(uri);
    window.location.hash = '#/';
  }

  function onContentMouseUp(e: MouseEvent) {
    const sel = window.getSelection();
    const text = sel?.toString().trim();
    if (!text || !isLoggedIn) { quotePopup = null; return; }
    const range = sel!.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    quotePopup = { x: rect.left + rect.width / 2, y: rect.top - 8, text };
  }

  function startQuoteComment() {
    if (!quotePopup) return;
    quoteText = quotePopup.text;
    quotePopup = null;
    window.getSelection()?.removeAllRanges();
    // Scroll to comment form
    document.querySelector('.comment-form')?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  function scrollToQuote(text: string) {
    if (!contentEl) return;
    // Use TreeWalker to find the text node
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
        // Scroll the found text into view
        const rect = range.getBoundingClientRect();
        window.scrollTo({ top: window.scrollY + rect.top - 120, behavior: 'smooth' });
        // Highlight briefly
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

  async function renderKatex(el: HTMLDivElement) {
    const katex = await import('katex');
    import('katex/dist/katex.min.css');

    // Render inline math
    el.querySelectorAll('.katex-inline').forEach(span => {
      const tex = span.textContent || '';
      try {
        katex.default.render(tex, span as HTMLElement, { throwOnError: false, displayMode: false });
      } catch { /* ignore */ }
    });

    // Render display math
    el.querySelectorAll('.katex-display').forEach(div => {
      const tex = div.textContent || '';
      try {
        katex.default.render(tex, div as HTMLElement, { throwOnError: false, displayMode: true });
      } catch { /* ignore */ }
    });
  }

  function convertFootnotesToSidenotes(el: HTMLDivElement) {
    // Typst HTML output:
    //   refs: <a id="loc-1" href="#loc-3" role="doc-noteref"><sup>1</sup></a>
    //   endnotes: <section role="doc-endnotes"><ol><li id="loc-3"><a href="#loc-1" role="doc-backlink"><sup>1</sup></a>Content</li></ol></section>
    const fnSection = el.querySelector('section[role="doc-endnotes"], .footnotes');
    if (!fnSection) return;

    // Build map: endnote id -> content (stripping backlink)
    const fnItems = fnSection.querySelectorAll('li[id]');
    const fnMap = new Map<string, string>();
    fnItems.forEach(li => {
      const clone = li.cloneNode(true) as HTMLLIElement;
      const backLink = clone.querySelector('a[role="doc-backlink"]');
      if (backLink) backLink.remove();
      fnMap.set(clone.id, clone.innerHTML.trim());
    });

    let counter = 0;
    // Find all footnote references
    const refs = el.querySelectorAll('a[role="doc-noteref"]');
    refs.forEach(a => {
      const href = (a as HTMLAnchorElement).getAttribute('href');
      if (!href) return;
      const fnId = href.slice(1); // remove #
      const fnContent = fnMap.get(fnId);
      if (!fnContent) return;

      counter++;

      // Create sidenote elements
      const label = document.createElement('label');
      label.htmlFor = `sn-${counter}`;
      label.className = 'margin-toggle sidenote-number';

      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.id = `sn-${counter}`;
      checkbox.className = 'margin-toggle';

      const sidenote = document.createElement('span');
      sidenote.className = 'sidenote';
      sidenote.innerHTML = fnContent;

      // Replace the <a> (which contains <sup>)
      a.replaceWith(label, checkbox, sidenote);
    });

    // Remove the endnotes section
    fnSection.remove();
  }
</script>

{#if error}
  <div class="empty"><p>Error: {error}</p></div>
{:else if !article}
  <p class="meta">Loading...</p>
{:else}
  <div class="article-layout">
    <!-- Left floating TOC + forks -->
    {#if tocItems.length > 0 || topForks.length > 0}
      <aside class="toc-box">
        <div class="toc-sticky">
          {#if tocItems.length > 0}
            <nav class="toc">
              <ul>
                {#each tocItems as item}
                  <li class="toc-{item.level}" class:active={activeId === item.id}>
                    <a href="#{item.id}">{item.text}</a>
                  </li>
                {/each}
              </ul>
            </nav>
          {/if}
          {#if topForks.length > 0}
            <div class="sidebar-forks">
              <span class="sidebar-forks-title">Forks ({forks.length})</span>
              {#each topForks as f}
                <a href="#/article?uri={encodeURIComponent(f.forked_uri)}" class="sidebar-fork-item">
                  <span class="sf-title">{f.title}</span>
                  <span class="sf-meta">
                    {f.author_handle ? `@${f.author_handle}` : f.did.slice(0, 16) + '…'}
                    <span class="sf-score">+{f.vote_score}</span>
                  </span>
                </a>
              {/each}
              {#if forks.length > 3}
                <a href="#/forks?uri={encodeURIComponent(uri)}" class="sidebar-fork-more">{t('article.viewAllForks', forks.length)}</a>
              {/if}
            </div>
          {/if}
        </div>
      </aside>
    {/if}

    <!-- Series navigation arrows (fixed on sides) -->
    {#each seriesContext as ctx}
      {#if ctx.prev.length > 0}
        <a href="#/article?uri={encodeURIComponent(ctx.prev[0].article_uri)}" class="series-nav series-prev" title={t('article.seriesPrev', ctx.prev[0].title)}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6"/></svg>
        </a>
      {/if}
      {#if ctx.next.length > 0}
        <a href="#/article?uri={encodeURIComponent(ctx.next[0].article_uri)}" class="series-nav series-next" title={t('article.seriesNext', ctx.next[0].title)}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6"/></svg>
        </a>
      {/if}
    {/each}

    <!-- Main article -->
    <article>
      <!-- Series banner -->
      {#if seriesContext.length > 0}
        {#each seriesContext as ctx}
          <div class="series-banner">
            <a href="#/series?id={encodeURIComponent(ctx.series_id)}" class="series-link">{ctx.series_title}</a>
            <span class="series-pos">{t('article.seriesCount', ctx.total)}</span>
            <div class="series-nav-inline">
              {#each ctx.prev as p}
                <a href="#/article?uri={encodeURIComponent(p.article_uri)}" class="nav-link prev">← {p.title}</a>
              {/each}
              {#each ctx.next as n}
                <a href="#/article?uri={encodeURIComponent(n.article_uri)}" class="nav-link next">{n.title} →</a>
              {/each}
            </div>
          </div>
        {/each}
      {/if}

      <h1 class="article-title">{article.title}</h1>

      {#if translations.length > 0}
        <div class="lang-switcher">
          <span class="lang-current">{LANG_NAMES[article.lang] || article.lang}</span>
          {#each translations as tr}
            <a href="#/article?uri={encodeURIComponent(tr.at_uri)}" class="lang-option">{LANG_NAMES[tr.lang] || tr.lang}</a>
          {/each}
        </div>
      {/if}

      <div class="article-meta">
        <a href="#/profile?did={encodeURIComponent(article.did)}" class="author-link">{article.author_handle ? `@${article.author_handle}` : article.did}</a>
        <span>{article.created_at.split(' ')[0]}</span>
        <span>{article.content_format}</span>
        <span>{article.license}</span>
        {#if prereqs.length > 0}
          <span class="prereq-sep">|</span>
          {#each prereqs as p}
            <span class="tag {p.prereq_type}">{tagName(p.tag_names, p.tag_name, p.tag_id)}</span>
          {/each}
        {/if}
      </div>

      {#if content}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="content" bind:this={contentEl} onmouseup={onContentMouseUp}>{@html content.html}</div>
      {/if}

      {#if quotePopup}
        <button
          class="quote-popup"
          style="left: {quotePopup.x}px; top: {quotePopup.y}px;"
          onclick={startQuoteComment}
        >
          {t('comments.quoteHint')}
        </button>
      {/if}

      <div class="action-bar">
        <div class="action-group">
          <button class="action-btn" class:active={myVote > 0} onclick={() => doVote(1)} disabled={!isLoggedIn} title={isLoggedIn ? t('article.upvote') : t('article.loginToVote')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill={myVote > 0 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14z"/><path d="M7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3"/></svg>
          </button>
          <span class="action-score">{votes?.score ?? 0}</span>
          <button class="action-btn" class:active={myVote < 0} onclick={() => doVote(-1)} disabled={!isLoggedIn} title={isLoggedIn ? t('article.downvote') : t('article.loginToVote')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill={myVote < 0 ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M10 15v4a3 3 0 003 3l4-9V2H5.72a2 2 0 00-2 1.7l-1.38 9a2 2 0 002 2.3H10z"/><path d="M17 2h2.67A2.31 2.31 0 0122 4v7a2.31 2.31 0 01-2.33 2H17"/></svg>
          </button>
        </div>

        <button class="action-btn" class:active={isBookmarked} onclick={toggleBookmark} title={isBookmarked ? t('article.bookmarked') : t('article.bookmark')}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill={isBookmarked ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/></svg>
        </button>

        <button class="action-btn" onclick={doFork} title={t('article.fork')}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9"/><path d="M12 12v3"/></svg>
        </button>

        {#if isOwner}
          <div class="action-separator"></div>
          <button class="action-btn" onclick={doEdit} title={t('article.edit')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button class="action-btn danger" onclick={doDelete} title={t('article.delete')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
          </button>
        {/if}
      </div>

      <!-- Comments -->
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

    </article>
  </div>
{/if}

<style>

  .article-layout {
    position: relative;
  }

  /* Series navigation */
  .series-nav {
    position: fixed;
    top: 50%;
    transform: translateY(-50%);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--bg-white);
    border: 1px solid var(--border);
    color: var(--text-hint);
    text-decoration: none;
    transition: all 0.15s;
    box-shadow: 0 2px 8px rgba(0,0,0,0.08);
  }
  .series-nav:hover {
    color: var(--accent);
    border-color: var(--accent);
    text-decoration: none;
  }
  .series-prev { left: max(1rem, calc(50% - 26rem)); }
  .series-next { right: max(1rem, calc(50% - 26rem)); }

  .series-banner {
    background: var(--bg-gray, #f8f8f8);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 10px 16px;
    margin-bottom: 16px;
    font-size: 13px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }
  .series-link {
    font-family: var(--font-serif);
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
  }
  .series-link:hover { text-decoration: underline; }
  .series-pos {
    color: var(--text-hint);
  }
  .series-nav-inline {
    margin-left: auto;
    display: flex;
    gap: 16px;
  }
  .nav-link {
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 13px;
    white-space: nowrap;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .nav-link:hover { color: var(--accent); }

  @media (max-width: 60rem) {
    .series-nav { display: none; }
  }

  .article-title {
    font-family: var(--font-serif);
    font-size: 2.5rem;
    font-weight: 400;
    line-height: 1.15;
    margin-bottom: 0.5rem;
    color: var(--text-primary);
  }
  .lang-switcher {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 0.5rem;
    font-size: 13px;
  }
  .lang-current {
    color: var(--accent);
    font-weight: 600;
    padding: 2px 8px;
    background: rgba(95, 155, 101, 0.1);
    border-radius: 3px;
  }
  .lang-option {
    color: var(--text-secondary);
    text-decoration: none;
    padding: 2px 8px;
    border-radius: 3px;
    transition: background 0.15s;
  }
  .lang-option:hover {
    background: var(--bg-hover);
    text-decoration: none;
  }
  .article-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    font-size: 14px;
    color: var(--text-secondary);
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }
  .author-link {
    color: var(--text-secondary);
    text-decoration: none;
  }
  .author-link:hover { color: var(--accent); }
  .prereq-sep {
    color: var(--text-hint);
  }
  .action-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2rem;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .action-group {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-right: 6px;
  }
  .action-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    color: var(--text-hint);
    transition: all 0.15s;
  }
  .action-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .action-btn.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .action-btn.danger:hover {
    border-color: #c44;
    color: #c44;
  }
  .action-score {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 20px;
    text-align: center;
  }
  .action-separator {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 4px;
  }

  /* Comments */
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
  /* Sidebar forks */
  .sidebar-forks {
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }
  .sidebar-forks-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-hint);
    display: block;
    margin-bottom: 8px;
  }
  .sidebar-fork-item {
    display: block;
    padding: 4px 0;
    text-decoration: none;
    transition: color 0.15s;
  }
  .sidebar-fork-item:hover { text-decoration: none; }
  .sf-title {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sidebar-fork-item:hover .sf-title { color: var(--accent); }
  .sf-meta {
    font-size: 11px;
    color: var(--text-hint);
  }
  .sf-score {
    color: var(--accent);
    margin-left: 4px;
  }
  .sidebar-fork-more {
    display: block;
    font-size: 12px;
    color: var(--accent);
    margin-top: 6px;
    text-decoration: none;
  }
  .sidebar-fork-more:hover { text-decoration: underline; }


  /* Left floating TOC */
  .toc-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 0;
    height: 100%;
  }
  .toc-sticky {
    position: sticky;
    top: 3rem;
    width: clamp(12rem, calc((100vw - 52rem) / 2 - 3rem), 20rem);
    margin-left: calc(-1 * clamp(12rem, calc((100vw - 52rem) / 2 - 3rem), 20rem) - 2rem);
    max-height: calc(100vh - 4rem);
    overflow-y: auto;
  }
  .toc {
    border-left: 2px solid var(--border);
    padding-left: 0.75rem;
    font-size: 13px;
    font-family: var(--font-sans);
    line-height: 1.5;
  }
  .toc ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .toc li {
    margin: 3px 0;
  }
  .toc a {
    color: var(--text-hint);
    text-decoration: none;
    display: block;
    padding: 2px 0;
    transition: color 0.15s;
  }
  .toc a:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .toc li.active > a {
    color: var(--accent);
    font-weight: 500;
  }
  .toc-3 { padding-left: 0.75rem; }
  .toc-4 { padding-left: 1.5rem; }

  /* Quote comment popup */
  .quote-popup {
    position: fixed;
    transform: translate(-50%, -100%);
    z-index: 50;
    background: var(--text-primary);
    color: var(--bg-white);
    border: none;
    border-radius: 4px;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
    transition: opacity 0.15s;
  }
  .quote-popup:hover { opacity: 0.85; }

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

  /* Temporary highlight when scrolling to quoted text */
  :global(.quote-highlight) {
    background: rgba(95, 155, 101, 0.3);
    border-radius: 2px;
    transition: background 1s ease-out;
  }

  /* Sidenotes */
  :global(.sidenote) {
    float: right;
    clear: right;
    width: clamp(10rem, calc((100vw - 52rem) / 2 - 3rem), 18rem);
    margin-right: calc(-1 * clamp(10rem, calc((100vw - 52rem) / 2 - 3rem), 18rem) - 2rem);
    margin-bottom: 0.75rem;
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    position: relative;
  }
  :global(label.sidenote-number) {
    display: inline;
    margin: 0;
    padding: 0;
    font-size: inherit;
    font-weight: inherit;
    counter-increment: sidenote-counter;
    cursor: pointer;
    text-transform: none;
    letter-spacing: normal;
    color: inherit;
  }
  :global(.sidenote-number::after) {
    content: counter(sidenote-counter);
    font-size: 0.65em;
    vertical-align: super;
    color: var(--accent);
    margin-left: 0.1em;
  }
  :global(.sidenote::before) {
    content: counter(sidenote-counter) ". ";
    font-weight: 600;
    color: var(--accent);
  }
  :global(.margin-toggle) {
    display: none;
  }

  /* Responsive: hide TOC and sidenotes on narrow screens */
  @media (max-width: 75rem) {
    .toc-box {
      display: none;
    }
  }
  @media (max-width: 85rem) {
    :global(.sidenote) {
      display: none;
    }
    :global(.margin-toggle:checked + .sidenote) {
      display: block;
      float: none;
      width: 100%;
      margin: 0.5rem 0 0.5rem 1rem;
      padding: 8px;
      background: rgba(0, 0, 0, 0.02);
      border-left: 2px solid var(--border);
      border-radius: 2px;
    }
    :global(.margin-toggle) {
      display: none;
    }
    :global(.sidenote-number) {
      cursor: pointer;
    }
  }
</style>
