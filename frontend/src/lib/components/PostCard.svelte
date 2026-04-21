<script lang="ts">
  import { tagName, authorName, fmtRep } from '../display';
  import { t } from '../i18n/index.svelte';
  import { getArticleContent, getSeries, castVote, getMyVote, addBookmark, removeBookmark, listArticleAuthors } from '../api';
  import type { ArticleAuthor } from '../api';
  import { getAuth } from '../auth.svelte';
  import { timeAgo } from '../utils';
  import type { Article, ArticleContent, ContentTeachRow, ContentPrereqBulkRow, Series } from '../types';
  import CommentThread from './CommentThread.svelte';
  import { prepareWithSegments, layoutNextLineRange } from '@chenglou/pretext';

  let {
    article = undefined,
    series = undefined,
    articleCount = 0,
    articleTeaches = [],
    articlePrereqs = [],
  }: {
    article?: Article;
    series?: Series;
    articleCount?: number;
    articleTeaches?: ContentTeachRow[];
    articlePrereqs?: ContentPrereqBulkRow[];
  } = $props();

  function navToTag(e: MouseEvent | KeyboardEvent, tagId: string) {
    if (e instanceof KeyboardEvent && e.key !== 'Enter') return;
    e.preventDefault();
    e.stopPropagation();
    window.location.href = `/tag?id=${encodeURIComponent(tagId)}`;
  }

  const fmtTime = timeAgo;

  let expanded = $state(false);
  let expandedContent = $state<ArticleContent | null>(null);
  let expandLoading = $state(false);
  let contentEl = $state<HTMLDivElement | undefined>(undefined);
  let cardEl = $state<HTMLAnchorElement | undefined>(undefined);
  let coverMaxH = $state(180);

  const TITLE_FONT = '600 19.2px "Noto Serif SC", Garamond, serif';
  const SUMMARY_FONT = '14px system-ui, -apple-system, sans-serif';
  const TITLE_LINE_H = 19.2 * 1.35;
  const SUMMARY_LINE_H = 14 * 1.55;
  const META_RESERVE = 30;
  const PADDING_X = 40;
  const MAX_COVER = 220;
  const COVER_GAP = 18;

  // Flow one prepared text block through a two-width layout (narrow column
  // while y < coverH, full width after). Returns bottom y.
  function flowBlock(prep: ReturnType<typeof prepareWithSegments>, lineH: number, narrowW: number, fullW: number, coverH: number, startY: number): number {
    let cursor = { segmentIndex: 0, graphemeIndex: 0 };
    let y = startY;
    for (let guard = 0; guard < 200; guard++) {
      const w = y < coverH ? narrowW : fullW;
      const line = layoutNextLineRange(prep, cursor, w);
      if (!line) break;
      if (line.end.segmentIndex === cursor.segmentIndex && line.end.graphemeIndex === cursor.graphemeIndex) break;
      cursor = line.end;
      y += lineH;
    }
    return y;
  }

  function measureCover() {
    if (!cardEl) return;
    const title = article?.title ?? series?.title ?? '';
    const summary = article?.summary ?? series?.summary ?? '';
    const fullW = cardEl.clientWidth - PADDING_X;
    if (fullW < 100) return;

    const titlePrep = title ? prepareWithSegments(title, TITLE_FONT) : null;
    const summaryPrep = summary ? prepareWithSegments(summary, SUMMARY_FONT) : null;

    // Iterate to a fixed point: find cover height H s.t. H ≤ text height
    // when text wraps around a coverW × H image. coverW = min(MAX_COVER, H)
    // (square assumption, conservative — real covers are never wider than
    // their height unless the source image is landscape, in which case
    // MAX_COVER caps it).
    let H = MAX_COVER;
    for (let iter = 0; iter < 8; iter++) {
      const coverW = Math.min(MAX_COVER, H);
      const narrowW = Math.max(80, fullW - coverW - COVER_GAP);
      let y = 0;
      if (titlePrep) y = flowBlock(titlePrep, TITLE_LINE_H, narrowW, fullW, H, y);
      if (summaryPrep) y = flowBlock(summaryPrep, SUMMARY_LINE_H, narrowW, fullW, H, y);
      y += META_RESERVE;
      const newH = Math.min(MAX_COVER, y);
      if (Math.abs(newH - H) < 2) { H = newH; break; }
      H = newH;
    }
    coverMaxH = Math.round(H);
  }

  $effect(() => {
    if (!cardEl) return;
    measureCover();
    const ro = new ResizeObserver(measureCover);
    ro.observe(cardEl);
    return () => ro.disconnect();
  });


  let expandedTitle = $state('');
  let expandedUri = $state('');
  let expandedVote = $state(0);
  let expandedVoteScore = $state(0);
  let expandedBookmarked = $state(false);
  let expandedAuthors = $state<ArticleAuthor[]>([]);
  let showComments = $state(false);

  // Render KaTeX after expanded content is inserted into DOM
  $effect(() => {
    if (contentEl && expandedContent) {
      import('katex').then(katex => {
        import('katex/dist/katex.min.css');
        contentEl!.querySelectorAll('.katex-inline').forEach(span => {
          const tex = span.textContent || '';
          try { katex.default.render(tex, span as HTMLElement, { throwOnError: false, displayMode: false }); } catch {}
        });
        contentEl!.querySelectorAll('.katex-display').forEach(div => {
          const tex = div.textContent || '';
          try { katex.default.render(tex, div as HTMLElement, { throwOnError: false, displayMode: true }); } catch {}
        });
      });
    }
  });

  async function toggleExpand(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (expanded) {
      expanded = false;
      return;
    }
    if (!expandedContent) {
      expandLoading = true;
      try {
        let uri = '';
        if (article) {
          uri = article.at_uri;
          expandedContent = await getArticleContent(uri);
          expandedTitle = article.title;
          expandedVoteScore = article.vote_score;
          // Fetch full author list (avatar + display_name + role) so the
          // expanded header matches the Article detail page.
          listArticleAuthors(uri).then(a => { expandedAuthors = a; }).catch(() => { expandedAuthors = []; });
        } else if (series) {
          const detail = await getSeries(series.id);
          if (detail.articles.length > 0) {
            uri = detail.articles[0].article_uri;
            expandedContent = await getArticleContent(uri);
            expandedTitle = detail.articles[0].title;
          }
        }
        expandedUri = uri;
        if (uri && getAuth()) {
          getMyVote(uri).then(v => { expandedVote = v.value; }).catch(() => {});
        }
      } catch { /* */ }
      expandLoading = false;
    }
    expanded = true;
  }


  async function doVote(value: number) {
    if (!getAuth() || !expandedUri) return;
    const v = expandedVote === value ? 0 : value;
    try {
      const result = await castVote(expandedUri, v);
      expandedVote = v;
      expandedVoteScore = result.score;
    } catch { /* */ }
  }

  async function doBookmark() {
    if (!getAuth() || !expandedUri) return;
    try {
      if (expandedBookmarked) {
        await removeBookmark(expandedUri);
        expandedBookmarked = false;
      } else {
        await addBookmark(expandedUri);
        expandedBookmarked = true;
      }
    } catch { /* */ }
  }

  function seriesAuthor(s: Series): string {
    if (s.author_handle) return `@${s.author_handle}`;
    return s.created_by.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
  }
