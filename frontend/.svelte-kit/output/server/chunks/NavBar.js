import { c as escape_html, i as attr, d as bind_props, b as stringify } from "./index2.js";
import "@sveltejs/kit/internal";
import "./exports.js";
import "./utils.js";
import "@sveltejs/kit/internal/server";
import "./root.js";
import "./state.svelte.js";
import { g as getAuth } from "./auth.svelte.js";
import { t, L as LOCALES, g as getLocale } from "./index.svelte.js";
function LoginModal($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { open = false } = $$props;
    let handle = "";
    let password = "";
    let loading = false;
    if (open) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="overlay svelte-1y960i6"><div class="modal svelte-1y960i6"><h2 class="svelte-1y960i6">${escape_html(t("nav.login"))}</h2> <p class="hint svelte-1y960i6">${escape_html(t("auth.loginHint"))}</p> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <label class="svelte-1y960i6">${escape_html(t("auth.handle"))} <input type="text"${attr("value", handle)} placeholder="alice.bsky.social"${attr("disabled", loading, true)} class="svelte-1y960i6"/></label> <label class="svelte-1y960i6">${escape_html(t("auth.password"))} <input type="password"${attr("value", password)} placeholder="xxxx-xxxx-xxxx-xxxx"${attr("disabled", loading, true)} class="svelte-1y960i6"/></label> <p class="hint small svelte-1y960i6"><a href="https://bsky.app/settings/app-passwords" target="_blank" rel="noopener" class="svelte-1y960i6">${escape_html(t("auth.createAppPw"))}</a></p> <div class="actions svelte-1y960i6"><button class="btn-cancel svelte-1y960i6"${attr("disabled", loading, true)}>${escape_html(t("common.cancel"))}</button> <button class="btn-login svelte-1y960i6"${attr("disabled", !handle, true)}>${escape_html(t("auth.submit"))}</button></div></div></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
    bind_props($$props, { open });
  });
}
function NavBar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let locale = getLocale();
    let isDark = localStorage.getItem("theme") === "dark" || !localStorage.getItem("theme") && window.matchMedia("(prefers-color-scheme: dark)").matches;
    let loginOpen = false;
    let user = getAuth();
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<nav class="svelte-q971rm"><a href="/" class="brand svelte-q971rm">Fedi-Xanadu</a> <div class="nav-links svelte-q971rm"><a href="/questions" class="svelte-q971rm">${escape_html(t("nav.questions"))}</a> <a href="/skills" class="svelte-q971rm">${escape_html(t("nav.skills"))}</a> <a href="/library" class="svelte-q971rm">${escape_html(t("nav.library"))}</a> <a href="/books" class="svelte-q971rm">${escape_html(t("nav.books"))}</a> <a href="/roadmap" class="svelte-q971rm">${escape_html(t("nav.roadmap"))}</a> <a href="/about" class="svelte-q971rm">${escape_html(t("nav.about"))}</a></div> <div class="nav-right svelte-q971rm"><button type="button" class="locale-toggle svelte-q971rm" title="Switch language">${escape_html((() => {
        const codes = LOCALES.map((l) => l.code);
        return LOCALES[(codes.indexOf(locale) + 1) % codes.length].label;
      })())}</button> <button type="button" class="theme-toggle svelte-q971rm"${attr("title", isDark ? "Light mode" : "Dark mode")}>`);
      if (isDark) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        $$renderer3.push(`<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>`);
      }
      $$renderer3.push(`<!--]--></button> <button type="button" class="search-btn svelte-q971rm" aria-label="Search"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg></button> `);
      if (user) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<a href="/notifications" class="notif-btn svelte-q971rm"${attr("title", t("nav.notifications"))}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path><path d="M13.73 21a2 2 0 0 1-3.46 0"></path></svg> `);
        {
          $$renderer3.push("<!--[-1-->");
        }
        $$renderer3.push(`<!--]--></a> <div class="user-menu svelte-q971rm"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(user.did))}`)} class="user-link svelte-q971rm">`);
        if (user.avatar) {
          $$renderer3.push("<!--[0-->");
          $$renderer3.push(`<img${attr("src", user.avatar)} alt="" class="user-avatar svelte-q971rm"/>`);
        } else {
          $$renderer3.push("<!--[-1-->");
        }
        $$renderer3.push(`<!--]--> <span class="user-handle svelte-q971rm">@${escape_html(user.handle)}</span></a> <button class="btn-logout svelte-q971rm">${escape_html(t("nav.logout"))}</button></div>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        $$renderer3.push(`<button class="btn-login svelte-q971rm">${escape_html(t("nav.login"))}</button>`);
      }
      $$renderer3.push(`<!--]--> `);
      if (user) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<a href="/settings" class="settings-btn svelte-q971rm"${attr("title", t("nav.settings"))}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg></a> <a href="/drafts" class="btn-drafts svelte-q971rm">${escape_html(t("nav.drafts"))}</a>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--> <a href="/new" class="btn-new svelte-q971rm">${escape_html(t("nav.newArticle"))}</a> <a href="/new-series" class="btn-new svelte-q971rm">${escape_html(t("nav.newSeries"))}</a></div></nav> `);
      LoginModal($$renderer3, {
        get open() {
          return loginOpen;
        },
        set open($$value) {
          loginOpen = $$value;
          $$settled = false;
        }
      });
      $$renderer3.push(`<!----> `);
      {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]-->`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
export {
  NavBar as N
};
