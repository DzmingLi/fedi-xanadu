import "clsx";
import { N as NavBar } from "../../../chunks/NavBar.js";
import { c as escape_html, e as ensure_array_like, i as attr, j as derived } from "../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../chunks/root.js";
import "../../../chunks/state.svelte.js";
import { g as getAuth } from "../../../chunks/auth.svelte.js";
import { t } from "../../../chunks/index.svelte.js";
function Sidebar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let follows = [];
    let user = getAuth();
    function displayName(f) {
      return f.display_name || f.handle || f.follows_did.slice(0, 20);
    }
    $$renderer2.push(`<aside class="sidebar svelte-129hoe0"><nav class="sidebar-nav svelte-129hoe0"><a href="/" class="sidebar-link active-home svelte-129hoe0"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline></svg> ${escape_html(t("sidebar.home"))}</a> <a href="/questions" class="sidebar-link svelte-129hoe0"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3"></path><line x1="12" y1="17" x2="12.01" y2="17"></line></svg> ${escape_html(t("nav.questions"))}</a> <a href="/skills" class="sidebar-link svelte-129hoe0"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg> ${escape_html(t("sidebar.skills"))}</a> <a href="/library" class="sidebar-link svelte-129hoe0"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"></path></svg> ${escape_html(t("sidebar.library"))}</a></nav> `);
    if (user && follows.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="sidebar-divider svelte-129hoe0"></div> <div class="sidebar-section svelte-129hoe0"><div class="sidebar-heading svelte-129hoe0">${escape_html(t("home.following"))}</div> <nav class="sidebar-nav follows-list svelte-129hoe0"><!--[-->`);
      const each_array = ensure_array_like(follows);
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let f = each_array[$$index];
        $$renderer2.push(`<button class="sidebar-link follow-link svelte-129hoe0">`);
        if (f.avatar_url) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<img${attr("src", f.avatar_url)} alt="" class="follow-avatar svelte-129hoe0"/>`);
        } else {
          $$renderer2.push("<!--[-1-->");
          $$renderer2.push(`<span class="follow-avatar-placeholder svelte-129hoe0"></span>`);
        }
        $$renderer2.push(`<!--]--> <span class="follow-name svelte-129hoe0">${escape_html(displayName(f))}</span> `);
        if (f.has_update) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="update-dot svelte-129hoe0"></span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></button>`);
      }
      $$renderer2.push(`<!--]--></nav></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="sidebar-divider svelte-129hoe0"></div> <nav class="sidebar-nav sidebar-secondary svelte-129hoe0"><a href="/guide" class="sidebar-link svelte-129hoe0">${escape_html(t("sidebar.guide"))}</a> <a href="/about" class="sidebar-link svelte-129hoe0">${escape_html(t("sidebar.about"))}</a></nav> <div class="sidebar-divider svelte-129hoe0"></div> <div class="sidebar-section svelte-129hoe0"><div class="sidebar-heading svelte-129hoe0">Fedi-Xanadu</div> <p class="sidebar-text svelte-129hoe0">${escape_html(t("sidebar.desc"))}</p> <p class="sidebar-text svelte-129hoe0"><a href="/about">${escape_html(t("sidebar.learnMore"))} →</a></p></div></aside>`);
  });
}
function RightSidebar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let skills = [];
    let litCount = derived(() => skills.length);
    $$renderer2.push(`<aside class="right-sidebar svelte-17x09pi"><div class="sidebar-section svelte-17x09pi"><div class="sidebar-heading svelte-17x09pi">${escape_html(t("rsidebar.yourSkills"))}</div> <p class="sidebar-text svelte-17x09pi">${escape_html(t("rsidebar.litTags", litCount()))}</p> <a href="/skills" class="sidebar-link-small svelte-17x09pi">${escape_html(t("rsidebar.manageTree"))}</a></div> <div class="sidebar-divider svelte-17x09pi"></div> <div class="sidebar-section svelte-17x09pi"><div class="sidebar-heading svelte-17x09pi">${escape_html(t("rsidebar.explore"))}</div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="sidebar-text svelte-17x09pi">${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]--></div></aside>`);
  });
}
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="layout-wide svelte-1h8i7x8">`);
  NavBar($$renderer);
  $$renderer.push(`<!----> <div class="layout-body svelte-1h8i7x8">`);
  Sidebar($$renderer);
  $$renderer.push(`<!----> <main class="layout-main svelte-1h8i7x8">`);
  children($$renderer);
  $$renderer.push(`<!----></main> `);
  RightSidebar($$renderer);
  $$renderer.push(`<!----></div></div>`);
}
export {
  _layout as default
};
