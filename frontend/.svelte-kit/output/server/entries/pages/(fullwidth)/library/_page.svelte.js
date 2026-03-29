import { c as escape_html, i as attr, j as derived, a as attr_class, l as attr_style, e as ensure_array_like, b as stringify } from "../../../../chunks/index2.js";
import "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let bookmarks = [];
    let myArticles = [];
    let allSeries = [];
    let seriesArticleMap = /* @__PURE__ */ new Map();
    let expandedFolders = /* @__PURE__ */ new Set(["/", `/${t("library.seriesFolder")}`]);
    let selectedFolder = "/";
    let folderTree = derived(() => {
      const folders = /* @__PURE__ */ new Set(["/"]);
      for (
        const b of
        // Add this folder and all parent folders
        // Hierarchical folder structure for tree rendering
        // Count bookmarks per folder
        // Articles in selected folder (and subfolders)
        // Filter to user's series
        // Build a set of article URIs that belong to any of the user's series
        // "My articles" as virtual bookmark items — series articles go into series folders
        // Articles that belong to a series → placed in /${t('library.seriesFolder')}/SeriesTitle folder
        // Standalone articles (not in any series)
        allItems()
      ) {
        const parts = b.folder_path.split("/").filter(Boolean);
        let path = "";
        for (const p of parts) {
          path += "/" + p;
          folders.add(path);
        }
      }
      return Array.from(folders).sort();
    });
    let folderNodes = derived(() => {
      const root = { name: t("nav.library"), path: "/", children: [], count: 0 };
      const nodeMap = /* @__PURE__ */ new Map();
      nodeMap.set("/", root);
      const folderCounts = /* @__PURE__ */ new Map();
      for (
        const b of
        // Articles in selected folder (and subfolders)
        // Filter to user's series
        // Build a set of article URIs that belong to any of the user's series
        // "My articles" as virtual bookmark items — series articles go into series folders
        // Articles that belong to a series → placed in /${t('library.seriesFolder')}/SeriesTitle folder
        // Standalone articles (not in any series)
        allItems()
      ) {
        folderCounts.set(b.folder_path, (folderCounts.get(b.folder_path) || 0) + 1);
      }
      root.count = // Articles in selected folder (and subfolders)
      // Filter to user's series
      // Build a set of article URIs that belong to any of the user's series
      // "My articles" as virtual bookmark items — series articles go into series folders
      // Articles that belong to a series → placed in /${t('library.seriesFolder')}/SeriesTitle folder
      // Standalone articles (not in any series)
      allItems().length;
      for (const path of folderTree()) {
        if (path === "/") continue;
        const parts = path.split("/").filter(Boolean);
        const name = parts[parts.length - 1];
        const parentPath = parts.length > 1 ? "/" + parts.slice(0, -1).join("/") : "/";
        const node = { name, path, children: [], count: folderCounts.get(path) || 0 };
        nodeMap.set(path, node);
        const parent = nodeMap.get(parentPath);
        if (parent) parent.children.push(node);
      }
      return root;
    });
    let visibleArticles = derived(() => {
      return;
    });
    let seriesArticleUris = derived(() => {
      const uris = /* @__PURE__ */ new Set();
      for (const s of allSeries) {
        const arts = seriesArticleMap.get(s.id) || [];
        for (const uri of arts) uris.add(uri);
      }
      return uris;
    });
    let myArticleItems = derived(() => {
      const items = [];
      for (const s of allSeries) {
        const articleUris = seriesArticleMap.get(s.id) || [];
        for (const uri of articleUris) {
          const art = myArticles.find((a) => a.at_uri === uri);
          if (art) {
            items.push({
              article_uri: art.at_uri,
              folder_path: `/${t("library.seriesFolder")}/${s.title}`,
              created_at: art.created_at,
              title: art.title,
              description: art.description
            });
          }
        }
      }
      for (const a of myArticles) {
        if (!seriesArticleUris().has(a.at_uri)) {
          items.push({
            article_uri: a.at_uri,
            folder_path: `/${t("profile.works")}`,
            created_at: a.created_at,
            title: a.title,
            description: a.description
          });
        }
      }
      return items;
    });
    let allItems = derived(() => [...bookmarks, ...myArticleItems()]);
    function folderItem($$renderer3, node, depth) {
      $$renderer3.push(`<div${attr_class("tree-item svelte-58azz5", void 0, { "selected": selectedFolder === node.path })}${attr_style(`padding-left: ${stringify(8 + depth * 16)}px`)} role="treeitem" tabindex="0"${attr("aria-selected", selectedFolder === node.path)}>`);
      if (node.children.length > 0) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<button${attr_class("tree-chevron svelte-58azz5", void 0, { "open": expandedFolders.has(node.path) })}${attr("title", t("article.toggleCollapse"))}><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"></polyline></svg></button>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        $$renderer3.push(`<span class="tree-spacer svelte-58azz5"></span>`);
      }
      $$renderer3.push(`<!--]--> <svg class="tree-icon svelte-58azz5" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">`);
      if (node.path === "/") {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        $$renderer3.push(`<path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"></path>`);
      }
      $$renderer3.push(`<!--]--></svg> <span class="tree-name svelte-58azz5">${escape_html(node.name)}</span> `);
      if (node.count > 0) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<span class="tree-count svelte-58azz5">${escape_html(node.count)}</span>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></div> `);
      if (expandedFolders.has(node.path) && node.children.length > 0) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<!--[-->`);
        const each_array = ensure_array_like(node.children);
        for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
          let child = each_array[$$index];
          folderItem($$renderer3, child, depth + 1);
        }
        $$renderer3.push(`<!--]-->`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]-->`);
    }
    $$renderer2.push(`<div class="library-layout svelte-58azz5"><aside class="folder-tree svelte-58azz5"><div class="tree-header svelte-58azz5"><span class="tree-title svelte-58azz5">${escape_html(t("nav.library"))}</span> <button class="tree-action svelte-58azz5"${attr("title", t("library.newFolder"))}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg></button></div> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <nav class="tree-nav svelte-58azz5">`);
    folderItem($$renderer2, folderNodes(), 0);
    $$renderer2.push(`<!----></nav></aside> <main class="file-list svelte-58azz5"><div class="list-header svelte-58azz5"><span class="list-path svelte-58azz5">${escape_html(t("nav.library"))}</span> <span class="list-count svelte-58azz5">${escape_html(visibleArticles().length)}</span></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]--></main></div>`);
  });
}
export {
  _page as default
};
