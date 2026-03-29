import { c as escape_html, i as attr, e as ensure_array_like, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { p as page } from "../../../../chunks/stores.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let bookId = derived(() => store_get($$store_subs ??= {}, "$page", page).url.searchParams.get("bookId") ?? "");
    let title = "";
    let lang = "zh";
    let isbn = "";
    let publisher = "";
    let year = "";
    let coverUrl = "";
    let translatorsText = "";
    let purchaseLinks = [];
    let newLinkLabel = "";
    let newLinkUrl = "";
    let submitting = false;
    $$renderer2.push(`<h1>${escape_html(t("bookEdition.title"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form svelte-1ki0ig"><label class="svelte-1ki0ig">${escape_html(t("bookEdition.editionTitle"))} <input type="text"${attr("value", title)}${attr("placeholder", t("bookEdition.editionTitlePlaceholder"))} class="svelte-1ki0ig"/></label> <label class="svelte-1ki0ig">${escape_html(t("bookEdition.lang"))} `);
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
        $$renderer3.option({ value: "ja" }, ($$renderer4) => {
          $$renderer4.push(`日本語`);
        });
        $$renderer3.option({ value: "de" }, ($$renderer4) => {
          $$renderer4.push(`Deutsch`);
        });
        $$renderer3.option({ value: "es" }, ($$renderer4) => {
          $$renderer4.push(`Español`);
        });
      },
      "svelte-1ki0ig"
    );
    $$renderer2.push(`</label> <label class="svelte-1ki0ig">ISBN <input type="text"${attr("value", isbn)} placeholder="978-..." class="svelte-1ki0ig"/></label> <label class="svelte-1ki0ig">${escape_html(t("bookEdition.publisher"))} <input type="text"${attr("value", publisher)} class="svelte-1ki0ig"/></label> <label class="svelte-1ki0ig">${escape_html(t("bookEdition.year"))} <input type="text"${attr("value", year)} placeholder="2024" class="svelte-1ki0ig"/></label> <label class="svelte-1ki0ig">${escape_html(t("bookEdition.translators"))} <input type="text"${attr("value", translatorsText)}${attr("placeholder", t("bookEdition.translatorsPlaceholder"))} class="svelte-1ki0ig"/></label> <label class="svelte-1ki0ig">${escape_html(t("bookEdition.coverUrl"))} <input type="text"${attr("value", coverUrl)} placeholder="https://..." class="svelte-1ki0ig"/></label> `);
    if (coverUrl.trim()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="cover-preview svelte-1ki0ig"><img${attr("src", coverUrl)} alt="cover preview" class="svelte-1ki0ig"/></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="purchase-section svelte-1ki0ig"><h3 class="svelte-1ki0ig">${escape_html(t("bookEdition.purchaseLinks"))}</h3> <!--[-->`);
    const each_array = ensure_array_like(purchaseLinks);
    for (let i = 0, $$length = each_array.length; i < $$length; i++) {
      let link = each_array[i];
      $$renderer2.push(`<div class="link-row svelte-1ki0ig"><span class="link-label svelte-1ki0ig">${escape_html(link.label)}</span> <a${attr("href", link.url)} target="_blank" rel="noopener" class="link-url svelte-1ki0ig">${escape_html(link.url)}</a> <button class="remove-btn svelte-1ki0ig">×</button></div>`);
    }
    $$renderer2.push(`<!--]--> <div class="add-link-row svelte-1ki0ig"><input type="text"${attr("value", newLinkLabel)}${attr("placeholder", t("bookEdition.linkLabel"))} class="svelte-1ki0ig"/> <input type="text"${attr("value", newLinkUrl)}${attr("placeholder", t("bookEdition.linkUrl"))} class="svelte-1ki0ig"/> <button class="add-link-btn svelte-1ki0ig">${escape_html(t("common.add"))}</button></div></div> <div class="form-actions svelte-1ki0ig"><button class="submit-btn svelte-1ki0ig"${attr("disabled", submitting, true)}>${escape_html(t("bookEdition.submit"))}</button> <a${attr("href", `/book?id=${stringify(encodeURIComponent(bookId()))}`)} class="cancel-link svelte-1ki0ig">${escape_html(t("books.cancel"))}</a></div></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
