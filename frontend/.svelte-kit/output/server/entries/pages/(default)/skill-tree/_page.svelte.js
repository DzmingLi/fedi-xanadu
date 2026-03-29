import { c as escape_html, i as attr, b as stringify, e as ensure_array_like, j as derived, l as attr_style, a as attr_class, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t as tagName } from "../../../../chunks/display.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let skillTreeQuery = createQuery({});
    createQuery({});
    let detail = derived(() => store_get($$store_subs ??= {}, "$skillTreeQuery", skillTreeQuery).data ?? null);
    let loading = derived(() => store_get($$store_subs ??= {}, "$skillTreeQuery", skillTreeQuery).isPending);
    let isLoggedIn = derived(() => !!getAuth());
    let isOwner = derived(() => isLoggedIn() && detail()?.tree.did === getAuth()?.did);
    let collapsed = /* @__PURE__ */ new Set();
    let newParent = "";
    let newChild = "";
    let treeStructure = derived(() => {
      if (!detail()) return { roots: [], children: /* @__PURE__ */ new Map() };
      const children = /* @__PURE__ */ new Map();
      const hasParent = /* @__PURE__ */ new Set();
      const allNodes = /* @__PURE__ */ new Set();
      for (const e of detail().edges) {
        const arr = children.get(e.parent_tag) || [];
        arr.push(e.child_tag);
        children.set(e.parent_tag, arr);
        hasParent.add(e.child_tag);
        allNodes.add(e.parent_tag);
        allNodes.add(e.child_tag);
      }
      const roots = [...allNodes].filter((n) => !hasParent.has(n)).sort();
      return { roots, children };
    });
    function tagName$1(id) {
      const i18nNames = detail()?.tag_names_i18n?.[id];
      const fallbackName = detail()?.tag_names_map[id] || id;
      return tagName(i18nNames, fallbackName, id);
    }
    function treeNode($$renderer3, id, depth) {
      $$renderer3.push(`<div class="tree-item svelte-mqeqei"${attr_style(`padding-left: ${stringify(depth * 24)}px`)}>`);
      if (treeStructure().children.has(id)) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<button${attr_class("collapse-btn svelte-mqeqei", void 0, { "collapsed": collapsed.has(id) })}${attr("title", t("skillTree.collapse"))}><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"></polyline></svg></button>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        $$renderer3.push(`<span class="collapse-spacer svelte-mqeqei"></span>`);
      }
      $$renderer3.push(`<!--]--> <a${attr("href", `/tag?id=${stringify(encodeURIComponent(id))}`)} class="node-link svelte-mqeqei">${escape_html(tagName$1(id))}</a> `);
      if (treeStructure().children.has(id)) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<span class="child-count svelte-mqeqei">${escape_html(treeStructure().children.get(id).length)}</span>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></div> `);
      if (treeStructure().children.has(id) && !collapsed.has(id)) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<!--[-->`);
        const each_array = ensure_array_like(treeStructure().children.get(id));
        for (let $$index_4 = 0, $$length = each_array.length; $$index_4 < $$length; $$index_4++) {
          let child = each_array[$$index_4];
          treeNode($$renderer3, child, depth + 1);
        }
        $$renderer3.push(`<!--]-->`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]-->`);
    }
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (detail()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<div class="tree-header svelte-mqeqei"><div class="tree-title-row svelte-mqeqei"><h1 class="svelte-mqeqei">${escape_html(detail().tree.title)}</h1> `);
      if (detail().tree.tag_id) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<span class="field-badge svelte-mqeqei">${escape_html(tagName$1(detail().tree.tag_id))}</span>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div> <div class="tree-actions svelte-mqeqei">`);
      if (isLoggedIn()) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<button class="btn svelte-mqeqei">${escape_html(t("skillTree.adopt"))}</button> <button class="btn svelte-mqeqei">Fork</button> <button class="btn svelte-mqeqei">👍</button>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div></div> `);
      if (detail().tree.description) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="desc svelte-mqeqei">${escape_html(detail().tree.description)}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (detail().tree.forked_from) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="forked-info svelte-mqeqei">Forked from <a${attr("href", `/skill-tree?uri=${stringify(encodeURIComponent(detail().tree.forked_from))}`)} class="svelte-mqeqei">${escape_html(detail().tree.forked_from.slice(0, 40))}...</a></p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="tree-visual svelte-mqeqei"><!--[-->`);
      const each_array_1 = ensure_array_like(treeStructure().roots);
      for (let $$index = 0, $$length = each_array_1.length; $$index < $$length; $$index++) {
        let root = each_array_1[$$index];
        treeNode($$renderer2, root, 0);
      }
      $$renderer2.push(`<!--]--></div> `);
      if (isOwner()) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="editor-section svelte-mqeqei"><h3 class="svelte-mqeqei">${escape_html(t("skillTree.editRelations"))}</h3> <div class="edge-form svelte-mqeqei"><div class="input-wrap svelte-mqeqei"><input type="text"${attr("value", newParent)}${attr("placeholder", t("skillTree.parentTag"))} class="svelte-mqeqei"/> `);
        {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div> <span class="arrow svelte-mqeqei">→</span> <div class="input-wrap svelte-mqeqei"><input type="text"${attr("value", newChild)}${attr("placeholder", t("skillTree.childTag"))} class="svelte-mqeqei"/> `);
        {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div> <button class="add-btn svelte-mqeqei">${escape_html(t("common.add"))}</button></div> <p class="hint svelte-mqeqei">${escape_html(t("skillTree.autoCreateHint"))}</p></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="edge-list svelte-mqeqei"><h3 class="svelte-mqeqei">${escape_html(t("skillTree.allRelations", detail().edges.length))}</h3> <!--[-->`);
      const each_array_4 = ensure_array_like(detail().edges);
      for (let $$index_3 = 0, $$length = each_array_4.length; $$index_3 < $$length; $$index_3++) {
        let e = each_array_4[$$index_3];
        $$renderer2.push(`<div class="edge-row svelte-mqeqei"><a${attr("href", `/tag?id=${stringify(encodeURIComponent(e.parent_tag))}`)} class="tag">${escape_html(tagName$1(e.parent_tag))}</a> <span class="arrow svelte-mqeqei">→</span> <a${attr("href", `/tag?id=${stringify(encodeURIComponent(e.child_tag))}`)} class="tag">${escape_html(tagName$1(e.child_tag))}</a> `);
        if (isOwner()) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<button class="remove-btn svelte-mqeqei">×</button>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
