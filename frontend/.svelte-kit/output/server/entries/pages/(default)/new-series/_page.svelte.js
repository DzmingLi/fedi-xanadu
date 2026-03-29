import { c as escape_html, i as attr, e as ensure_array_like, j as derived } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let title = "";
    let description = "";
    let longDescription = "";
    let tagSearch = "";
    let parentSearch = "";
    let category = "general";
    let creating = false;
    let seriesArticles = [];
    let articleSearch = "";
    let seriesPrereqs = [];
    createQuery({});
    createQuery({});
    createQuery({});
    let filteredTags = derived(() => []);
    let filteredArticles = derived(() => []);
    let filteredSeries = derived(() => []);
    $$renderer2.push(`<h1>${escape_html(t("newSeries.title"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form svelte-kgcnfe"><label class="svelte-kgcnfe">${escape_html(t("newSeries.parentLabel"))} <div class="article-search svelte-kgcnfe"><input type="text"${attr("value", parentSearch)}${attr("placeholder", t("newSeries.parentPlaceholder"))} class="svelte-kgcnfe"/> `);
    if (filteredSeries().length > 0 && true) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="dropdown svelte-kgcnfe"><!--[-->`);
      const each_array = ensure_array_like(filteredSeries());
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let s = each_array[$$index];
        $$renderer2.push(`<button class="dropdown-item svelte-kgcnfe">${escape_html(s.title)}</button>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></label> <label class="svelte-kgcnfe">${escape_html(t("newSeries.titleLabel"))} <input type="text"${attr("value", title)} class="svelte-kgcnfe"/></label> <label class="svelte-kgcnfe">${escape_html(t("newArticle.descLabel"))} <textarea rows="2"${attr("placeholder", t("newSeries.descPlaceholder"))} class="svelte-kgcnfe">`);
    const $$body = escape_html(description);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea></label> <label class="svelte-kgcnfe">${escape_html(t("newSeries.longDescLabel"))} <textarea rows="5"${attr("placeholder", t("newSeries.longDescPlaceholder"))} class="svelte-kgcnfe">`);
    const $$body_1 = escape_html(longDescription);
    if ($$body_1) {
      $$renderer2.push(`${$$body_1}`);
    }
    $$renderer2.push(`</textarea></label> <label class="svelte-kgcnfe">${escape_html(t("newArticle.categoryLabel"))} `);
    $$renderer2.select({ value: category }, ($$renderer3) => {
      $$renderer3.option({ value: "general" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.general"))}`);
      });
      $$renderer3.option({ value: "lecture" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.lecture"))}`);
      });
      $$renderer3.option({ value: "paper" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.paper"))}`);
      });
    });
    $$renderer2.push(`</label> <label class="svelte-kgcnfe">${escape_html(t("newSeries.tagLabel"))} <input type="text"${attr("value", tagSearch)}${attr("placeholder", t("newSeries.tagSearch"))} class="svelte-kgcnfe"/> `);
    if (filteredTags().length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="dropdown svelte-kgcnfe"><!--[-->`);
      const each_array_1 = ensure_array_like(filteredTags());
      for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
        let t2 = each_array_1[$$index_1];
        $$renderer2.push(`<button class="dropdown-item svelte-kgcnfe">${escape_html(t2.name)}</button>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></label> <h2 class="svelte-kgcnfe">${escape_html(t("newSeries.articleList"))}</h2> <p class="hint svelte-kgcnfe">${escape_html(t("newSeries.articleHint"))}</p> <div class="article-search svelte-kgcnfe"><input type="text"${attr("value", articleSearch)}${attr("placeholder", t("newSeries.articleSearch"))} class="svelte-kgcnfe"/> `);
    if (filteredArticles().length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="dropdown svelte-kgcnfe"><!--[-->`);
      const each_array_2 = ensure_array_like(filteredArticles());
      for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
        let a = each_array_2[$$index_2];
        $$renderer2.push(`<button class="dropdown-item svelte-kgcnfe">${escape_html(a.title)}</button>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> `);
    if (seriesArticles.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="article-list svelte-kgcnfe"><!--[-->`);
      const each_array_3 = ensure_array_like(seriesArticles);
      for (let i = 0, $$length = each_array_3.length; i < $$length; i++) {
        let sa = each_array_3[i];
        $$renderer2.push(`<div class="article-row svelte-kgcnfe"><span class="row-num svelte-kgcnfe">${escape_html(i + 1)}</span> <span class="row-title svelte-kgcnfe">${escape_html(sa.title)}</span> <div class="row-actions svelte-kgcnfe"><button${attr("disabled", i === 0, true)}${attr("title", t("newSeries.moveUp"))} class="svelte-kgcnfe">↑</button> <button${attr("disabled", i === seriesArticles.length - 1, true)}${attr("title", t("newSeries.moveDown"))} class="svelte-kgcnfe">↓</button> <button${attr("title", t("common.remove"))} class="svelte-kgcnfe">×</button></div></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (seriesArticles.length > 1) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<h3 class="svelte-kgcnfe">${escape_html(t("newSeries.prereqTitle"))}</h3> <p class="hint svelte-kgcnfe">${escape_html(t("newSeries.prereqHint"))}</p> <div class="prereq-builder svelte-kgcnfe"><select id="prereq-article" class="svelte-kgcnfe"><!--[-->`);
      const each_array_4 = ensure_array_like(seriesArticles);
      for (let i = 0, $$length = each_array_4.length; i < $$length; i++) {
        let sa = each_array_4[i];
        $$renderer2.option({ value: i }, ($$renderer3) => {
          $$renderer3.push(`#${escape_html(i + 1)} ${escape_html(sa.title)}`);
        });
      }
      $$renderer2.push(`<!--]--></select> <span>${escape_html(t("newSeries.prereqNeedsReading"))}</span> <select id="prereq-dep" class="svelte-kgcnfe"><!--[-->`);
      const each_array_5 = ensure_array_like(seriesArticles);
      for (let i = 0, $$length = each_array_5.length; i < $$length; i++) {
        let sa = each_array_5[i];
        $$renderer2.option({ value: i }, ($$renderer3) => {
          $$renderer3.push(`#${escape_html(i + 1)} ${escape_html(sa.title)}`);
        });
      }
      $$renderer2.push(`<!--]--></select> <button class="svelte-kgcnfe">${escape_html(t("common.add"))}</button></div> `);
      if (seriesPrereqs.length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="prereq-list svelte-kgcnfe"><!--[-->`);
        const each_array_6 = ensure_array_like(seriesPrereqs);
        for (let i = 0, $$length = each_array_6.length; i < $$length; i++) {
          let [aIdx, pIdx] = each_array_6[i];
          $$renderer2.push(`<div class="prereq-row svelte-kgcnfe">#${escape_html(aIdx + 1)} ${escape_html(seriesArticles[aIdx].title)} → ${escape_html(t("series.prereqLabel"))} #${escape_html(pIdx + 1)} ${escape_html(seriesArticles[pIdx].title)} <button class="svelte-kgcnfe">×</button></div>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]-->`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form-actions svelte-kgcnfe"><button class="submit-btn svelte-kgcnfe"${attr("disabled", creating, true)}>${escape_html(t("newSeries.create"))}</button></div></div>`);
  });
}
export {
  _page as default
};
