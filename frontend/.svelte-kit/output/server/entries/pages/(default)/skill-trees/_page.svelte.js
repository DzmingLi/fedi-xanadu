import { c as escape_html, a as attr_class, e as ensure_array_like, i as attr, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { t as tagName } from "../../../../chunks/display.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let filterField = "";
    let isLoggedIn = derived(() => !!getAuth());
    const treesQuery = createQuery({});
    let trees = derived(() => store_get($$store_subs ??= {}, "$treesQuery", treesQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$treesQuery", treesQuery).isPending);
    let filteredTrees = derived(() => trees());
    let availableFields = derived(() => {
      const fieldMap = /* @__PURE__ */ new Map();
      for (const tr of trees()) {
        if (!tr.tag_id) continue;
        const existing = fieldMap.get(tr.tag_id);
        if (existing) {
          existing.count++;
        } else {
          const name = tr.tag_names ? tagName(tr.tag_names, tr.tag_name || tr.tag_id, tr.tag_id) : tr.tag_name || tr.tag_id;
          fieldMap.set(tr.tag_id, { id: tr.tag_id, name, count: 1 });
        }
      }
      return [...fieldMap.values()].sort((a, b) => b.count - a.count);
    });
    $$renderer2.push(`<div class="header svelte-mqdtul"><h1 class="svelte-mqdtul">${escape_html(t("skills.communityTrees"))}</h1> `);
    if (isLoggedIn()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<a href="/skill-tree/new" class="create-btn svelte-mqdtul">${escape_html(t("skills.createTree"))}</a>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> <p class="subtitle svelte-mqdtul">${escape_html(t("skills.browseHint"))}</p> <div class="field-filter svelte-mqdtul"><button${attr_class("filter-btn svelte-mqdtul", void 0, { "active": !filterField })}>${escape_html(t("home.all"))}</button> <!--[-->`);
    const each_array = ensure_array_like(availableFields());
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let f = each_array[$$index];
      $$renderer2.push(`<button${attr_class("filter-btn svelte-mqdtul", void 0, { "active": filterField === f.id })}>${escape_html(f.name)} (${escape_html(f.count)})</button>`);
    }
    $$renderer2.push(`<!--]--></div> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("common.loading"))}</p>`);
    } else if (trees().length === 0) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<div class="empty svelte-mqdtul"><p>${escape_html(t("skills.noTrees"))}</p> `);
      if (isLoggedIn()) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<a href="/skill-tree/new">${escape_html(t("skills.createFirst"))}</a>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="tree-list svelte-mqdtul"><!--[-->`);
      const each_array_1 = ensure_array_like(filteredTrees());
      for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
        let tree = each_array_1[$$index_1];
        $$renderer2.push(`<div class="tree-card svelte-mqdtul"><div class="tree-main svelte-mqdtul"><a${attr("href", `/skill-tree?uri=${stringify(encodeURIComponent(tree.at_uri))}`)} class="tree-title svelte-mqdtul">${escape_html(tree.title)}</a> `);
        if (tree.tag_id) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="field-badge svelte-mqdtul">${escape_html(tree.tag_names ? tagName(tree.tag_names, tree.tag_name || tree.tag_id, tree.tag_id) : tree.tag_name || tree.tag_id)}</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> `);
        if (tree.forked_from) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="forked-badge svelte-mqdtul">Fork</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> `);
        if (tree.description) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<p class="tree-desc svelte-mqdtul">${escape_html(tree.description)}</p>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> <div class="tree-meta svelte-mqdtul"><span>${escape_html(tree.author_handle ? `@${tree.author_handle}` : tree.did.slice(0, 20))}</span> <span>${escape_html(tree.edge_count)} ${escape_html(t("skills.edgeCount"))}</span> <span>${escape_html(tree.adopt_count)} ${escape_html(t("skills.adoptCount"))}</span></div></div> <div class="tree-actions svelte-mqdtul"><div class="vote-col svelte-mqdtul"><button class="vote-btn svelte-mqdtul"${attr("disabled", !isLoggedIn(), true)}>▲</button> <span class="score svelte-mqdtul">${escape_html(tree.score ?? 0)}</span> <button class="vote-btn svelte-mqdtul"${attr("disabled", !isLoggedIn(), true)}>▼</button></div> <button class="adopt-btn svelte-mqdtul"${attr("disabled", !isLoggedIn(), true)}>${escape_html(t("skills.adopt"))}</button></div></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
