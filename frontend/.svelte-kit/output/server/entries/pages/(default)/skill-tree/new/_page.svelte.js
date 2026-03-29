import { c as escape_html, i as attr, e as ensure_array_like, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../../chunks/exports.js";
import "../../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../../chunks/root.js";
import "../../../../../chunks/state.svelte.js";
import "../../../../../chunks/auth.svelte.js";
import { t } from "../../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let title = "";
    let description = "";
    let fieldQuery = "";
    let edges = [];
    let creating = false;
    let newParent = "";
    let newChild = "";
    const tagsQuery = createQuery({});
    let allTags = derived(() => store_get($$store_subs ??= {}, "$tagsQuery", tagsQuery).data ?? []);
    function tagDisplay(id) {
      const t2 = allTags().find((t3) => t3.id === id);
      return t2 ? t2.name : id;
    }
    $$renderer2.push(`<h1>${escape_html(t("newSkillTree.title"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form svelte-2kl3w5"><label class="svelte-2kl3w5">${escape_html(t("newSkillTree.titleLabel"))} <input type="text"${attr("value", title)}${attr("placeholder", t("newSkillTree.titlePlaceholder"))} class="svelte-2kl3w5"/></label> <label class="svelte-2kl3w5">${escape_html(t("newArticle.descLabel"))} <textarea rows="2"${attr("placeholder", t("newSkillTree.descPlaceholder"))} class="svelte-2kl3w5">`);
    const $$body = escape_html(description);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea></label> <label class="svelte-2kl3w5">${escape_html(t("newSkillTree.tagLabel"))} <div class="field-input-wrap svelte-2kl3w5"><input type="text"${attr("value", fieldQuery)}${attr("placeholder", t("newSkillTree.tagPlaceholder"))} class="svelte-2kl3w5"/> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div></label> <h2 class="svelte-2kl3w5">${escape_html(t("newSkillTree.addRelation"))}</h2> <p class="hint svelte-2kl3w5">${escape_html(t("newSkillTree.relationHint"))}</p> <div class="edge-form svelte-2kl3w5"><div class="input-wrap svelte-2kl3w5"><input type="text"${attr("value", newParent)}${attr("placeholder", t("newSkillTree.parentPlaceholder"))} class="svelte-2kl3w5"/> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> <span class="arrow svelte-2kl3w5">-></span> <div class="input-wrap svelte-2kl3w5"><input type="text"${attr("value", newChild)}${attr("placeholder", t("newSkillTree.childPlaceholder"))} class="svelte-2kl3w5"/> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> <button class="add-btn svelte-2kl3w5">${escape_html(t("common.add"))}</button></div> `);
    if (edges.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="edge-list svelte-2kl3w5"><!--[-->`);
      const each_array_3 = ensure_array_like(edges);
      for (let i = 0, $$length = each_array_3.length; i < $$length; i++) {
        let e = each_array_3[i];
        $$renderer2.push(`<div class="edge-row svelte-2kl3w5"><span class="tag">${escape_html(tagDisplay(e.parent_tag))}</span> <span class="arrow svelte-2kl3w5">-></span> <span class="tag">${escape_html(tagDisplay(e.child_tag))}</span> <button class="remove-btn svelte-2kl3w5">x</button></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <button class="submit-btn svelte-2kl3w5"${attr("disabled", creating, true)}>${escape_html(t("newSkillTree.create"))}</button></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
