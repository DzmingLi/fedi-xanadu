import { e as ensure_array_like, c as escape_html } from "./index2.js";
import { g as getAllBindings, C as CATEGORY_LABELS, f as formatKeyDisplay, A as ACTIONS } from "./keybindings.svelte.js";
import { a as getToken } from "./auth.svelte.js";
function KeybindingsEditor($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { onclose } = $$props;
    let bindings = getAllBindings();
    let recording = null;
    let recordedKeys = [];
    function groupedActions() {
      const groups = {};
      for (const action of ACTIONS) {
        if (!groups[action.category]) groups[action.category] = [];
        groups[action.category].push(action);
      }
      return groups;
    }
    $$renderer2.push(`<div class="editor svelte-1ejh6bo"><!--[-->`);
    const each_array = ensure_array_like(Object.entries(groupedActions()));
    for (let $$index_1 = 0, $$length = each_array.length; $$index_1 < $$length; $$index_1++) {
      let [cat, actions] = each_array[$$index_1];
      $$renderer2.push(`<div class="ed-category svelte-1ejh6bo"><h3 class="svelte-1ejh6bo">${escape_html(CATEGORY_LABELS[cat]?.en ?? cat)}</h3> <!--[-->`);
      const each_array_1 = ensure_array_like(actions);
      for (let $$index = 0, $$length2 = each_array_1.length; $$index < $$length2; $$index++) {
        let action = each_array_1[$$index];
        $$renderer2.push(`<div class="ed-row svelte-1ejh6bo"><span class="ed-label svelte-1ejh6bo">${escape_html(action.label)}</span> <div class="ed-key-area svelte-1ejh6bo">`);
        if (recording === action.id) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="ed-recording svelte-1ejh6bo">`);
          if (recordedKeys.length > 0) {
            $$renderer2.push("<!--[0-->");
            $$renderer2.push(`${escape_html(recordedKeys.join(" "))} ...`);
          } else {
            $$renderer2.push("<!--[-1-->");
            $$renderer2.push(`Press key...`);
          }
          $$renderer2.push(`<!--]--></span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
          $$renderer2.push(`<button class="ed-key-btn svelte-1ejh6bo">${escape_html(formatKeyDisplay(bindings[action.id]))}</button>`);
        }
        $$renderer2.push(`<!--]--> `);
        if (bindings[action.id] !== action.defaultKey) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<button class="ed-reset-btn svelte-1ejh6bo" title="Reset to default">×</button>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<!--]--></div> <div class="ed-footer svelte-1ejh6bo"><button class="ed-reset-all svelte-1ejh6bo">Reset all</button> <div class="ed-footer-right svelte-1ejh6bo">`);
    if (!getToken()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<span class="ed-hint svelte-1ejh6bo">Log in to sync shortcuts to PDS</span>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <button class="ed-save-btn svelte-1ejh6bo">${escape_html("Close")}</button></div></div>`);
  });
}
export {
  KeybindingsEditor as default
};