</script>

{#if article}
  {@const cardHref =
    article.kind === 'question' ? `/question?uri=${encodeURIComponent(article.at_uri)}`
    : article.kind === 'answer' && article.question_uri ? `/question?uri=${encodeURIComponent(article.question_uri)}#${encodeURIComponent(article.at_uri)}`
    : `/article?uri=${encodeURIComponent(article.at_uri)}`}
  <a href={cardHref} class="post-card" class:hidden={expanded} class:has-cover={!!article.cover_url} bind:this={cardEl}>
    <div class="card-body">
    {#if article.cover_url}
      <div class="post-cover">
        <img src={article.cover_url} alt="" loading="lazy" style:max-height={coverMaxH + 'px'} />
      </div>
    {/if}
    <div class="card-top">
      {#if article.kind === 'question'}
        <span class="question-badge">{t('qa.questionBadge')}</span>
      {:else if article.kind === 'answer'}
        <span class="answer-badge">{t('qa.answerBadge')}</span>
      {:else if article.category === 'review' && article.book_id}
        <a href="/book?id={encodeURIComponent(article.book_id)}" class="review-badge" onclick={(e) => e.stopPropagation()}>{t('article.reviewBadge')}</a>
      {/if}
      <span class="post-title">{article.title}</span>
      {#if article.category === 'paper' && article.paper_accepted && article.paper_venue}
        <!-- Accepted-venue badge for papers: surfaces the publication outlet inline -->
        <span class="venue-badge" title={t('article.acceptedAt')}>
          {article.paper_venue}{#if article.paper_year} {article.paper_year}{/if}
        </span>
      {/if}
    </div>

    {#if articleTeaches.length > 0 || articlePrereqs.length > 0}
      <div class="card-tags">
        {#each articleTeaches as t}
          <span class="tag" role="link" tabindex="0" onclick={(e) => navToTag(e, t.tag_id)} onkeydown={(e) => navToTag(e, t.tag_id)}>{tagName(t.tag_names, t.tag_name, t.tag_id)}</span>
        {/each}
        {#each articlePrereqs as p}
          <span class="tag {p.prereq_type}" role="link" tabindex="0" onclick={(e) => navToTag(e, p.tag_id)} onkeydown={(e) => navToTag(e, p.tag_id)}>{tagName(p.tag_names, p.tag_name, p.tag_id)}</span>
        {/each}
      </div>
    {/if}

    {#if article.summary_html}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      <p class="post-desc">{@html article.summary_html}</p>
    {:else if article.summary}
      <p class="post-desc">{article.summary}</p>
    {/if}

    <div class="card-bottom">
      <span class="post-meta">
        <span class="author-link" role="link" tabindex="0" onclick={(e) => { e.preventDefault(); e.stopPropagation(); window.location.href = `/profile?did=${encodeURIComponent(article.did)}`; }}>
          {#if article.author_avatar}
            <img src={article.author_avatar} alt="" class="post-avatar" />
          {:else}
            <img src="/api/avatars/{encodeURIComponent(article.did)}" alt="" class="post-avatar" onerror={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }} />
          {/if}
          {article.author_display_name || authorName(article)}
          {#if article.author_handle && article.author_display_name}<span class="post-handle">@{article.author_handle}</span>{/if}
        </span>
        {#if article.author_reputation > 0}<span class="rep-badge" title="Reputation">{fmtRep(article.author_reputation)}</span>{/if}
        &middot; {fmtTime(article.created_at)}
      </span>
      <span class="card-stats">
        <span class="stat" title={t('home.votes')}>&#9650; {article.vote_score}</span>
        <span class="stat" title={t('home.bookmarks')}>&#9733; {article.bookmark_count}</span>
        <span class="stat" title="Comments">&#128172; {article.comment_count}</span>
        {#if article.fork_count > 0}<span class="stat" title="Forks">&#9095; {article.fork_count}</span>{/if}
      </span>
    </div>
    <button class="expand-btn" onclick={toggleExpand}>
      {#if expandLoading}...{:else}{expanded ? t('home.collapse') : t('home.expand')}{/if}
    </button>
    </div>
  </a>

  {#if expanded && expandedContent}
    <div class="expanded-full">
      <div class="expanded-header">
        <h1 class="expanded-title"><a href={cardHref}>{expandedTitle}</a></h1>
        <div class="expanded-authors">
          {#if expandedAuthors.length > 0}
            {#each expandedAuthors as au}
              {#if au.author_did}
                <a href="/profile?did={encodeURIComponent(au.author_did)}" class="exp-author-chip">
                  {#if au.author_avatar}
                    <img src={au.author_avatar} alt="" class="exp-author-avatar" />
                  {:else}
                    <span class="exp-author-avatar placeholder">{(au.author_display_name || au.author_handle || '?').charAt(0).toUpperCase()}</span>
                  {/if}
                  <span class="exp-author-name">{au.author_display_name || au.author_handle || '?'}</span>
                  {#if au.is_corresponding}<span class="exp-corr" title="Corresponding">✉</span>{/if}
                </a>
              {:else if au.author_name}
                <span class="exp-author-chip static">
                  <span class="exp-author-avatar placeholder">{au.author_name.charAt(0).toUpperCase()}</span>
                  <span class="exp-author-name">{au.author_name}</span>
                </span>
              {/if}
            {/each}
          {:else}
            <!-- Fallback while authors load — uses the primary author we already have -->
            <a href="/profile?did={encodeURIComponent(article.did)}" class="exp-author-chip">
              {#if article.author_avatar}
                <img src={article.author_avatar} alt="" class="exp-author-avatar" />
              {:else}
                <span class="exp-author-avatar placeholder">{(article.author_display_name || article.author_handle || '?').charAt(0).toUpperCase()}</span>
              {/if}
              <span class="exp-author-name">{article.author_display_name || authorName(article)}</span>
            </a>
          {/if}
        </div>
        <div class="expanded-meta">
          <span>{fmtTime(article.created_at)}</span>
          <span>&middot;</span>
          <span>{article.content_format}</span>
          <button class="collapse-btn" onclick={toggleExpand}>{t('home.collapse')} ▲</button>
        </div>
      </div>
      <div class="content" bind:this={contentEl}>{@html expandedContent.html}</div>
      <div class="expanded-actions">
        <button class="vote-btn" class:active={expandedVote > 0} onclick={() => doVote(1)}>&#9650;</button>
        <span class="vote-score">{expandedVoteScore}</span>
        <button class="vote-btn" class:active={expandedVote < 0} onclick={() => doVote(-1)}>&#9660;</button>
        <button class="bookmark-btn" class:active={expandedBookmarked} onclick={doBookmark}>&#9733;</button>
        <button class="comment-toggle" onclick={() => { showComments = !showComments; }}>
          &#128172; {showComments ? t('qa.hideComments') : t('qa.showComments')}{#if article && article.comment_count > 0} ({article.comment_count}){/if}
        </button>
        <a href={cardHref} class="read-full">{t('home.readFull') || 'Read full →'}</a>
        <button class="collapse-btn" onclick={toggleExpand}>{t('home.collapse')} ▲</button>
      </div>
      {#if showComments && expandedUri}
        <div class="expanded-comments">
          <CommentThread contentUri={expandedUri} />
        </div>
      {/if}
    </div>
  {/if}
{:else if series}
  <a href="/series?id={encodeURIComponent(series.id)}" class="post-card series-card" class:has-cover={!!series.cover_url} bind:this={cardEl}>
    <div class="card-body">
    {#if series.cover_url}
      <div class="post-cover">
        <img src={series.cover_url} alt="" loading="lazy" style:max-height={coverMaxH + 'px'} />
      </div>
    {/if}
    <div class="card-top">
      <span class="post-title">{series.title}</span>
      <span class="series-badge">{t('home.series')}</span>
    </div>

    {#if series.summary_html}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      <p class="post-desc">{@html series.summary_html}</p>
    {:else if series.summary}
      <p class="post-desc">{series.summary}</p>
    {/if}

    <div class="card-bottom">
      <span class="post-meta">
        <span class="author-link" role="link" tabindex="0" onclick={(e) => { e.preventDefault(); e.stopPropagation(); window.location.href = `/profile?did=${encodeURIComponent(series.created_by)}`; }}>
          {#if series.author_avatar}
            <img src={series.author_avatar} alt="" class="post-avatar" />
          {:else}
            <img src="/api/avatars/{encodeURIComponent(series.created_by)}" alt="" class="post-avatar" onerror={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }} />
          {/if}
          {series.author_display_name || seriesAuthor(series)}
          {#if series.author_handle && series.author_display_name}<span class="post-handle">@{series.author_handle}</span>{/if}
        </span>
        &middot; {fmtTime(series.created_at)}
      </span>
      <span class="card-stats">
        <span class="stat">{articleCount} {t('home.lectures')}</span>
        <span class="stat" title={t('home.votes')}>&#9650; {series.vote_score}</span>
        <span class="stat" title={t('home.bookmarks')}>&#9733; {series.bookmark_count}</span>
      </span>
    </div>
    <button class="expand-btn" onclick={toggleExpand}>
      {#if expandLoading}...{:else}{expanded ? t('home.collapse') : t('home.expandFirst')}{/if}
    </button>
    </div>
  </a>

  {#if expanded && expandedContent}
    <div class="expanded-full">
      <div class="expanded-header">
        <h1 class="expanded-title"><a href="/article?uri={encodeURIComponent(expandedUri)}">{expandedTitle}</a></h1>
        <div class="expanded-meta">
          <span>{seriesAuthor(series)}</span>
          <button class="collapse-btn" onclick={toggleExpand}>{t('home.collapse')} ▲</button>
        </div>
      </div>
      <div class="content" bind:this={contentEl}>{@html expandedContent.html}</div>
      <div class="expanded-actions">
        <button class="vote-btn" class:active={expandedVote > 0} onclick={() => doVote(1)}>&#9650;</button>
        <span class="vote-score">{expandedVoteScore}</span>
        <button class="vote-btn" class:active={expandedVote < 0} onclick={() => doVote(-1)}>&#9660;</button>
        <button class="bookmark-btn" class:active={expandedBookmarked} onclick={doBookmark}>&#9733;</button>
        <button class="comment-toggle" onclick={() => { showComments = !showComments; }}>
          &#128172; {showComments ? t('qa.hideComments') : t('qa.showComments')}
        </button>
        <a href="/article?uri={encodeURIComponent(expandedUri)}" class="read-full">{t('home.readFull') || 'Read full →'}</a>
        <button class="collapse-btn" onclick={toggleExpand}>{t('home.collapse')} ▲</button>
      </div>
      {#if showComments && expandedUri}
        <div class="expanded-comments">
          <CommentThread contentUri={expandedUri} />
        </div>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .post-card.hidden {
    display: none;
  }
  .post-card {
    display: block;
    gap: 14px;
    position: relative;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 16px 20px;
    margin-bottom: 24px;
    transition: border-color 0.15s, box-shadow 0.15s;
    text-decoration: none;
    color: inherit;
  }
  .post-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    text-decoration: none;
  }
  .card-body::after {
    content: '';
    display: block;
    clear: both;
  }
  .post-cover {
    float: right;
    margin: 0 0 10px 18px;
    border-radius: 3px;
    overflow: hidden;
    background: var(--bg-hover, #f5f5f5);
  }
  .post-cover img {
    display: block;
    max-width: 220px;
    width: auto;
    height: auto;
  }
  .card-top {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .post-title {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    color: var(--text-primary);
    line-height: 1.35;
    flex: 1;
    min-width: 0;
  }
  .post-card:hover .post-title {
    color: var(--accent);
  }
  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
    margin-top: 6px;
  }
  .post-desc {
    margin: 8px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.55;
  }
  .card-bottom {
    margin-top: 10px;
    display: flex;
    align-items: center;
  }
  .post-meta {
    font-size: 13px;
    color: var(--text-hint);
  }
  .author-link { display: inline-flex; align-items: center; gap: 4px; color: inherit; text-decoration: none; }
  .author-link:hover { color: var(--accent); text-decoration: none; }
  .post-avatar { width: 18px; height: 18px; border-radius: 50%; object-fit: cover; flex-shrink: 0; }
  .post-handle { font-size: 11px; color: var(--text-hint); margin-left: 2px; }
  .rep-badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-page);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    margin-left: 4px;
    vertical-align: baseline;
  }
  .card-stats {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }
  .stat {
    font-size: 12px;
    color: var(--text-hint);
  }

  /* Series card */
  .series-card {
    border-left: 3px solid var(--accent);
  }
  .question-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .answer-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #0d9488;
    background: rgba(13, 148, 136, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .review-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #6366f1;
    background: rgba(99, 102, 241, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
    text-decoration: none;
  }
  .venue-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
    border: 1px solid rgba(95, 155, 101, 0.3);
    padding: 1px 7px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
    align-self: center;
  }
  .review-badge:hover {
    background: rgba(99, 102, 241, 0.2);
    text-decoration: none;
  }
  /* Expand button — bookmark tab that emerges from the card's bottom-right.
     The button body is transparent and overlaps into the card's bottom
     padding so the text rides high. A ::before pseudo-element draws the
     background + side borders + rounded bottom, but only from the card's
     bottom edge downward — the upper portion leaves no border trail inside
     the card. */
  .expand-btn {
    position: absolute;
    right: -1px;                   /* flush with card's outer right edge */
    top: 100%;
    margin-top: -10px;             /* text area reaches 10px into card */
    background: transparent;
    border: none;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    padding: 3px 14px 6px;
    line-height: 1;
    transition: color 0.15s;
    z-index: 1;
  }
  .expand-btn::before {
    /* Bottom half (below card): the visible tab body + accent border. */
    content: '';
    position: absolute;
    top: 10px;                     /* start at card's bottom border line */
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--bg-white);
    border: 1px solid var(--accent);
    border-top: none;              /* no line across card's bottom edge */
    border-radius: 0 0 3px 3px;
    transition: background 0.15s;
    z-index: -1;
  }
  .expand-btn::after {
    /* Top half (inside card's bottom padding): grey outline — same colour
       as the card's border so it reads as a continuation of the card's
       skeleton, not an intrusive accent. */
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 10px;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 3px 3px 0 0;
    pointer-events: none;
    z-index: -1;
  }
  .expand-btn:hover { color: white; }
  .expand-btn:hover::before { background: var(--accent); }

  /* Expanded actions — sticky bottom bar */
  .expanded-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 12px 0 4px;
    border-top: 1px solid var(--border);
    position: sticky;
    bottom: 0;
    background: var(--bg-white);
    z-index: 5;
  }
  .expanded-actions .vote-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 3px 10px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .expanded-actions .vote-btn:hover { border-color: var(--accent); color: var(--accent); }
  .expanded-actions .vote-btn.active { background: var(--accent); color: white; border-color: var(--accent); }
  .expanded-actions .vote-score { font-size: 14px; font-weight: 500; min-width: 20px; text-align: center; }
  .expanded-actions .bookmark-btn {
    background: none; border: 1px solid var(--border); border-radius: 3px;
    padding: 3px 8px; font-size: 13px; color: var(--text-hint); cursor: pointer;
  }
  .expanded-actions .bookmark-btn:hover { border-color: #d4a017; color: #d4a017; }
  .expanded-actions .bookmark-btn.active { background: #d4a017; color: white; border-color: #d4a017; }
  .expanded-actions .comment-toggle {
    background: none; border: 1px solid var(--border); border-radius: 3px;
    padding: 4px 12px; font-size: 13px; color: var(--text-secondary); cursor: pointer; transition: all 0.15s;
  }
  .expanded-actions .comment-toggle:hover { border-color: var(--accent); color: var(--accent); }
  .read-full {
    margin-left: auto;
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
    padding: 4px 14px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    transition: all 0.15s;
  }
  .read-full:hover { background: var(--accent); color: white; text-decoration: none; }
  .expanded-comments {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  /* Full-width expanded article */
  .expanded-full {
    margin-bottom: 24px;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 20px 24px;
    background: var(--bg-white);
  }
  .expanded-header {
    margin-bottom: 16px;
  }
  .expanded-title {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: 400;
    margin: 0 0 8px;
    line-height: 1.3;
  }
  .expanded-title a { color: inherit; text-decoration: none; }
  .expanded-title a:hover { color: var(--accent); }
  .expanded-authors {
    display: flex; align-items: center; flex-wrap: wrap;
    gap: 6px 10px; margin: 8px 0 6px;
  }
  .exp-author-chip {
    display: inline-flex; align-items: center; gap: 5px;
    color: var(--text-primary); text-decoration: none;
    font-size: 13px;
  }
  .exp-author-chip:hover { color: var(--accent); text-decoration: none; }
  .exp-author-chip.static { color: var(--text-secondary); cursor: default; }
  .exp-author-avatar {
    width: 20px; height: 20px; border-radius: 50%; object-fit: cover;
    background: var(--bg-hover, #f5f5f5);
    display: inline-flex; align-items: center; justify-content: center;
    font-size: 10px; font-weight: 600; color: var(--text-secondary);
    flex-shrink: 0;
  }
  .exp-author-name { font-weight: 500; }
  .exp-corr { color: var(--accent); font-size: 11px; }
  .expanded-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-hint);
  }
  .expanded-meta a { color: var(--text-secondary); text-decoration: none; }
  .expanded-meta a:hover { color: var(--accent); }
  .collapse-btn {
    margin-left: auto;
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 3px 10px;
    font-size: 12px;
    color: var(--text-hint);
    cursor: pointer;
  }
  .collapse-btn:hover { border-color: var(--accent); color: var(--accent); }

  .series-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }
</style>
