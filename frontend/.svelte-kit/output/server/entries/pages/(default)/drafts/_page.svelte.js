import { c as escape_html, e as ensure_array_like, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "../../../../chunks/auth.svelte.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const draftsQuery = createQuery({});
    let drafts = derived(() => store_get($$store_subs ??= {}, "$draftsQuery", draftsQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$draftsQuery", draftsQuery).isPending);
    function formatDate(s) {
      return s.replace("T", " ").slice(0, 16);
    }
    $$renderer2.push(`<h1>${escape_html(t("drafts.title"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("drafts.loading"))}</p>`);
    } else if (drafts().length === 0) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("drafts.empty"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="draft-list svelte-1d0xg5i"><!--[-->`);
      const each_array = ensure_array_like(drafts());
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let draft = each_array[$$index];
        $$renderer2.push(`<div class="draft-card svelte-1d0xg5i"><div class="draft-header svelte-1d0xg5i"><button class="draft-title svelte-1d0xg5i">${escape_html(draft.title || t("drafts.untitled"))}</button> <span class="draft-format svelte-1d0xg5i">${escape_html(draft.content_format === "markdown" ? "MD" : draft.content_format === "html" ? "HTML" : "Typst")}</span></div> `);
        if (draft.description) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<p class="draft-desc svelte-1d0xg5i">${escape_html(draft.description)}</p>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> <div class="draft-meta svelte-1d0xg5i"><span>${escape_html(formatDate(draft.updated_at))}</span> <span>${escape_html(draft.lang)}</span></div> <div class="draft-actions svelte-1d0xg5i"><button class="btn-edit svelte-1d0xg5i">${escape_html(t("drafts.edit"))}</button> <button class="btn-publish svelte-1d0xg5i">${escape_html(t("drafts.publish"))}</button> <button class="btn-delete svelte-1d0xg5i">${escape_html(t("drafts.delete"))}</button></div></div>`);
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
