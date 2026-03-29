import { c as escape_html, a as attr_class, e as ensure_array_like, i as attr, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { p as page } from "../../../../chunks/stores.js";
import "../../../../chunks/auth.svelte.js";
import { t as tagName, a as authorName } from "../../../../chunks/display.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let id = derived(() => store_get($$store_subs ??= {}, "$page", page).url.searchParams.get("id") ?? "");
    let tagQuery = createQuery({});
    let articlesQuery = createQuery({});
    let skillsQuery = createQuery({});
    let tag = derived(() => store_get($$store_subs ??= {}, "$tagQuery", tagQuery).data ?? null);
    let articles = derived(() => store_get($$store_subs ??= {}, "$articlesQuery", articlesQuery).data ?? []);
    let skills = derived(() => store_get($$store_subs ??= {}, "$skillsQuery", skillsQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$tagQuery", tagQuery).isPending || store_get($$store_subs ??= {}, "$articlesQuery", articlesQuery).isPending);
    let voteMap = /* @__PURE__ */ new Map();
    let isLit = derived(() => skills().some((s) => s.tag_id === id()));
    let topArticles = derived(() => [...articles()].sort((a, b) => (voteMap.get(b.at_uri) ?? 0) - (voteMap.get(a.at_uri) ?? 0)).slice(0, 20));
    let trendingArticles = derived(() => [...articles()].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 20));
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (tag()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<div class="tag-header svelte-1ehu148"><div class="tag-title-row svelte-1ehu148"><h1 class="svelte-1ehu148">${escape_html(tagName(tag().names, tag().name, tag().id))}</h1> <button${attr_class("skill-btn svelte-1ehu148", void 0, { "lit": isLit() })}>${escape_html(isLit() ? t("tags.mastered") : t("tags.light"))}</button></div> `);
      if (tag().description) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="tag-desc svelte-1ehu148">${escape_html(tag().description)}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <p class="tag-meta svelte-1ehu148">${escape_html(articles().length)} ${escape_html(t("tags.articles"))}</p></div> `);
      if (articles().length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="meta">${escape_html(t("tags.empty"))}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<div class="columns svelte-1ehu148"><div class="column svelte-1ehu148"><h2 class="svelte-1ehu148">Top Articles</h2> <!--[-->`);
        const each_array = ensure_array_like(topArticles());
        for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
          let a = each_array[$$index];
          $$renderer2.push(`<a${attr("href", `/article?uri=${stringify(encodeURIComponent(a.at_uri))}`)} class="article-item svelte-1ehu148"><span class="article-score svelte-1ehu148">${escape_html(voteMap.get(a.at_uri) ?? 0)}</span> <div class="article-info svelte-1ehu148"><span class="article-title svelte-1ehu148">${escape_html(a.title)}</span> `);
          if (a.description) {
            $$renderer2.push("<!--[0-->");
            $$renderer2.push(`<span class="article-desc svelte-1ehu148">${escape_html(a.description)}</span>`);
          } else {
            $$renderer2.push("<!--[-1-->");
          }
          $$renderer2.push(`<!--]--> <span class="article-meta svelte-1ehu148">${escape_html(authorName(a))} · ${escape_html(a.created_at.split(" ")[0])}</span></div></a>`);
        }
        $$renderer2.push(`<!--]--></div> <div class="column svelte-1ehu148"><h2 class="svelte-1ehu148">Trending</h2> <!--[-->`);
        const each_array_1 = ensure_array_like(trendingArticles());
        for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
          let a = each_array_1[$$index_1];
          $$renderer2.push(`<a${attr("href", `/article?uri=${stringify(encodeURIComponent(a.at_uri))}`)} class="article-item svelte-1ehu148"><span class="article-score svelte-1ehu148">${escape_html(voteMap.get(a.at_uri) ?? 0)}</span> <div class="article-info svelte-1ehu148"><span class="article-title svelte-1ehu148">${escape_html(a.title)}</span> `);
          if (a.description) {
            $$renderer2.push("<!--[0-->");
            $$renderer2.push(`<span class="article-desc svelte-1ehu148">${escape_html(a.description)}</span>`);
          } else {
            $$renderer2.push("<!--[-1-->");
          }
          $$renderer2.push(`<!--]--> <span class="article-meta svelte-1ehu148">${escape_html(authorName(a))} · ${escape_html(a.created_at.split(" ")[0])}</span></div></a>`);
        }
        $$renderer2.push(`<!--]--></div></div>`);
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
