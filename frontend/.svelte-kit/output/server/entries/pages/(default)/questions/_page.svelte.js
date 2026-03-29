import { c as escape_html, e as ensure_array_like, i as attr, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t as tagName, a as authorName } from "../../../../chunks/display.js";
import { t } from "../../../../chunks/index.svelte.js";
import { b as buildArticleRowMap } from "../../../../chunks/series.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const questionsQuery = createQuery({});
    const teachesQuery = createQuery({});
    let questions = derived(() => store_get($$store_subs ??= {}, "$questionsQuery", questionsQuery).data ?? []);
    let articleTeaches = derived(() => buildArticleRowMap(store_get($$store_subs ??= {}, "$teachesQuery", teachesQuery).data ?? []));
    let loading = derived(() => store_get($$store_subs ??= {}, "$questionsQuery", questionsQuery).isPending || store_get($$store_subs ??= {}, "$teachesQuery", teachesQuery).isPending);
    $$renderer2.push(`<div class="questions-header svelte-1qpbf0h"><h1 class="svelte-1qpbf0h">${escape_html(t("qa.questions"))}</h1> `);
    if (getAuth()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<a href="/new-question" class="btn-ask svelte-1qpbf0h">${escape_html(t("qa.askQuestion"))}</a>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("common.loading"))}</p>`);
    } else if (questions().length === 0) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="empty svelte-1qpbf0h">${escape_html(t("qa.noQuestions"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<!--[-->`);
      const each_array = ensure_array_like(questions());
      for (let $$index_1 = 0, $$length = each_array.length; $$index_1 < $$length; $$index_1++) {
        let q = each_array[$$index_1];
        $$renderer2.push(`<a${attr("href", `/question?uri=${stringify(encodeURIComponent(q.at_uri))}`)} class="question-card svelte-1qpbf0h"><div class="q-top svelte-1qpbf0h"><span class="q-badge svelte-1qpbf0h">${escape_html(t("qa.questionBadge"))}</span> <span class="q-title svelte-1qpbf0h">${escape_html(q.title)}</span></div> `);
        if (articleTeaches().get(q.at_uri)?.length) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<div class="q-tags svelte-1qpbf0h"><!--[-->`);
          const each_array_1 = ensure_array_like(articleTeaches().get(q.at_uri) || []);
          for (let $$index = 0, $$length2 = each_array_1.length; $$index < $$length2; $$index++) {
            let tag = each_array_1[$$index];
            $$renderer2.push(`<span class="tag" role="link" tabindex="0">${escape_html(tagName(tag.tag_names, tag.tag_name, tag.tag_id))}</span>`);
          }
          $$renderer2.push(`<!--]--></div>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> `);
        if (q.description) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<p class="q-desc svelte-1qpbf0h">${escape_html(q.description)}</p>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> <div class="q-bottom svelte-1qpbf0h"><span class="q-meta svelte-1qpbf0h">${escape_html(authorName(q))} · ${escape_html(q.created_at.split(" ")[0])}</span> <span class="q-stats svelte-1qpbf0h"><span class="stat svelte-1qpbf0h">${escape_html(t("qa.answerCount", q.answer_count))}</span> `);
        if (q.vote_score !== 0) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="stat svelte-1qpbf0h">▲ ${escape_html(q.vote_score)}</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></span></div></a>`);
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
