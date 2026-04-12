<script lang="ts">
  import { listBookmarks, moveBookmark, removeBookmark, getArticlesByDid, listSeries, getAllSeriesArticles, getArticleContent } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { buildSeriesArticleMaps } from '../lib/series';
  import type { BookmarkWithTitle, Article, Series, ArticleContent } from '../lib/types';

  let bookmarks = $state<BookmarkWithTitle[]>([]);
  let myArticles = $state<Article[]>([]);
  let allSeries = $state<Series[]>([]);
  let seriesArticleMap = $state<Map<string, string[]>>(new Map());
  let loading = $state(true);
  let expandedFolders = $state(new Set<string>(['/', `/${t('library.seriesFolder')}`]));
  let newFolderName = $state('');
  let showNewFolder = $state(false);

  // Selected article for rendering
  let selectedUri = $state<string | null>(null);
  let selectedTitle = $state('');
  let articleHtml = $state('');
  let contentLoading = $state(false);

  // Build folder tree from bookmark paths
  let folderTree = $derived.by(() => {
    const folders = new Set<string>(['/']);
    for (const b of allItems) {
      const parts = b.folder_path.split('/').filter(Boolean);
      let path = '';
      for (const p of parts) {
        path += '/' + p;
        folders.add(path);
      }
    }
    return Array.from(folders).sort();
  });

  // Hierarchical folder structure for tree rendering
  type FolderNode = { name: string; path: string; children: FolderNode[]; items: BookmarkWithTitle[] };
  let folderNodes = $derived.by(() => {
    const root: FolderNode = { name: t('nav.library'), path: '/', children: [], items: [] };
    const nodeMap = new Map<string, FolderNode>();
    nodeMap.set('/', root);

    for (const path of folderTree) {
      if (path === '/') continue;
      const parts = path.split('/').filter(Boolean);
      const name = parts[parts.length - 1];
      const parentPath = parts.length > 1 ? '/' + parts.slice(0, -1).join('/') : '/';
      const node: FolderNode = { name, path, children: [], items: [] };
      nodeMap.set(path, node);
      const parent = nodeMap.get(parentPath);
      if (parent) parent.children.push(node);
    }

    // Place items into their folder nodes
    for (const b of allItems) {
      const node = nodeMap.get(b.folder_path);
      if (node) node.items.push(b);
      else root.items.push(b);
    }

    return root;
  });

  $effect(() => {
    const auth = getAuth();
    Promise.all([
      listBookmarks(),
      auth ? getArticlesByDid(auth.did) : Promise.resolve([]),
      listSeries(),
      getAllSeriesArticles(),
    ]).then(([bk, arts, series, sa]) => {
      bookmarks = bk;
      myArticles = arts;
      const did = auth?.did;
      allSeries = did ? series.filter(s => s.created_by === did) : [];
      const saMaps = buildSeriesArticleMaps(sa);
      seriesArticleMap = saMaps.seriesArticleMap;
      loading = false;
    });
  });

  // Build a set of article URIs that belong to any of the user's series
  let seriesArticleUris = $derived.by(() => {
    const uris = new Set<string>();
    for (const s of allSeries) {
      const arts = seriesArticleMap.get(s.id) || [];
      for (const uri of arts) uris.add(uri);
    }
    return uris;
  });

  // "My articles" as virtual bookmark items
  let myArticleItems = $derived.by(() => {
    const items: BookmarkWithTitle[] = [];
    for (const s of allSeries) {
      const articleUris = seriesArticleMap.get(s.id) || [];
      for (const uri of articleUris) {
        const art = myArticles.find(a => a.at_uri === uri);
        if (art) {
          items.push({
            article_uri: art.at_uri,
            folder_path: `/${t('library.seriesFolder')}/${s.title}`,
            created_at: art.created_at,
            title: art.title,
            description: art.description,
          });
        }
      }
    }
    for (const a of myArticles) {
      if (!seriesArticleUris.has(a.at_uri)) {
        items.push({
          article_uri: a.at_uri,
          folder_path: `/${t('profile.works')}`,
          created_at: a.created_at,
          title: a.title,
          description: a.description,
        });
      }
    }
    return items;
  });

  let allItems = $derived([...bookmarks, ...myArticleItems]);

  function toggleFolder(path: string) {
    if (expandedFolders.has(path)) {
      expandedFolders.delete(path);
    } else {
      expandedFolders.add(path);
    }
    expandedFolders = new Set(expandedFolders);
  }

  async function selectArticle(uri: string, title: string) {
    if (selectedUri === uri) return;
    selectedUri = uri;
    selectedTitle = title;
    articleHtml = '';
    contentLoading = true;
    try {
      const c = await getArticleContent(uri);
      articleHtml = c.html;
    } catch {
      articleHtml = `<p style="color:var(--text-hint)">${t('common.loading')} failed</p>`;
    }
    contentLoading = false;
  }

  async function createFolder() {
    const name = newFolderName.trim();
    if (!name) return;
    showNewFolder = false;
    newFolderName = '';
  }

  async function moveToFolder(articleUri: string, folder: string) {
    await moveBookmark(articleUri, folder);
    bookmarks = await listBookmarks();
  }

  async function doRemoveBookmark(articleUri: string) {
    await removeBookmark(articleUri);
    bookmarks = await listBookmarks();
  }

  let dragArticle = $state<string | null>(null);

  function onDragStart(articleUri: string) {
    dragArticle = articleUri;
  }

  function onDrop(folderPath: string) {
    if (dragArticle) {
      moveToFolder(dragArticle, folderPath);
      dragArticle = null;
    }
  }
