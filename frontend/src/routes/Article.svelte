<script lang="ts">
  import { getArticleFull, listBookmarks, addBookmark, removeBookmark, castVote, deleteArticle, markLearned as apiMarkLearned, unmarkLearned as apiUnmarkLearned, setRestricted, grantAccess, revokeAccess, listAccessGrants, blockUser as apiBlockUser, createReport, getForkAhead, applyChange, listDiscussions, createDiscussion, listArticleAuthors } from '../lib/api';
  import type { ArticleAuthor } from '../lib/api';
  import type { Discussion } from '../lib/api';
  import ArticleHistory from '../lib/components/ArticleHistory.svelte';
  import { getAuth } from '../lib/auth.svelte';
  import { timeAgo } from '../lib/utils';
  import { tagName } from '../lib/display';
  import { isBlocked, addBlocked } from '../lib/blocklist.svelte';
  import { t, LANG_NAMES } from '../lib/i18n/index.svelte';
  import CommentThread from '../lib/components/CommentThread.svelte';
  import SeriesSidebar from '../lib/components/SeriesSidebar.svelte';
  import type { Article, ArticleContent, ArticlePrereqRow, ForkWithTitle, BookmarkWithTitle, VoteSummary, SeriesContextItem, AccessGrant } from '../lib/types';

  let { uri, seriesId = '' }: { uri: string; seriesId?: string } = $props();

  let article = $state<Article | null>(null);
  let content = $state<ArticleContent | null>(null);
  let prereqs = $state<ArticlePrereqRow[]>([]);
  let forks = $state<ForkWithTitle[]>([]);
  let forkSource = $state<string | null>(null);
  let translations = $state<Article[]>([]);
  let error = $state('');
  let bookmarks = $state<BookmarkWithTitle[]>([]);
  let isBookmarked = $derived(bookmarks.some(b => b.article_uri === uri));
  let votes = $state<VoteSummary | null>(null);
  let myVote = $state(0);
  let isLoggedIn = $derived(!!getAuth());
  let seriesContext = $state<SeriesContextItem[]>([]);
  let isOwner = $derived(!!getAuth() && article?.did === getAuth()?.did);
  let learned = $state(false);
  let accessDenied = $state(false);
  let paperMeta = $state<any>(null);
  let experienceMeta = $state<any>(null);
  let articleAuthors = $state<ArticleAuthor[]>([]);

  let accessGrants = $state<AccessGrant[]>([]);
  let newGrantDid = $state('');
  let reportOpen = $state(false);
  let reportReason = $state('');

  async function doBlockUser() {
    if (!article || !confirm(t('block.confirm'))) return;
    try {
      await apiBlockUser(article.did);
      addBlocked(article.did);
      window.location.href = '/';
    } catch { /* */ }
  }

  async function doReportArticle() {
    if (!article || !reportReason.trim()) return;
    try {
      await createReport(article.did, 'article', reportReason.trim(), article.at_uri);
      reportOpen = false;
      reportReason = '';
      alert(t('report.success'));
    } catch {
      alert(t('report.failed'));
    }
  }

  interface TocItem { id: string; text: string; level: number; }
  let tocItems = $state<TocItem[]>([]);
  let activeId = $state('');

  let contentEl: HTMLDivElement | undefined = $state();
  let topForks = $derived(forks.slice(0, 3));
  let quotePopup = $state<{ x: number; y: number; text: string } | null>(null);

  // Fork ahead changes
  let forkAheadMap = $state(new Map<string, string[]>());
  let forkAheadLoading = $state(new Set<string>());
  let applyingChange = $state('');

  // Discussions / PR
  let prTab = $state<'pr' | 'forks'>('pr');
  let discussions = $state<Discussion[]>([]);
  let openDiscussions = $derived(discussions.filter(d => d.status === 'open'));
  let showCreateDisc = $state(false);
  let discTitle = $state('');
  let discBody = $state('');
  let creatingDisc = $state(false);
  let commentThread: CommentThread | undefined = $state();

  $effect(() => {
    if (!uri) return;
    error = '';
    article = null;
    content = null;
    tocItems = [];
    seriesContext = [];
    translations = [];
    learned = false;
    accessDenied = false;
    accessGrants = [];
    const ac = new AbortController();

    // Single request for all article page data
    getArticleFull(uri).then(data => {
      if (ac.signal.aborted) return;
      article = data.article;
      content = data.content;
      prereqs = data.prereqs;
      forks = data.forks;
      forkSource = data.fork_source;
      votes = { target_uri: uri, score: data.votes.score, upvotes: data.votes.upvotes, downvotes: data.votes.downvotes };
      seriesContext = data.series_context;
      translations = data.translations;
      paperMeta = data.paper || null;
      experienceMeta = data.experience || null;
      myVote = data.my_vote;
      learned = data.learned;
      accessDenied = data.access_denied;
      document.title = `${data.article.title} — NightBoat`;
      listArticleAuthors(uri).then(a => { articleAuthors = a; }).catch(() => {});
      // Load access grants if owner of restricted article
      if (data.article.restricted && data.article.did === getAuth()?.did) {
        listAccessGrants(uri).then(g => { accessGrants = g; }).catch(() => {});
      }
      // Set bookmarked state from server response
      if (data.is_bookmarked) {
        bookmarks = [{ article_uri: uri, folder_path: '/', created_at: '', title: '', description: '' }];
      } else {
        bookmarks = bookmarks.filter(b => b.article_uri !== uri);
      }
      commentThread?.loadComments();
      // Load discussions
      listDiscussions(uri).then(d => { discussions = d; }).catch(() => {});
    }).catch(e => {
      if (ac.signal.aborted) return;
      error = e.message;
    });

    return () => ac.abort();
  });

  // After content renders, extract headings for TOC and convert footnotes to sidenotes
  $effect(() => {
    if (!contentEl || !content) return;

    const headings = contentEl.querySelectorAll('h2, h3, h4');
    const items: TocItem[] = [];
    const usedIds = new Set<string>();
    headings.forEach(h => {
      let id = h.id || h.textContent!.trim()
        .toLowerCase()
        .replace(/[^\w\u4e00-\u9fff]+/g, '-')
        .replace(/^-|-$/g, '');
      let finalId = id;
      let n = 1;
      while (usedIds.has(finalId)) { finalId = `${id}-${n++}`; }
      usedIds.add(finalId);
      h.id = finalId;
      items.push({ id: finalId, text: h.textContent!.trim(), level: parseInt(h.tagName[1]) });
    });
    tocItems = items;

    convertFootnotesToSidenotes(contentEl);
    initPytutorWidgets(contentEl);

    if (article?.content_format === 'markdown') {
      renderKatex(contentEl);
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) { activeId = entry.target.id; }
        }
      },
      { rootMargin: '-80px 0px -70% 0px' }
    );
    headings.forEach(h => observer.observe(h));
    return () => observer.disconnect();
  });

  // --- Actions ---

  async function doVote(value: number) {
    if (!isLoggedIn) return;
    const newValue = myVote === value ? 0 : value;
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

  async function toggleLearned() {
    if (learned) {
      await apiUnmarkLearned(uri);
    } else {
      await apiMarkLearned(uri);
    }
    learned = !learned;
  }

  function doFork() {
    if (!article) return;
    window.location.href = `/new?fork_of=${encodeURIComponent(article.at_uri)}`;
  }

  async function loadForkAhead(forkUri: string) {
    if (forkAheadMap.has(forkUri)) {
      // Toggle: clear if already loaded
      forkAheadMap.delete(forkUri);
      forkAheadMap = new Map(forkAheadMap);
      return;
    }
    forkAheadLoading.add(forkUri);
    forkAheadLoading = new Set(forkAheadLoading);
    try {
      const ahead = await getForkAhead(forkUri, uri);
      forkAheadMap.set(forkUri, ahead);
      forkAheadMap = new Map(forkAheadMap);
    } catch { /* */ }
    forkAheadLoading.delete(forkUri);
    forkAheadLoading = new Set(forkAheadLoading);
  }

  async function applyForkChange(forkUri: string, changeHash: string) {
    applyingChange = changeHash;
    try {
      const result = await applyChange({ source_uri: forkUri, target_uri: uri, change_hash: changeHash });
      if (result.has_conflicts) {
        alert('应用成功但存在冲突，请编辑解决');
      }
      // Reload article content and fork ahead
      const full = await getArticleFull(uri);
      if (full.content) content = full.content;
      // Refresh ahead list
      const ahead = await getForkAhead(forkUri, uri);
      forkAheadMap.set(forkUri, ahead);
      forkAheadMap = new Map(forkAheadMap);
    } catch { /* */ }
    applyingChange = '';
  }

  async function doCreateDiscussion() {
    if (!article || !forkSource || !discTitle.trim()) return;
    creatingDisc = true;
    try {
      const ahead = await getForkAhead(uri, forkSource);
      const disc = await createDiscussion({
        target_uri: forkSource,
        source_uri: uri,
        title: discTitle.trim(),
        body: discBody.trim() || undefined,
        change_hashes: ahead,
      });
      showCreateDisc = false;
      discTitle = '';
      discBody = '';
      window.location.href = `/discussion?id=${encodeURIComponent(disc.id)}`;
    } catch { /* */ }
    creatingDisc = false;
  }

  function doEdit() {
    if (!article) return;
    window.location.href = `/new?edit=${encodeURIComponent(article.at_uri)}`;
  }

  async function toggleRestricted() {
    if (!article) return;
    const newVal = !article.restricted;
    await setRestricted(uri, newVal);
    article = { ...article, restricted: newVal };
    if (newVal) {
      accessGrants = await listAccessGrants(uri);
    }
  }

  async function doGrantAccess() {
    if (!newGrantDid.trim()) return;
    await grantAccess(uri, newGrantDid.trim());
    accessGrants = await listAccessGrants(uri);
    newGrantDid = '';
  }

  async function doRevokeAccess(did: string) {
    await revokeAccess(uri, did);
    accessGrants = accessGrants.filter(g => g.grantee_did !== did);
  }

  async function doDelete() {
    if (!article || !confirm(t('article.deleteConfirm'))) return;
    await deleteArticle(uri);
    window.location.href = '/';
  }

  // --- Quote comment ---

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
    commentThread?.setQuoteText(quotePopup.text);
    quotePopup = null;
    window.getSelection()?.removeAllRanges();
    document.querySelector('.comment-form')?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  // --- Content post-processing ---

  async function renderKatex(el: HTMLDivElement) {
    const katex = await import('katex');
    import('katex/dist/katex.min.css');
    el.querySelectorAll('.katex-inline').forEach(span => {
      const tex = span.textContent || '';
      try { katex.default.render(tex, span as HTMLElement, { throwOnError: false, displayMode: false }); } catch { /* ignore */ }
    });
    el.querySelectorAll('.katex-display').forEach(div => {
      const tex = div.textContent || '';
      try { katex.default.render(tex, div as HTMLElement, { throwOnError: false, displayMode: true }); } catch { /* ignore */ }
    });
  }

  function initPytutorWidgets(el: HTMLDivElement) {
    const widgets = el.querySelectorAll('.pytutor-widget[data-trace]');
    if (widgets.length === 0) return;

    widgets.forEach((widget, idx) => {
      const traceB64 = widget.getAttribute('data-trace');
      if (!traceB64) return;

      const iframe = document.createElement('iframe');
      iframe.className = 'pytutor-iframe';
      iframe.sandbox.add('allow-scripts');
      // No allow-same-origin: fully isolated from parent page

      const srcdoc = `<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<link rel="stylesheet" href="/pytutor/pytutor.css">
<style>
  body { margin: 0; padding: 8px; font-family: sans-serif; overflow: hidden; }
  #pytutor-container { width: 100%; }
</style>
</head><body>
<div id="pytutor-container"></div>
<script src="/pytutor/jquery.min.js"><\/script>
<script src="/pytutor/jquery-ui.min.js"><\/script>
<script src="/pytutor/jquery.ba-bbq.min.js"><\/script>
<script src="/pytutor/jquery.jsPlumb.min.js"><\/script>
<script src="/pytutor/d3.v2.min.js"><\/script>
<script src="/pytutor/codemirror.js"><\/script>
<script src="/pytutor/codemirror-python.js"><\/script>
<script src="/pytutor/pytutor.js"><\/script>
<script>
try {
  var trace = JSON.parse(atob("${traceB64}"));
  var vis = new ExecutionVisualizer('pytutor-container', trace, {
    embeddedMode: true,
    heightChangeCallback: function() {
      var h = document.getElementById('pytutor-container').scrollHeight + 16;
      window.parent.postMessage({ type: 'pytutor-resize', idx: ${idx}, height: h }, '*');
    }
  });
  // Initial resize after render
  setTimeout(function() {
    var h = document.getElementById('pytutor-container').scrollHeight + 16;
    window.parent.postMessage({ type: 'pytutor-resize', idx: ${idx}, height: h }, '*');
  }, 200);
} catch(e) { document.body.textContent = 'Failed to load visualization'; }
<\/script>
</body></html>`;

      iframe.srcdoc = srcdoc;
      iframe.style.width = '100%';
      iframe.style.border = '1px solid var(--border, #ddd)';
      iframe.style.borderRadius = '4px';
      iframe.style.minHeight = '400px';
      iframe.setAttribute('data-pytutor-idx', String(idx));

      widget.replaceWith(iframe);
    });

    // Listen for resize messages from sandboxed iframes
    const resizeHandler = (e: MessageEvent) => {
      if (e.data?.type === 'pytutor-resize') {
        const iframe = el.querySelector(`iframe[data-pytutor-idx="${e.data.idx}"]`) as HTMLIFrameElement;
        if (iframe) iframe.style.height = e.data.height + 'px';
      }
    };
    window.addEventListener('message', resizeHandler);
    // Cleanup on effect re-run would need a separate mechanism, but iframes are replaced on content change
  }

  function convertFootnotesToSidenotes(el: HTMLDivElement) {
    // Collect footnote definitions — pulldown_cmark uses div.footnote-definition,
    // other renderers use section[role="doc-endnotes"] > li[id]
    const fnMap = new Map<string, string>();
    const fnDefs = el.querySelectorAll('div.footnote-definition');
    const fnSection = el.querySelector('section[role="doc-endnotes"], .footnotes');

    if (fnDefs.length > 0) {
      // pulldown_cmark format: <div class="footnote-definition" id="1"><sup>1</sup><p>content</p></div>
      fnDefs.forEach(def => {
        const id = def.id;
        if (!id) return;
        const clone = def.cloneNode(true) as HTMLElement;
        // Remove the label <sup>
        const labelSup = clone.querySelector('sup.footnote-definition-label');
        if (labelSup) labelSup.remove();
        fnMap.set(id, clone.innerHTML.trim());
      });
    } else if (fnSection) {
      // Standard format: section > ol > li[id]
      const fnItems = fnSection.querySelectorAll('li[id]');
      fnItems.forEach(li => {
        const clone = li.cloneNode(true) as HTMLLIElement;
        const backLink = clone.querySelector('a[role="doc-backlink"]');
        if (backLink) backLink.remove();
        fnMap.set(clone.id, clone.innerHTML.trim());
      });
    }

    if (fnMap.size === 0) return;

    // Replace references — pulldown_cmark: sup.footnote-reference > a,
    // standard: a[role="doc-noteref"]
    let counter = 0;
    const endnotes: { num: number; html: string }[] = [];
    const refs = el.querySelectorAll('sup.footnote-reference > a, a[role="doc-noteref"]');
    refs.forEach(a => {
      const href = (a as HTMLAnchorElement).getAttribute('href');
      if (!href) return;
      const fnId = href.slice(1);
      const fnContent = fnMap.get(fnId);
      if (!fnContent) return;

      counter++;
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

      endnotes.push({ num: counter, html: fnContent });

      // For pulldown_cmark, replace the parent <sup>, not just the <a>
      const parent = a.parentElement;
      if (parent && parent.tagName === 'SUP' && parent.classList.contains('footnote-reference')) {
        parent.replaceWith(label, checkbox, sidenote);
      } else {
        a.replaceWith(label, checkbox, sidenote);
      }
    });

    // Remove footnote definitions from the bottom
    fnDefs.forEach(def => def.remove());
    if (fnSection) fnSection.remove();

    // Build endnotes section (shown when sidenotes can't fit in margin)
    if (endnotes.length > 0) {
      const section = document.createElement('section');
      section.className = 'endnotes';
      const hr = document.createElement('hr');
      section.appendChild(hr);
      const ol = document.createElement('ol');
      for (const en of endnotes) {
        const li = document.createElement('li');
        li.innerHTML = en.html;
        ol.appendChild(li);
      }
      section.appendChild(ol);
      el.appendChild(section);
    }
  }
</script>

{#if error}
  <div class="empty"><p>Error: {error}</p></div>
{:else if !article}
  <p class="meta">Loading...</p>
{:else}
  <div class="article-layout">
    {#if tocItems.length > 0 || seriesId || topForks.length > 0}
      <aside class="article-sidebar">
        <div class="sidebar-sticky">
          {#if tocItems.length > 0}
            <nav class="toc">
              <ul>
                {#each tocItems as item}
                  <li class="toc-{item.level}" class:active={activeId === item.id}>
                    <a href="javascript:void(0)" onclick={(e: MouseEvent) => { e.preventDefault(); document.getElementById(item.id)?.scrollIntoView({ behavior: 'smooth', block: 'start' }); }}>{item.text}</a>
                  </li>
                {/each}
              </ul>
            </nav>
          {/if}
          {#if seriesId}
            <div class="sidebar-series">
              <SeriesSidebar {seriesId} currentUri={uri} />
            </div>
          {/if}
          {#if topForks.length > 0}
            <div class="sidebar-forks">
              <span class="sidebar-forks-title">Forks ({forks.length})</span>
              {#each topForks as f}
                <a href="/article?uri={encodeURIComponent(f.forked_uri)}" class="sidebar-fork-item">
                  <span class="sf-title">{f.title}</span>
                  <span class="sf-meta">
                    {f.author_handle ? `@${f.author_handle}` : f.did.slice(0, 16) + '…'}
                    <span class="sf-score">+{f.vote_score}</span>
                  </span>
                </a>
              {/each}
              {#if forks.length > 3}
                <a href="/forks?uri={encodeURIComponent(uri)}" class="sidebar-fork-more">{t('article.viewAllForks', forks.length)}</a>
              {/if}
            </div>
          {/if}
        </div>
      </aside>
    {/if}

    <!-- Series navigation arrows (fixed on sides, hidden when sidebar is shown) -->
    {#if !seriesId}
    {#each seriesContext as ctx}
      {#if ctx.prev.length > 0}
        <a href="/article?uri={encodeURIComponent(ctx.prev[0].article_uri)}{seriesId ? `&series_id=${encodeURIComponent(seriesId)}` : ''}" class="series-nav series-prev" title={t('article.seriesPrev', ctx.prev[0].title)}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6"/></svg>
        </a>
      {/if}
      {#if ctx.next.length > 0}
        <a href="/article?uri={encodeURIComponent(ctx.next[0].article_uri)}{seriesId ? `&series_id=${encodeURIComponent(seriesId)}` : ''}" class="series-nav series-next" title={t('article.seriesNext', ctx.next[0].title)}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6"/></svg>
        </a>
      {/if}
    {/each}
    {/if}

    <!-- Main article -->
    <article>
      <!-- Series banner -->
      {#if seriesContext.length > 0}
        {#each seriesContext as ctx}
          <div class="series-banner">
            <a href="/series?id={encodeURIComponent(ctx.series_id)}" class="series-link">{ctx.series_title}</a>
            <span class="series-pos">{t('article.seriesCount', ctx.total)}</span>
            <div class="series-nav-inline">
              {#each ctx.prev as p}
                <a href="/article?uri={encodeURIComponent(p.article_uri)}{seriesId ? `&series_id=${encodeURIComponent(seriesId)}` : ''}" class="nav-link prev">← {p.title}</a>
              {/each}
              {#each ctx.next as n}
                <a href="/article?uri={encodeURIComponent(n.article_uri)}{seriesId ? `&series_id=${encodeURIComponent(seriesId)}` : ''}" class="nav-link next">{n.title} →</a>
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
            <a href="/article?uri={encodeURIComponent(tr.at_uri)}" class="lang-option">{LANG_NAMES[tr.lang] || tr.lang}</a>
          {/each}
        </div>
      {/if}

      <!-- Authors -->
      {#if articleAuthors.length > 0}
        <div class="article-authors">
          {#each articleAuthors as au}
            {#if au.author_did}
              <a href="/profile?did={encodeURIComponent(au.author_did)}" class="author-chip">
                {#if au.author_avatar}
                  <img src={au.author_avatar} alt="" class="author-avatar" />
                {:else}
                  <span class="author-avatar placeholder">{(au.author_display_name || au.author_handle || '?').charAt(0)}</span>
                {/if}
                <span class="author-info">
                  <span class="author-display-name">{au.author_display_name || au.author_handle || au.author_did.slice(0, 16)}</span>
                  {#if au.author_handle}<span class="author-handle">@{au.author_handle}</span>{/if}
                </span>
                {#if au.is_corresponding}<span class="author-corresponding" title="Corresponding author">✉</span>{/if}
                {#if au.status === 'verified'}<span class="author-verified" title="Verified">✓</span>{/if}
              </a>
            {:else if au.author_name}
              <span class="author-chip text-only">
                <span class="author-avatar placeholder">{au.author_name.charAt(0)}</span>
                <span class="author-display-name">{au.author_name}</span>
              </span>
            {/if}
          {/each}
        </div>
      {:else}
        <div class="article-authors">
          <a href="/profile?did={encodeURIComponent(article.did)}" class="author-chip">
            <span class="author-avatar placeholder">{(article.author_handle || '?').charAt(0)}</span>
            <span class="author-display-name">{article.author_handle || article.did}</span>
          </a>
        </div>
      {/if}

      <div class="article-meta">
        <span>{timeAgo(article.created_at)}</span>
        {#if paperMeta}
          {#if paperMeta.venue}
            <span class="meta-sep">&middot;</span>
            <span class="meta-badge venue">{paperMeta.venue}{#if paperMeta.year} {paperMeta.year}{/if}</span>
          {/if}
          {#if paperMeta.accepted}<span class="meta-badge accepted">{t('article.accepted') || 'Accepted'}</span>{/if}
          {#if paperMeta.doi}<a href="https://doi.org/{paperMeta.doi}" target="_blank" rel="noopener" class="meta-link">DOI</a>{/if}
          {#if paperMeta.arxiv_id}<a href="https://arxiv.org/abs/{paperMeta.arxiv_id}" target="_blank" rel="noopener" class="meta-link">arXiv</a>{/if}
        {/if}
        {#if experienceMeta}
          {#if experienceMeta.kind}<span class="meta-sep">&middot;</span><span class="meta-badge">{experienceMeta.kind}</span>{/if}
          {#if experienceMeta.target}<span class="meta-label">{experienceMeta.target}</span>{/if}
          {#if experienceMeta.result}<span class="meta-badge {experienceMeta.result}">{experienceMeta.result}</span>{/if}
        {/if}
        <span class="meta-sep">&middot;</span>
        <span>{article.license}</span>
        {#if prereqs.length > 0}
          <span class="prereq-sep">|</span>
          {#each prereqs as p}
            <span class="tag {p.prereq_type}">{tagName(p.tag_names, p.tag_name, p.tag_id)}</span>
          {/each}
        {/if}
      </div>

      {#if accessDenied}
        <div class="paywall-notice">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0110 0v4"/></svg>
          <p>{t('article.restricted')}</p>
        </div>
      {:else if content}
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

      <!-- Action bar -->
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

        <button class="action-btn labeled-btn" class:active={isBookmarked} onclick={toggleBookmark} disabled={!isLoggedIn} title={isBookmarked ? t('article.bookmarked') : t('article.bookmark')}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill={isBookmarked ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/></svg>
          <span class="btn-label">{isBookmarked ? t('article.bookmarked') : t('article.bookmark')}</span>
        </button>

        <button class="action-btn labeled-btn" class:active={learned} onclick={toggleLearned} disabled={!isLoggedIn} title={learned ? '已学会' : '标记学会'}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill={learned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
          <span class="btn-label">{learned ? '已学会' : '学会'}</span>
        </button>

        {#if article.license !== 'All-Rights-Reserved'}
          <button class="action-btn labeled-btn" onclick={doFork} disabled={!isLoggedIn} title={t('article.fork')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9"/><path d="M12 12v3"/></svg>
            <span class="btn-label">{t('article.fork')}</span>
          </button>
        {/if}

        {#if forkSource && isOwner}
          <button class="action-btn labeled-btn" onclick={() => { showCreateDisc = true; }} title="提交贡献给原文">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/></svg>
            <span class="btn-label">提交贡献</span>
          </button>
        {/if}

        {#if isLoggedIn && !isOwner}
          <div class="action-separator"></div>
          <button class="action-btn" onclick={() => { reportOpen = true; }} title={t('report.report')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15"/></svg>
          </button>
          <button class="action-btn" onclick={doBlockUser} title={t('block.block')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
          </button>
        {/if}

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

      <!-- Report modal -->
      {#if reportOpen}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="report-overlay" onclick={() => { reportOpen = false; }}>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="report-modal" onclick={(e) => e.stopPropagation()}>
            <h3>{t('report.title')}</h3>
            <p class="report-target">{t('report.kindArticle')}: {article.title}</p>
            <textarea
              bind:value={reportReason}
              placeholder={t('report.reasonPlaceholder')}
              class="report-textarea"
              rows="4"
            ></textarea>
            <div class="report-actions">
              <button class="report-cancel" onclick={() => { reportOpen = false; }}>{t('common.cancel')}</button>
              <button class="report-submit" onclick={doReportArticle} disabled={!reportReason.trim()}>{t('report.submit')}</button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Access control panel (owner only) -->
      {#if isOwner}
        <div class="access-panel">
          <label class="restricted-toggle">
            <input type="checkbox" checked={article.restricted} onchange={toggleRestricted} />
            {t('article.restrictedToggle')}
          </label>
          {#if article.restricted}
            <div class="grant-list">
              <h4>{t('article.accessList')}</h4>
              <div class="grant-add">
                <input type="text" bind:value={newGrantDid} placeholder="did:plc:... 或 handle" />
                <button onclick={doGrantAccess}>{t('article.grantAccess')}</button>
              </div>
              {#each accessGrants as g (g.grantee_did)}
                <div class="grant-item">
                  <span>{g.grantee_did}</span>
                  <button class="revoke-btn" onclick={() => doRevokeAccess(g.grantee_did)}>{t('article.revokeAccess')}</button>
                </div>
              {/each}
              {#if accessGrants.length === 0}
                <p class="no-grants">{t('article.noGrants')}</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Create discussion modal -->
      {#if showCreateDisc}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="report-overlay" onclick={() => { showCreateDisc = false; }}>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="report-modal" onclick={(e) => e.stopPropagation()}>
            <h3>提交贡献</h3>
            <p class="disc-hint">将此 fork 的修改提交给原文作者审查</p>
            <input type="text" class="disc-title-input" placeholder="标题（例如：修正定理 3.2 的证明）" bind:value={discTitle} />
            <textarea class="disc-body-input" placeholder="说明（可选）" bind:value={discBody} rows="3"></textarea>
            <div class="disc-modal-actions">
              <button onclick={() => { showCreateDisc = false; }}>取消</button>
              <button class="disc-submit-btn" onclick={doCreateDiscussion} disabled={creatingDisc || !discTitle.trim()}>
                {creatingDisc ? '提交中...' : '提交'}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Version history -->
      {#if isOwner || forkSource}
        <details class="history-section">
          <summary>{t('article.versionHistory')}</summary>
          <div class="history-wrap">
            <ArticleHistory {uri} {isOwner} applyTargetUri={forkSource || ''} />
          </div>
        </details>
      {/if}

      <!-- Comments -->
      <CommentThread bind:this={commentThread} contentUri={uri} {contentEl} />

      <!-- Pull Requests & Forks -->
      {#if discussions.length > 0 || forks.length > 0}
        <details class="pr-section">
          <summary>
            Pull Requests & Forks
            {#if openDiscussions.length > 0}
              <span class="pr-count">{openDiscussions.length} PR</span>
            {/if}
            {#if forks.length > 0}
              <span class="pr-forks-count">{forks.length} Forks</span>
            {/if}
          </summary>
          <div class="pr-body">
            <!-- PR tab bar -->
            <div class="pr-tabs">
              <button class="pr-tab" class:active={prTab === 'pr'} onclick={() => { prTab = 'pr'; }}>
                PR ({discussions.length})
              </button>
              <button class="pr-tab" class:active={prTab === 'forks'} onclick={() => { prTab = 'forks'; }}>
                Forks ({forks.length})
              </button>
            </div>

            {#if prTab === 'pr'}
              {#if discussions.length === 0}
                <p class="pr-empty">暂无 Pull Request</p>
              {:else}
                <div class="pr-list">
                  {#each discussions as d (d.id)}
                    <a href="/discussion?id={encodeURIComponent(d.id)}" class="disc-link">
                      <span class="disc-link-title">{d.title}</span>
                      <span class="disc-link-status {d.status === 'open' ? 'status-open' : d.status === 'merged' ? 'status-merged' : 'status-closed'}">
                        {d.status === 'open' ? '开放' : d.status === 'merged' ? '已合并' : '已关闭'}
                      </span>
                      <span class="disc-link-date">{d.created_at.split('T')[0]}</span>
                    </a>
                  {/each}
                </div>
              {/if}
            {:else}
              {#if forks.length === 0}
                <p class="pr-empty">暂无 Fork</p>
              {:else}
                <div class="pr-fork-list">
                  {#each forks as f (f.fork_uri)}
                    <div class="fork-contrib-item">
                      <div class="fork-contrib-header">
                        <a href="/article?uri={encodeURIComponent(f.forked_uri)}" class="fork-contrib-title">{f.title}</a>
                        <span class="fork-contrib-author">{f.author_handle ? `@${f.author_handle}` : f.did.slice(0, 16) + '…'}</span>
                        <span class="fork-contrib-score">+{f.vote_score}</span>
                        <button
                          class="fork-expand-btn"
                          onclick={() => loadForkAhead(f.forked_uri)}
                          disabled={forkAheadLoading.has(f.forked_uri)}
                        >
                          {#if forkAheadLoading.has(f.forked_uri)}
                            ...
                          {:else if forkAheadMap.has(f.forked_uri)}
                            收起
                          {:else}
                            查看 changes
                          {/if}
                        </button>
                      </div>
                      {#if forkAheadMap.has(f.forked_uri)}
                        {@const ahead = forkAheadMap.get(f.forked_uri)!}
                        {#if ahead.length === 0}
                          <p class="fork-no-changes">与原文完全同步</p>
                        {:else}
                          <div class="fork-changes-list">
                            {#each ahead as hash}
                              <div class="fork-change-row">
                                <code class="fork-change-hash">{hash.slice(0, 16)}…</code>
                                {#if isOwner}
                                  <button
                                    class="fork-apply-btn"
                                    onclick={() => applyForkChange(f.forked_uri, hash)}
                                    disabled={applyingChange === hash}
                                  >
                                    {applyingChange === hash ? '...' : '应用'}
                                  </button>
                                {/if}
                              </div>
                            {/each}
                          </div>
                        {/if}
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            {/if}
          </div>
        </details>
      {/if}

    </article>
  </div>
{/if}

<style>

  .article-sidebar {
    position: absolute;
    left: 0;
    top: 0;
    width: 0;
    height: 100%;
  }
  .sidebar-sticky {
    position: sticky;
    top: 3rem;
    width: clamp(12rem, calc((100vw - 52rem) / 2 - 3rem), 20rem);
    margin-left: calc(-1 * clamp(12rem, calc((100vw - 52rem) / 2 - 3rem), 20rem) - 2rem);
    max-height: calc(100vh - 4rem);
    overflow-y: auto;
  }
  .sidebar-series {
    margin-top: 0.75rem;
    border-top: 1px solid var(--border);
    padding-top: 0.75rem;
  }
  .sidebar-series :global(.series-sidebar) {
    position: static;
    max-height: none;
    overflow-y: visible;
    border-right: none;
    width: auto;
  }

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
  .article-authors {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .author-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 4px;
    border-radius: 20px;
    background: var(--bg-secondary);
    text-decoration: none;
    color: var(--text-primary);
    font-size: 14px;
    transition: background 0.15s;
  }
  .author-chip:hover { background: var(--border); }
  .author-chip.text-only { cursor: default; }
  .author-chip.text-only:hover { background: var(--bg-secondary); }
  .author-avatar {
    width: 28px; height: 28px; border-radius: 50%; object-fit: cover;
  }
  .author-avatar.placeholder {
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--accent); color: white; font-size: 13px; font-weight: 600;
  }
  .author-info { display: flex; flex-direction: column; line-height: 1.2; }
  .author-display-name { font-weight: 500; font-size: 14px; }
  .author-handle { font-size: 11px; color: var(--text-hint); }
  .author-corresponding { font-size: 12px; margin-left: 2px; color: var(--text-secondary); }
  .author-verified { color: var(--accent); font-size: 12px; margin-left: 2px; }
  .meta-sep { color: var(--text-hint); }
  .author-link {
    color: var(--text-secondary);
    text-decoration: none;
  }
  .category-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    font-size: 13px;
    margin-bottom: 1.5rem;
  }
  .meta-badge {
    padding: 2px 8px;
    border-radius: 3px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-weight: 500;
  }
  .meta-badge.venue { background: var(--accent-light, #e8f5e9); color: var(--accent); }
  .meta-badge.accepted { background: #e8f5e9; color: #2e7d32; }
  .meta-badge.rejected, .meta-badge.failed { background: #ffebee; color: #c62828; }
  .meta-badge.passed, .meta-badge.accepted { background: #e8f5e9; color: #2e7d32; }
  .meta-badge.pending { background: #fff3e0; color: #e65100; }
  .meta-label { color: var(--text-hint); }
  .meta-link { color: var(--accent); text-decoration: none; font-size: 12px; }
  .meta-link:hover { text-decoration: underline; }
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
  .labeled-btn {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .labeled-btn.active {
    background: rgba(95, 155, 101, 0.1);
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-label {
    font-size: 12px;
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


  /* Left floating sidebar (TOC, series nav, forks) — positioned in left viewport margin */
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

  /* Responsive: hide sidebar on narrow screens */
  @media (max-width: 75rem) {
    .article-sidebar {
      display: none;
    }
  }
  /* Endnotes section: hidden on wide screens, visible on narrow */
  :global(.endnotes) {
    display: none;
  }
  :global(.endnotes hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 2rem 0 1rem;
  }
  :global(.endnotes ol) {
    padding-left: 1.5em;
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-secondary);
  }
  :global(.endnotes li) {
    margin: 0.5em 0;
  }

  /* Narrow screen: hide margin sidenotes, show endnotes, allow inline toggle */
  @media (max-width: 60rem) {
    :global(.sidenote) {
      display: none;
    }
    :global(.margin-toggle:checked + .sidenote) {
      display: block;
      float: none;
      width: auto;
      margin: 0.3rem 0 0.5rem 1rem;
      padding: 6px 8px;
      background: rgba(0, 0, 0, 0.02);
      border-left: 2px solid var(--border);
      border-radius: 2px;
    }
    :global(.endnotes) {
      display: block;
    }
  }

  /* Paywall notice */
  .paywall-notice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 48px 24px;
    margin: 24px 0;
    border: 2px dashed var(--border);
    border-radius: 8px;
    color: var(--text-secondary);
    text-align: center;
  }
  .paywall-notice p {
    margin: 0;
    font-size: 15px;
  }

  /* Report modal */
  .report-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    z-index: 400;
    display: flex;
    justify-content: center;
    padding-top: 10vh;
  }
  .report-modal {
    width: 480px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.15);
    align-self: flex-start;
  }
  .report-modal h3 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .report-target {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 10px;
  }
  .report-textarea {
    width: 100%;
    padding: 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .report-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
  .report-cancel {
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .report-submit {
    padding: 5px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .report-submit:disabled { opacity: 0.4; cursor: not-allowed; }

  /* Version history */
  .history-section {
    margin-top: 24px;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .history-section summary {
    padding: 10px 16px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    background: var(--bg-hover);
    user-select: none;
  }
  .history-section summary:hover { color: var(--text-primary); }
  .history-section[open] summary { border-bottom: 1px solid var(--border); }
  .history-wrap { padding: 0; }

  /* Fork contributions */
  .fork-contributions {
    padding: 8px 16px;
  }
  .fork-contrib-item {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .fork-contrib-item:last-child { border-bottom: none; }
  .fork-contrib-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .fork-contrib-title {
    font-family: var(--font-serif);
    font-size: 14px;
    color: var(--text-primary);
    text-decoration: none;
  }
  .fork-contrib-title:hover { color: var(--accent); }
  .fork-contrib-author {
    font-size: 12px;
    color: var(--text-hint);
  }
  .fork-contrib-score {
    font-size: 12px;
    color: var(--accent);
  }
  .fork-expand-btn {
    margin-left: auto;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    font-size: 11px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .fork-expand-btn:hover { border-color: var(--accent); color: var(--accent); }
  .fork-expand-btn:disabled { opacity: 0.5; cursor: wait; }
  .fork-no-changes {
    font-size: 12px;
    color: var(--text-hint);
    margin: 4px 0 0 0;
  }
  .fork-changes-list {
    margin-top: 6px;
  }
  .fork-change-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 0;
  }
  .fork-change-hash {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .fork-apply-btn {
    padding: 2px 10px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: none;
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
  }
  .fork-apply-btn:hover { background: var(--accent); color: white; }
  .fork-apply-btn:disabled { opacity: 0.5; cursor: wait; }

  /* Pull Requests section */
  .pr-section {
    margin-top: 2rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .pr-section summary {
    padding: 10px 16px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    background: var(--bg-hover);
    user-select: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .pr-section summary:hover { color: var(--text-primary); }
  .pr-section[open] summary { border-bottom: 1px solid var(--border); }
  .pr-count {
    font-size: 11px;
    background: var(--accent);
    color: white;
    padding: 1px 6px;
    border-radius: 8px;
    font-weight: 600;
  }
  .pr-body {
    padding: 0;
  }
  .pr-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
  }
  .pr-tab {
    flex: 1;
    padding: 8px 16px;
    border: none;
    background: none;
    font-size: 13px;
    color: var(--text-hint);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
  }
  .pr-tab:hover { color: var(--text-primary); }
  .pr-tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    font-weight: 500;
  }
  .pr-list, .pr-fork-list {
    padding: 8px 16px;
  }
  .pr-empty {
    padding: 16px;
    text-align: center;
    font-size: 13px;
    color: var(--text-hint);
  }
  .pr-forks-count {
    font-size: 11px;
    background: var(--bg-gray, #eee);
    color: var(--text-secondary);
    padding: 1px 6px;
    border-radius: 8px;
  }
  .disc-link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    text-decoration: none;
    border-bottom: 1px solid var(--border);
  }
  .disc-link:last-child { border-bottom: none; }
  .disc-link-title {
    flex: 1;
    font-size: 14px;
    color: var(--text-primary);
  }
  .disc-link:hover .disc-link-title { color: var(--accent); }
  .disc-link-status {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 3px;
  }
  .disc-link-date {
    font-size: 11px;
    color: var(--text-hint);
  }
  .disc-hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 12px;
  }
  .disc-title-input {
    width: 100%;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 14px;
    margin-bottom: 8px;
    box-sizing: border-box;
  }
  .disc-body-input {
    width: 100%;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 13px;
    resize: vertical;
    margin-bottom: 12px;
    box-sizing: border-box;
  }
  .disc-modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .disc-modal-actions button {
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    font-size: 13px;
    cursor: pointer;
  }
  .disc-submit-btn {
    background: var(--accent) !important;
    border-color: var(--accent) !important;
    color: white !important;
  }
  .disc-submit-btn:disabled { opacity: 0.5; cursor: not-allowed !important; }

  /* Access control panel */
  .access-panel {
    margin-top: 24px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
  }
  .restricted-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: 500;
  }
  .grant-list {
    margin-top: 12px;
  }
  .grant-list h4 {
    margin: 0 0 8px;
    font-weight: 500;
  }
  .grant-add {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .grant-add input {
    flex: 1;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 13px;
  }
  .grant-add button {
    padding: 4px 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }
  .grant-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .revoke-btn {
    background: none;
    border: none;
    color: var(--danger, #e53e3e);
    cursor: pointer;
    font-size: 12px;
  }
  .no-grants {
    color: var(--text-hint);
    font-size: 13px;
    margin: 4px 0 0;
  }
</style>
