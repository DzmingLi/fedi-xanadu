<script lang="ts">
  import { listBookmarks, moveBookmark, removeBookmark, getArticlesByDid, listSeries, getAllSeriesArticles } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { buildSeriesArticleMaps } from '../lib/series';
  import type { BookmarkWithTitle, Article, Series } from '../lib/types';

  let bookmarks = $state<BookmarkWithTitle[]>([]);
  let myArticles = $state<Article[]>([]);
  let allSeries = $state<Series[]>([]);
  let seriesArticleMap = $state<Map<string, string[]>>(new Map());
  let loading = $state(true);
  let expandedFolders = $state(new Set<string>(['/', `/${t('library.seriesFolder')}`]));
  let selectedFolder = $state('/');
  let newFolderName = $state('');
  let showNewFolder = $state(false);

  // Build folder tree from bookmark paths
  let folderTree = $derived.by(() => {
    const folders = new Set<string>(['/']);
    for (const b of allItems) {
      // Add this folder and all parent folders
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
  type FolderNode = { name: string; path: string; children: FolderNode[]; count: number };
  let folderNodes = $derived.by(() => {
    const root: FolderNode = { name: t('nav.library'), path: '/', children: [], count: 0 };
    const nodeMap = new Map<string, FolderNode>();
    nodeMap.set('/', root);

    // Count bookmarks per folder
    const folderCounts = new Map<string, number>();
    for (const b of allItems) {
      folderCounts.set(b.folder_path, (folderCounts.get(b.folder_path) || 0) + 1);
    }
    root.count = allItems.length;

    for (const path of folderTree) {
      if (path === '/') continue;
      const parts = path.split('/').filter(Boolean);
      const name = parts[parts.length - 1];
      const parentPath = parts.length > 1 ? '/' + parts.slice(0, -1).join('/') : '/';
      const node: FolderNode = { name, path, children: [], count: folderCounts.get(path) || 0 };
      nodeMap.set(path, node);
      const parent = nodeMap.get(parentPath);
      if (parent) parent.children.push(node);
    }

    return root;
  });

  // Articles in selected folder (and subfolders)
  let visibleArticles = $derived.by(() => {
    if (selectedFolder === '/') return allItems;
    return allItems.filter(b =>
      b.folder_path === selectedFolder || b.folder_path.startsWith(selectedFolder + '/')
    );
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
      // Filter to user's series
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

  // "My articles" as virtual bookmark items — series articles go into series folders
  let myArticleItems = $derived.by(() => {
    const items: BookmarkWithTitle[] = [];
    // Articles that belong to a series → placed in /${t('library.seriesFolder')}/SeriesTitle folder
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
    // Standalone articles (not in any series)
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

  function selectFolder(path: string) {
    selectedFolder = path;
  }

  async function createFolder() {
    const name = newFolderName.trim();
    if (!name) return;
    const path = selectedFolder === '/' ? '/' + name : selectedFolder + '/' + name;
    // Create folder by moving a placeholder — or just expand it
    // Folders are implicit from bookmark paths, so we just set selected
    expandedFolders.add(selectedFolder);
    expandedFolders = new Set(expandedFolders);
    selectedFolder = path;
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

    <nav class="tree-nav">
      {#snippet folderItem(node: FolderNode, depth: number)}
        <div
          class="tree-item"
          class:selected={selectedFolder === node.path}
          style="padding-left: {8 + depth * 16}px"
          role="treeitem"
          tabindex="0"
          aria-selected={selectedFolder === node.path}
          onclick={() => selectFolder(node.path)}
          onkeydown={(e) => { if (e.key === 'Enter') selectFolder(node.path); }}
          ondragover={(e) => e.preventDefault()}
          ondrop={() => onDrop(node.path)}
        >
          {#if node.children.length > 0}
            <button class="tree-chevron" title={t('article.toggleCollapse')} class:open={expandedFolders.has(node.path)} onclick={(e) => { e.stopPropagation(); toggleFolder(node.path); }}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          {:else}
            <span class="tree-spacer"></span>
          {/if}
          <svg class="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            {#if node.path === '/'}
              <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>
            {:else}
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            {/if}
          </svg>
          <span class="tree-name">{node.name}</span>
          {#if node.count > 0}
            <span class="tree-count">{node.count}</span>
          {/if}
        </div>
        {#if expandedFolders.has(node.path) && node.children.length > 0}
          {#each node.children as child}
            {@render folderItem(child, depth + 1)}
          {/each}
        {/if}
      {/snippet}
      {@render folderItem(folderNodes, 0)}
    </nav>
  </aside>

  <main class="file-list">
    <div class="list-header">
      <span class="list-path">{selectedFolder === '/' ? t('nav.library') : selectedFolder}</span>
      <span class="list-count">{visibleArticles.length}</span>
    </div>

    {#if loading}
      <p class="meta">{t('common.loading')}</p>
    {:else if visibleArticles.length === 0}
      <div class="empty-library">
        <p>{t('library.emptyFolder')}</p>
        <p class="meta">{t('library.emptyHint')}</p>
      </div>
    {:else}
      {#each visibleArticles as b}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <a
          href="#/article?uri={encodeURIComponent(b.article_uri)}"
          class="file-item"
          draggable="true"
          ondragstart={() => onDragStart(b.article_uri)}
        >
          <svg class="file-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
          </svg>
          <div class="file-info">
            <span class="file-title">{b.title}</span>
            {#if b.description}
              <span class="file-desc">{b.description}</span>
            {/if}
          </div>
          <div class="file-actions">
            <span class="file-folder">{b.folder_path}</span>
            <button class="file-remove" onclick={(e) => { e.preventDefault(); e.stopPropagation(); doRemoveBookmark(b.article_uri); }} title={t('library.removeBookmark')}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          </div>
        </a>
      {/each}
    {/if}
  </main>
</div>

<style>
  .library-layout {
    display: flex;
    gap: 0;
    min-height: calc(100vh - 6rem);
  }

  /* Folder tree sidebar */
  .folder-tree {
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: rgba(0,0,0,0.015);
    overflow-y: auto;
  }
  .tree-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
    border-bottom: 1px solid var(--border);
  }
  .tree-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
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
    padding: 4px 8px;
    cursor: pointer;
    transition: background 0.1s;
    font-size: 13px;
    color: var(--text-secondary);
    user-select: none;
  }
  .tree-item:hover {
    background: var(--bg-hover);
  }
  .tree-item.selected {
    background: var(--bg-hover);
    color: var(--text-primary);
    font-weight: 500;
  }
  .tree-chevron {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    color: var(--text-hint);
    transition: transform 0.15s;
    width: 14px;
    flex-shrink: 0;
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
  .tree-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tree-count {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-hover);
    padding: 0 5px;
    border-radius: 8px;
    flex-shrink: 0;
  }

  /* File list */
  .file-list {
    flex: 1;
    min-width: 0;
  }
  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: rgba(0,0,0,0.01);
  }
  .list-path {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .list-count {
    font-size: 12px;
    color: var(--text-hint);
  }

  .empty-library {
    padding: 3rem 1rem;
    text-align: center;
    color: var(--text-secondary);
  }
  .empty-library p { margin: 0.5rem 0; }

  .file-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    text-decoration: none;
    color: inherit;
    transition: background 0.1s;
    cursor: grab;
  }
  .file-item:hover {
    background: var(--bg-hover);
    text-decoration: none;
  }
  .file-item:active {
    cursor: grabbing;
  }
  .file-icon {
    flex-shrink: 0;
    color: var(--text-hint);
  }
  .file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .file-title {
    font-family: var(--font-serif);
    font-size: 14px;
    color: var(--text-primary);
    line-height: 1.35;
  }
  .file-item:hover .file-title {
    color: var(--accent);
  }
  .file-desc {
    font-size: 12px;
    color: var(--text-hint);
    margin-top: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .file-folder {
    font-size: 11px;
    color: var(--text-hint);
  }
  .file-remove {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    padding: 2px;
    display: flex;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .file-item:hover .file-remove {
    opacity: 1;
  }
  .file-remove:hover {
    color: #dc2626;
  }

  @media (max-width: 640px) {
    .folder-tree {
      width: 180px;
    }
  }
</style>
