import { c as escape_html, i as attr, e as ensure_array_like } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
/* empty css                                                     */
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let title = "";
    let description = "";
    let content = "";
    let contentFormat = "markdown";
    let lang = "zh";
    let tags = [];
    let tagQuery = "";
    let tagResults = [];
    let publishing = false;
    if (!getAuth()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("article.loginToComment"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<h1 class="page-title svelte-8ctw9">${escape_html(t("qa.askQuestion"))}</h1> <div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.titleLabel"))}</label> <input${attr("value", title)} type="text" placeholder="你的问题是什么？" class="svelte-8ctw9"/></div> <div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.descLabel"))}</label> <input${attr("value", description)} type="text"${attr("placeholder", t("newArticle.descPlaceholder"))} class="svelte-8ctw9"/></div> <div class="form-row svelte-8ctw9"><div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.formatLabel"))}</label> `);
      $$renderer2.select(
        { value: contentFormat, class: "" },
        ($$renderer3) => {
          $$renderer3.option({ value: "markdown" }, ($$renderer4) => {
            $$renderer4.push(`Markdown`);
          });
          $$renderer3.option({ value: "typst" }, ($$renderer4) => {
            $$renderer4.push(`Typst`);
          });
          $$renderer3.option({ value: "html" }, ($$renderer4) => {
            $$renderer4.push(`HTML`);
          });
        },
        "svelte-8ctw9"
      );
      $$renderer2.push(`</div> <div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.langLabel"))}</label> `);
      $$renderer2.select(
        { value: lang, class: "" },
        ($$renderer3) => {
          $$renderer3.option({ value: "zh" }, ($$renderer4) => {
            $$renderer4.push(`中文`);
          });
          $$renderer3.option({ value: "en" }, ($$renderer4) => {
            $$renderer4.push(`English`);
          });
          $$renderer3.option({ value: "fr" }, ($$renderer4) => {
            $$renderer4.push(`Français`);
          });
        },
        "svelte-8ctw9"
      );
      $$renderer2.push(`</div></div> <div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.contentLabel"))}</label> <textarea rows="10" placeholder="详细描述你的问题..." class="svelte-8ctw9">`);
      const $$body = escape_html(content);
      if ($$body) {
        $$renderer2.push(`${$$body}`);
      }
      $$renderer2.push(`</textarea></div> <div class="form-group svelte-8ctw9"><label class="svelte-8ctw9">${escape_html(t("newArticle.tagsLabel"))}</label> <div class="tag-input-wrap svelte-8ctw9"><!--[-->`);
      const each_array = ensure_array_like(tags);
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let tag = each_array[$$index];
        $$renderer2.push(`<span class="tag-chip svelte-8ctw9">${escape_html(tag)} <button type="button" class="svelte-8ctw9">×</button></span>`);
      }
      $$renderer2.push(`<!--]--> <input${attr("value", tagQuery)}${attr("placeholder", t("newArticle.tagInput"))} class="tag-input svelte-8ctw9"/></div> `);
      if (tagResults.length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="tag-dropdown svelte-8ctw9"><!--[-->`);
        const each_array_1 = ensure_array_like(tagResults);
        for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
          let tag = each_array_1[$$index_1];
          $$renderer2.push(`<button type="button" class="tag-option svelte-8ctw9">${escape_html(tag.name)}</button>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div> <div class="form-actions svelte-8ctw9"><button class="btn-publish svelte-8ctw9"${attr("disabled", publishing, true)}>${escape_html(t("newArticle.publish"))}</button></div>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
