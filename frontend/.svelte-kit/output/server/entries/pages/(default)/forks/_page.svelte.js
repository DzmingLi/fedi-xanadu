import { c as escape_html, i as attr, b as stringify, e as ensure_array_like, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { p as page } from "../../../../chunks/stores.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let uri = derived(() => store_get($$store_subs ??= {}, "$page", page).url.searchParams.get("uri") ?? "");
    let isLoggedIn = derived(() => !!getAuth());
    const articleQuery = createQuery({});
    const forksQuery = createQuery({});
    let article = derived(() => store_get($$store_subs ??= {}, "$articleQuery", articleQuery).data ?? null);
    let forks = derived(() => store_get($$store_subs ??= {}, "$forksQuery", forksQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$articleQuery", articleQuery).isPending || store_get($$store_subs ??= {}, "$forksQuery", forksQuery).isPending);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (article()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<h1 class="svelte-13rakcd">Forks of "${escape_html(article().title)}"</h1> <p class="meta"><a${attr("href", `/article?uri=${stringify(encodeURIComponent(uri()))}`)}>${escape_html(t("forks.backToOriginal"))}</a> · ${escape_html(forks().length)} forks</p> `);
      if (forks().length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="empty svelte-13rakcd"><p>${escape_html(t("forks.createHint"))}</p></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<div class="fork-list svelte-13rakcd"><!--[-->`);
        const each_array = ensure_array_like(forks());
        for (let i = 0, $$length = each_array.length; i < $$length; i++) {
          let f = each_array[i];
          $$renderer2.push(`<div class="fork-card svelte-13rakcd"><div class="fork-rank svelte-13rakcd">#${escape_html(i + 1)}</div> <div class="fork-body svelte-13rakcd"><a${attr("href", `/article?uri=${stringify(encodeURIComponent(f.forked_uri))}`)} class="fork-title svelte-13rakcd">${escape_html(f.title)}</a> <div class="fork-info svelte-13rakcd"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(f.did))}`)} class="fork-author svelte-13rakcd">${escape_html(f.author_handle ? `@${f.author_handle}` : f.did.slice(0, 24) + "...")}</a></div></div> <div class="fork-votes svelte-13rakcd"><button class="vote-btn svelte-13rakcd"${attr("title", t("common.upvote"))}${attr("disabled", !isLoggedIn(), true)}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="18 15 12 9 6 15"></polyline></svg></button> <span class="score svelte-13rakcd">${escape_html(f.vote_score)}</span> <button class="vote-btn svelte-13rakcd"${attr("title", t("common.downvote"))}${attr("disabled", !isLoggedIn(), true)}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"></polyline></svg></button></div></div>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      }
      $$renderer2.push(`<!--]-->`);
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
