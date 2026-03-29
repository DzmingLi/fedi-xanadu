import { a as attr_class, c as escape_html } from "../../../../chunks/index2.js";
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
    let activeTab = "my";
    $$renderer2.push(`<div class="skills-page svelte-1c8tibi"><div class="skills-toolbar svelte-1c8tibi"><div class="toolbar-left svelte-1c8tibi"><div class="tab-bar svelte-1c8tibi"><button${attr_class("tab svelte-1c8tibi", void 0, { "active": activeTab === "my" })}>${escape_html(t("skills.mySkills"))}</button> <button${attr_class("tab svelte-1c8tibi", void 0, { "active": activeTab === "community" })}>${escape_html(t("skills.communityTrees"))}</button></div> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> <div class="toolbar-right svelte-1c8tibi">`);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="legend svelte-1c8tibi"><span class="legend-item svelte-1c8tibi"><span class="dot mastered svelte-1c8tibi"></span>${escape_html(t("skills.mastered"))}</span> <span class="legend-item svelte-1c8tibi"><span class="dot learning svelte-1c8tibi"></span>${escape_html(t("skills.learning"))}</span> <span class="legend-item svelte-1c8tibi"><span class="dot available svelte-1c8tibi"></span>${escape_html(t("skills.available"))}</span> <span class="legend-item svelte-1c8tibi"><span class="dot locked svelte-1c8tibi"></span>${escape_html(t("skills.locked"))}</span></div>`);
    }
    $$renderer2.push(`<!--]--></div></div> `);
    {
      $$renderer2.push("<!--[0-->");
      {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="center-msg svelte-1c8tibi"><p>Loading...</p></div>`);
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
export {
  _page as default
};
