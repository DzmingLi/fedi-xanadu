import { c as escape_html, i as attr, b as stringify, e as ensure_array_like, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let seriesQuery = createQuery({});
    let detail = derived(() => store_get($$store_subs ??= {}, "$seriesQuery", seriesQuery).data ?? null);
    let loading = derived(() => store_get($$store_subs ??= {}, "$seriesQuery", seriesQuery).isPending);
    let error = derived(() => store_get($$store_subs ??= {}, "$seriesQuery", seriesQuery).error?.message ?? "");
    let tree = null;
    let articleVotes = /* @__PURE__ */ new Map();
    let bookmarkedUris = /* @__PURE__ */ new Set();
    let isLoggedIn = derived(() => !!getAuth());
    let hasChildren = derived(() => detail() ? detail().children.length > 0 : false);
    let prereqMap = derived(() => {
      if (!detail()) return /* @__PURE__ */ new Map();
      const m = /* @__PURE__ */ new Map();
      for (const p of detail().prereqs) {
        const arr = m.get(p.article_uri) || [];
        arr.push(p.prereq_article_uri);
        m.set(p.article_uri, arr);
      }
      return m;
    });
    let articleByUri = derived(() => {
      if (!detail()) return /* @__PURE__ */ new Map();
      return new Map(detail().articles.map((a) => [a.article_uri, a]));
    });
    function articleItem($$renderer3, article, idx) {
      $$renderer3.push(`<div class="series-item svelte-1im8znh"><div class="item-number svelte-1im8znh">${escape_html(idx + 1)}</div> <div class="item-content svelte-1im8znh">`);
      if (prereqMap().has(article.article_uri)) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<div class="item-prereqs svelte-1im8znh">${escape_html(t("series.prereqLabel"))} <!--[-->`);
        const each_array = ensure_array_like(prereqMap().get(article.article_uri));
        for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
          let pUri = each_array[$$index];
          const pArticle = articleByUri().get(pUri);
          if (pArticle) {
            $$renderer3.push("<!--[0-->");
            $$renderer3.push(`<a${attr("href", `/article?uri=${stringify(encodeURIComponent(pUri))}`)} class="prereq-link svelte-1im8znh">${escape_html(pArticle.title)}</a>`);
          } else {
            $$renderer3.push("<!--[-1-->");
          }
          $$renderer3.push(`<!--]-->`);
        }
        $$renderer3.push(`<!--]--></div>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--> <a${attr("href", `/article?uri=${stringify(encodeURIComponent(article.article_uri))}`)} class="item-title svelte-1im8znh">${escape_html(article.title)}</a> `);
      if (article.description) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<p class="item-desc svelte-1im8znh">${escape_html(article.description)}</p>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--> <div class="item-actions svelte-1im8znh"><span class="vote-score svelte-1im8znh">${escape_html(articleVotes.get(article.article_uri)?.score ?? 0)}</span> <button class="action-btn svelte-1im8znh"${attr("disabled", !isLoggedIn(), true)}${attr("title", t("common.upvote"))}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-6 0v4H5a2 2 0 00-2 2v7a2 2 0 002 2h14l-5-16z"></path></svg></button> <button class="action-btn svelte-1im8znh"${attr("disabled", !isLoggedIn(), true)}${attr("title", t("article.bookmark"))}><svg width="14" height="14" viewBox="0 0 24 24"${attr("fill", bookmarkedUris.has(article.article_uri) ? "currentColor" : "none")} stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"></path></svg></button></div></div></div>`);
    }
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (error()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="error">${escape_html(error())}</p>`);
    } else if (detail()) {
      $$renderer2.push("<!--[2-->");
      $$renderer2.push(`<div class="series-header svelte-1im8znh"><h1 class="svelte-1im8znh">${escape_html(detail().series.title)}</h1> `);
      if (detail().series.long_description) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="series-long-desc svelte-1im8znh">${escape_html(detail().series.long_description)}</p>`);
      } else if (detail().series.description) {
        $$renderer2.push("<!--[1-->");
        $$renderer2.push(`<p class="series-desc svelte-1im8znh">${escape_html(detail().series.description)}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="series-meta svelte-1im8znh">`);
      if (hasChildren() && tree) ;
      else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<span class="meta">${escape_html(detail().articles.length)} ${escape_html(t("series.articles"))}</span>`);
      }
      $$renderer2.push(`<!--]--> <span class="meta"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(detail().series.created_by))}`)}>@${escape_html(detail().series.author_handle || detail().series.created_by)}</a></span></div> `);
      if (detail().translations && detail().translations.length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="series-translations svelte-1im8znh"><span class="lang-current svelte-1im8znh">${escape_html(detail().series.lang)}</span> <!--[-->`);
        const each_array_3 = ensure_array_like(detail().translations);
        for (let $$index_3 = 0, $$length = each_array_3.length; $$index_3 < $$length; $$index_3++) {
          let tr = each_array_3[$$index_3];
          $$renderer2.push(`<a${attr("href", `/series?id=${stringify(encodeURIComponent(tr.id))}`)} class="lang-link svelte-1im8znh">${escape_html(tr.lang)}</a>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (detail().series.parent_id) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="series-parent svelte-1im8znh"><a${attr("href", `/series?id=${stringify(encodeURIComponent(detail().series.parent_id))}`)} class="svelte-1im8znh">${escape_html(t("series.backToParent"))}</a></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div> `);
      {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<div class="series-articles svelte-1im8znh"><!--[-->`);
        const each_array_4 = ensure_array_like(detail().articles);
        for (let i = 0, $$length = each_array_4.length; i < $$length; i++) {
          let article = each_array_4[i];
          articleItem($$renderer2, article, i);
        }
        $$renderer2.push(`<!--]--></div> `);
        if (detail().children.length > 0) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<div class="children-list svelte-1im8znh"><h2 class="svelte-1im8znh">${escape_html(t("series.sections"))}</h2> <!--[-->`);
          const each_array_5 = ensure_array_like(detail().children);
          for (let $$index_5 = 0, $$length = each_array_5.length; $$index_5 < $$length; $$index_5++) {
            let child = each_array_5[$$index_5];
            $$renderer2.push(`<a${attr("href", `/series?id=${stringify(encodeURIComponent(child.id))}`)} class="child-link svelte-1im8znh"><span class="child-title svelte-1im8znh">${escape_html(child.title)}</span> `);
            if (child.description) {
              $$renderer2.push("<!--[0-->");
              $$renderer2.push(`<span class="child-desc svelte-1im8znh">${escape_html(child.description)}</span>`);
            } else {
              $$renderer2.push("<!--[-1-->");
            }
            $$renderer2.push(`<!--]--></a>`);
          }
          $$renderer2.push(`<!--]--></div>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]-->`);
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
