import "clsx";
import { s as ssr_context, e as ensure_array_like, a as attr_class, b as stringify, c as escape_html, d as bind_props, f as await_block } from "../../chunks/index2.js";
/* empty css                                               */
import "@sveltejs/kit/internal";
import "../../chunks/exports.js";
import "../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../chunks/root.js";
import "../../chunks/state.svelte.js";
import { g as getAllBindings, C as CATEGORY_LABELS, f as formatKeyDisplay, A as ACTIONS } from "../../chunks/keybindings.svelte.js";
import "../../chunks/auth.svelte.js";
import { QueryClient } from "@tanstack/query-core";
import { s as setQueryClientContext } from "../../chunks/context.js";
function onDestroy(fn) {
  /** @type {SSRContext} */
  ssr_context.r.on_destroy(fn);
}
function QueryClientProvider($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const { client = new QueryClient(), children } = $$props;
    setQueryClientContext(client);
    onDestroy(() => {
      client.unmount();
    });
    children($$renderer2);
    $$renderer2.push(`<!---->`);
  });
}
let toasts = [];
function Toast($$renderer) {
  if (toasts.length > 0) {
    $$renderer.push("<!--[0-->");
    $$renderer.push(`<div class="toast-container svelte-1cpok13"><!--[-->`);
    const each_array = ensure_array_like(toasts);
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let t = each_array[$$index];
      $$renderer.push(`<div${attr_class(`toast toast-${stringify(t.type)}`, "svelte-1cpok13")}>${escape_html(t.message)}</div>`);
    }
    $$renderer.push(`<!--]--></div>`);
  } else {
    $$renderer.push("<!--[-1-->");
  }
  $$renderer.push(`<!--]-->`);
}
function KeyboardShortcuts($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let helpOpen = false;
    let settingsOpen = false;
    let pendingKeys = [];
    let bindings = getAllBindings();
    function openHelp() {
      helpOpen = true;
    }
    function openSettings() {
      settingsOpen = true;
    }
    function groupedActions() {
      const groups = {};
      for (const action of ACTIONS) {
        if (!groups[action.category]) groups[action.category] = [];
        groups[action.category].push(action);
      }
      return groups;
    }
    if (pendingKeys.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="key-pending svelte-t5v7m4">${escape_html(pendingKeys.join(" "))} ...</div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (helpOpen) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="kb-overlay svelte-t5v7m4"><div class="kb-modal svelte-t5v7m4"><div class="kb-header svelte-t5v7m4"><h2 class="svelte-t5v7m4">Keyboard Shortcuts</h2> <button class="kb-close svelte-t5v7m4">×</button></div> <div class="kb-body svelte-t5v7m4"><!--[-->`);
      const each_array = ensure_array_like(Object.entries(groupedActions()));
      for (let $$index_1 = 0, $$length = each_array.length; $$index_1 < $$length; $$index_1++) {
        let [cat, actions] = each_array[$$index_1];
        $$renderer2.push(`<div class="kb-category svelte-t5v7m4"><h3 class="svelte-t5v7m4">${escape_html(CATEGORY_LABELS[cat]?.en ?? cat)}</h3> <!--[-->`);
        const each_array_1 = ensure_array_like(actions);
        for (let $$index = 0, $$length2 = each_array_1.length; $$index < $$length2; $$index++) {
          let action = each_array_1[$$index];
          $$renderer2.push(`<div class="kb-row svelte-t5v7m4"><span class="kb-label svelte-t5v7m4">${escape_html(action.label)}</span> <kbd class="kb-key svelte-t5v7m4">${escape_html(formatKeyDisplay(bindings[action.id] ?? action.defaultKey))}</kbd></div>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      }
      $$renderer2.push(`<!--]--></div> <div class="kb-footer svelte-t5v7m4"><span class="kb-hint svelte-t5v7m4">Press <kbd class="svelte-t5v7m4">?</kbd> to toggle · <kbd class="svelte-t5v7m4">Esc</kbd> to close</span> <button class="kb-settings-btn svelte-t5v7m4">Customize</button></div></div></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (settingsOpen) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="kb-overlay svelte-t5v7m4"><div class="kb-modal kb-settings-modal svelte-t5v7m4"><div class="kb-header svelte-t5v7m4"><h2 class="svelte-t5v7m4">Customize Shortcuts</h2> <button class="kb-close svelte-t5v7m4">×</button></div> <div class="kb-body svelte-t5v7m4">`);
      await_block($$renderer2, import("../../chunks/KeybindingsEditor.js"), () => {
      }, (mod) => {
        if (mod.default) {
          $$renderer2.push("<!--[-->");
          mod.default($$renderer2, {
            onclose: () => {
              settingsOpen = false;
            }
          });
          $$renderer2.push("<!--]-->");
        } else {
          $$renderer2.push("<!--[!-->");
          $$renderer2.push("<!--]-->");
        }
      });
      $$renderer2.push(`<!--]--></div></div></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
    bind_props($$props, { openHelp, openSettings });
  });
}
function _layout($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          staleTime: 3e4,
          gcTime: 5 * 6e4,
          refetchOnWindowFocus: false,
          retry: 1
        }
      }
    });
    let { children } = $$props;
    QueryClientProvider($$renderer2, {
      client: (
        // Redirect old hash URLs to clean paths
        queryClient
      ),
      children: ($$renderer3) => {
        Toast($$renderer3);
        $$renderer3.push(`<!----> `);
        KeyboardShortcuts($$renderer3, {});
        $$renderer3.push(`<!----> `);
        children($$renderer3);
        $$renderer3.push(`<!---->`);
      }
    });
  });
}
export {
  _layout as default
};
