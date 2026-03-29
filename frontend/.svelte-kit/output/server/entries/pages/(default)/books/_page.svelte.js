import { c as escape_html, e as ensure_array_like, i as attr, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const booksQuery = createQuery({});
    let books = derived(() => store_get($$store_subs ??= {}, "$booksQuery", booksQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$booksQuery", booksQuery).isPending);
    $$renderer2.push(`<h1 class="svelte-pg9a6y">${escape_html(t("books.title"))}</h1> <p class="subtitle svelte-pg9a6y">${escape_html(t("books.subtitle"))}</p> `);
    if (getAuth()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<a href="/new-book" class="add-book-btn svelte-pg9a6y">${escape_html(t("books.addBook"))}</a>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (books().length === 0) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="empty svelte-pg9a6y">${escape_html(t("books.empty"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="book-grid svelte-pg9a6y"><!--[-->`);
      const each_array = ensure_array_like(books());
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let book = each_array[$$index];
        $$renderer2.push(`<a${attr("href", `/book?id=${stringify(encodeURIComponent(book.id))}`)} class="book-card svelte-pg9a6y">`);
        if (book.cover_url) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<img${attr("src", book.cover_url)}${attr("alt", book.title)} class="book-cover svelte-pg9a6y"/>`);
        } else {
          $$renderer2.push("<!--[-1-->");
          $$renderer2.push(`<div class="book-cover placeholder svelte-pg9a6y"><span>${escape_html(book.title.charAt(0))}</span></div>`);
        }
        $$renderer2.push(`<!--]--> <div class="book-info svelte-pg9a6y"><h3 class="book-title svelte-pg9a6y">${escape_html(book.title)}</h3> <p class="book-authors svelte-pg9a6y">${escape_html(book.authors.join(", "))}</p> `);
        if (book.description) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<p class="book-desc svelte-pg9a6y">${escape_html(book.description.slice(0, 120))}${escape_html(book.description.length > 120 ? "..." : "")}</p>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div></a>`);
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