</script>

<div class="library-layout">
  <aside class="folder-tree">
    <div class="tree-header">
      <span class="tree-title">{t('nav.library')}</span>
      <button class="tree-action" onclick={() => { showNewFolder = !showNewFolder; }} title={t('library.newFolder')}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
      </button>
    </div>

    {#if showNewFolder}
      <div class="new-folder-input">
        <input
          type="text"
          bind:value={newFolderName}
          placeholder={t('library.folderName')}
          onkeydown={(e) => { if (e.key === 'Enter') createFolder(); if (e.key === 'Escape') showNewFolder = false; }}
        />
        <button onclick={createFolder}>{t('common.create')}</button>
      </div>
    {/if}

    {#if loading}
      <p class="tree-loading">{t('common.loading')}</p>
    {:else}
      <nav class="tree-nav">
        {#snippet folderItem(node: FolderNode, depth: number)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="tree-item tree-folder"
            style="padding-left: {8 + depth * 14}px"
            onclick={() => toggleFolder(node.path)}
            ondragover={(e) => e.preventDefault()}
            ondrop={() => onDrop(node.path)}
          >
            {#if node.children.length > 0 || node.items.length > 0}
              <span class="tree-chevron" class:open={expandedFolders.has(node.path)}>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
              </span>
            {:else}
              <span class="tree-spacer"></span>
            {/if}
            <svg class="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
            <span class="tree-name">{node.name}</span>
          </div>

          {#if expandedFolders.has(node.path)}
            {#each node.children as child}
              {@render folderItem(child, depth + 1)}
            {/each}
            {#each node.items as item}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="tree-item tree-file"
                class:selected={selectedUri === item.article_uri}
                style="padding-left: {8 + (depth + 1) * 14}px"
                onclick={() => selectArticle(item.article_uri, item.title)}
                draggable="true"
                ondragstart={() => onDragStart(item.article_uri)}
              >
                <span class="tree-spacer"></span>
                <svg class="tree-icon file-doc" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                </svg>
                <span class="tree-name">{item.title}</span>
              </div>
            {/each}
          {/if}
        {/snippet}
        {@render folderItem(folderNodes, 0)}
      </nav>
    {/if}
  </aside>

  <main class="reader-pane">
    {#if selectedUri}
      <div class="reader-header">
        <h1 class="reader-title">{selectedTitle}</h1>
        <a href="/article?uri={encodeURIComponent(selectedUri)}" class="reader-open" title={t('common.edit')}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
        </a>
      </div>
      {#if contentLoading}
        <p class="meta">{t('common.loading')}</p>
      {:else}
        <div class="content">
          {@html articleHtml}
        </div>
      {/if}
    {:else}
      <div class="reader-empty">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--text-hint)" stroke-width="1" opacity="0.4">
          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
          <line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><line x1="10" y1="9" x2="8" y2="9"/>
        </svg>
        <p>{t('library.selectArticle')}</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .library-layout {
    display: flex;
    gap: 0;
    min-height: calc(100vh - 4rem);
  }

  /* ── Folder tree sidebar ── */
  .folder-tree {
    width: 280px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: rgba(0,0,0,0.015);
    overflow-y: auto;
    max-height: calc(100vh - 4rem);
    position: sticky;
    top: 4rem;
  }
  .tree-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
    border-bottom: 1px solid var(--border);
  }
  .tree-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .tree-action {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    padding: 2px;
    display: flex;
    transition: color 0.15s;
  }
  .tree-action:hover { color: var(--accent); }

  .tree-loading {
    padding: 16px;
    color: var(--text-hint);
    font-size: 13px;
  }

  .new-folder-input {
    display: flex;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  .new-folder-input input {
    flex: 1;
    padding: 3px 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 2px;
    background: var(--bg-white);
    font-family: var(--font-sans);
  }
  .new-folder-input button {
    padding: 3px 8px;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 2px;
    cursor: pointer;
  }

  .tree-nav {
    padding: 4px 0;
  }
  .tree-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    cursor: pointer;
    transition: background 0.1s;
    font-size: 13px;
    color: var(--text-secondary);
    user-select: none;
  }
  .tree-item:hover {
    background: rgba(0,0,0,0.04);
  }
  .tree-folder {
    font-weight: 500;
    color: var(--text-primary);
  }
  .tree-file {
    font-weight: 400;
    color: var(--text-secondary);
  }
  .tree-file.selected {
    background: rgba(95, 155, 101, 0.1);
    color: var(--accent);
  }
  .tree-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    flex-shrink: 0;
    color: var(--text-hint);
    transition: transform 0.15s;
  }
  .tree-chevron.open {
    transform: rotate(90deg);
  }
  .tree-spacer {
    width: 14px;
    flex-shrink: 0;
  }
  .tree-icon {
    flex-shrink: 0;
    color: var(--text-hint);
  }
  .tree-icon.file-doc {
    color: var(--text-hint);
    opacity: 0.6;
  }
  .tree-file.selected .tree-icon.file-doc {
    color: var(--accent);
    opacity: 1;
  }
  .tree-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Reader pane ── */
  .reader-pane {
    flex: 1;
    min-width: 0;
    max-width: 780px;
    margin: 0 auto;
    padding: 0 2rem;
  }
  .reader-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 1.5rem 0 0;
    margin-bottom: 0.5rem;
  }
  .reader-title {
    font-family: var(--font-serif);
    font-size: 1.8rem;
    font-weight: 400;
    margin: 0;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }
  .reader-open {
    color: var(--text-hint);
    flex-shrink: 0;
    padding: 4px;
    transition: color 0.15s;
  }
  .reader-open:hover { color: var(--accent); }

  .reader-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 60vh;
    gap: 12px;
    color: var(--text-hint);
    font-size: 14px;
  }

  @media (max-width: 700px) {
    .folder-tree { width: 200px; }
    .reader-pane { padding: 0 1rem; }
  }
  @media (max-width: 500px) {
    .library-layout { flex-direction: column; }
    .folder-tree {
      width: 100%;
      max-height: 40vh;
      position: static;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
  }
</style>
