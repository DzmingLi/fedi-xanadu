import { c as escape_html, e as ensure_array_like, a as attr_class, i as attr, b as stringify, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { g as getAuth } from "../../../../chunks/auth.svelte.js";
import { t, g as getLocale, a as LANG_NAMES } from "../../../../chunks/index.svelte.js";
import { C as CATEGORY_LABELS, f as formatKeyDisplay, A as ACTIONS } from "../../../../chunks/keybindings.svelte.js";
/* empty css                                                     */
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let locale = getLocale();
    let saving = false;
    let nativeLang = "zh";
    let knownLangs = ["zh"];
    let preferNative = true;
    let hideUnknown = false;
    let defaultFormat = "typst";
    let email = "";
    let bookmarksPublic = false;
    let blockedUsers = [];
    let members = [];
    let newMemberDid = "";
    let bindings = {};
    let editingAction = null;
    const ALL_LANGS = ["zh", "en", "ja", "ko", "fr", "de", "es", "pt"];
    const FORMATS = ["typst", "markdown"];
    let settingsQuery = createQuery({});
    createQuery({});
    createQuery({});
    createQuery({});
    createQuery({});
    let loading = derived(() => store_get($$store_subs ??= {}, "$settingsQuery", settingsQuery).isPending);
    function onNativeChange(e) {
      const val = e.target.value;
      nativeLang = val;
      if (!knownLangs.includes(val)) {
        knownLangs = [val, ...knownLangs];
      }
    }
    let actionsByCategory = derived(() => {
      const map = /* @__PURE__ */ new Map();
      for (const action of ACTIONS) {
        const cat = action.category;
        const arr = map.get(cat) || [];
        arr.push(action);
        map.set(cat, arr);
      }
      return map;
    });
    $$renderer2.push(`<div class="settings-page svelte-692nw1"><h1 class="svelte-692nw1">${escape_html(t("settings.title"))}</h1> `);
    if (!getAuth()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="login-hint svelte-692nw1">${escape_html(t("nav.login"))}</p>`);
    } else if (loading()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="meta svelte-692nw1">${escape_html(t("common.loading"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("settings.nativeLang"))}</h2> <p class="hint svelte-692nw1">${escape_html(t("settings.nativeLangHint"))}</p> `);
      $$renderer2.select(
        {
          value: nativeLang,
          onchange: onNativeChange,
          class: "select-input"
        },
        ($$renderer3) => {
          $$renderer3.push(`<!--[-->`);
          const each_array = ensure_array_like(ALL_LANGS);
          for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
            let lang = each_array[$$index];
            $$renderer3.option(
              { value: lang, class: "" },
              ($$renderer4) => {
                $$renderer4.push(`${escape_html(LANG_NAMES[lang] || lang)}`);
              },
              "svelte-692nw1"
            );
          }
          $$renderer3.push(`<!--]-->`);
        },
        "svelte-692nw1"
      );
      $$renderer2.push(`</div> <div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("settings.knownLangs"))}</h2> <p class="hint svelte-692nw1">${escape_html(t("settings.knownLangsHint"))}</p> <div class="lang-chips svelte-692nw1"><!--[-->`);
      const each_array_1 = ensure_array_like(ALL_LANGS);
      for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
        let lang = each_array_1[$$index_1];
        $$renderer2.push(`<button${attr_class("lang-chip svelte-692nw1", void 0, {
          "active": knownLangs.includes(lang),
          "native": lang === nativeLang
        })}${attr("disabled", lang === nativeLang, true)}>${escape_html(LANG_NAMES[lang] || lang)}</button>`);
      }
      $$renderer2.push(`<!--]--></div></div> <div class="settings-section svelte-692nw1"><label class="toggle-row svelte-692nw1"><input type="checkbox"${attr("checked", preferNative, true)} class="svelte-692nw1"/> <span class="toggle-label svelte-692nw1">${escape_html(t("settings.preferNative"))}</span></label> <p class="hint svelte-692nw1">${escape_html(t("settings.preferNativeHint"))}</p></div> <div class="settings-section svelte-692nw1"><label class="toggle-row svelte-692nw1"><input type="checkbox"${attr("checked", hideUnknown, true)} class="svelte-692nw1"/> <span class="toggle-label svelte-692nw1">${escape_html(t("settings.hideUnknown"))}</span></label> <p class="hint svelte-692nw1">${escape_html(t("settings.hideUnknownHint"))}</p></div> <div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("settings.defaultFormat"))}</h2> <div class="format-options svelte-692nw1"><!--[-->`);
      const each_array_2 = ensure_array_like(FORMATS);
      for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
        let fmt = each_array_2[$$index_2];
        $$renderer2.push(`<label class="radio-row svelte-692nw1"><input type="radio" name="format"${attr("value", fmt)}${attr("checked", defaultFormat === fmt, true)} class="svelte-692nw1"/> <span class="svelte-692nw1">${escape_html(fmt === "typst" ? "Typst" : "Markdown + KaTeX")}</span></label>`);
      }
      $$renderer2.push(`<!--]--></div></div> <div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("settings.email"))}</h2> <p class="hint svelte-692nw1">${escape_html(t("settings.emailHint"))}</p> <input type="email"${attr("value", email)} placeholder="user@example.com" class="text-input svelte-692nw1"/></div> <div class="settings-section svelte-692nw1"><label class="toggle-row svelte-692nw1"><input type="checkbox"${attr("checked", bookmarksPublic, true)} class="svelte-692nw1"/> <span class="toggle-label svelte-692nw1">${escape_html(t("settings.bookmarksPublic"))}</span></label> <p class="hint svelte-692nw1">${escape_html(t("settings.bookmarksPublicHint"))}</p> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div> <div class="settings-actions svelte-692nw1"><button class="save-btn svelte-692nw1"${attr("disabled", saving, true)}>${escape_html(t("common.save"))}</button></div> <div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("block.blockedUsers"))}</h2> `);
      if (blockedUsers.length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="hint svelte-692nw1">${escape_html(t("block.empty"))}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<!--[-->`);
        const each_array_4 = ensure_array_like(blockedUsers);
        for (let $$index_4 = 0, $$length = each_array_4.length; $$index_4 < $$length; $$index_4++) {
          let b = each_array_4[$$index_4];
          $$renderer2.push(`<div class="blocked-row svelte-692nw1"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(b.blocked_did))}`)} class="blocked-name svelte-692nw1">${escape_html(b.display_name || b.handle || b.blocked_did.slice(0, 20))}</a> <button class="unblock-btn svelte-692nw1">${escape_html(t("block.unblock"))}</button></div>`);
        }
        $$renderer2.push(`<!--]-->`);
      }
      $$renderer2.push(`<!--]--></div> <div class="settings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("books.members"))}</h2> <p class="hint svelte-692nw1">${escape_html(t("settings.membersHint"))}</p> <div class="member-add-row svelte-692nw1"><input type="text"${attr("value", newMemberDid)} placeholder="DID or handle" class="member-input svelte-692nw1"/> <button class="btn svelte-692nw1">${escape_html(t("books.addMember"))}</button></div> `);
      if (members.length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="hint svelte-692nw1">${escape_html(t("settings.noMembers"))}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<!--[-->`);
        const each_array_5 = ensure_array_like(members);
        for (let $$index_5 = 0, $$length = each_array_5.length; $$index_5 < $$length; $$index_5++) {
          let m = each_array_5[$$index_5];
          $$renderer2.push(`<div class="blocked-row svelte-692nw1"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(m.member_did))}`)} class="blocked-name svelte-692nw1">${escape_html(m.member_did.slice(0, 24))}…</a> <button class="unblock-btn svelte-692nw1">${escape_html(t("books.removeMember"))}</button></div>`);
        }
        $$renderer2.push(`<!--]-->`);
      }
      $$renderer2.push(`<!--]--></div> <div class="settings-section keybindings-section svelte-692nw1"><h2 class="svelte-692nw1">${escape_html(t("settings.keybindings"))}</h2> <!--[-->`);
      const each_array_6 = ensure_array_like([...actionsByCategory()]);
      for (let $$index_7 = 0, $$length = each_array_6.length; $$index_7 < $$length; $$index_7++) {
        let [category, actions] = each_array_6[$$index_7];
        $$renderer2.push(`<h3 class="kb-category svelte-692nw1">${escape_html(CATEGORY_LABELS[category]?.[locale === "zh" ? "zh" : "en"] || category)}</h3> <!--[-->`);
        const each_array_7 = ensure_array_like(actions);
        for (let $$index_6 = 0, $$length2 = each_array_7.length; $$index_6 < $$length2; $$index_6++) {
          let action = each_array_7[$$index_6];
          $$renderer2.push(`<div class="kb-row svelte-692nw1"><span class="kb-action svelte-692nw1">${escape_html(locale === "zh" ? action.labelZh : action.label)}</span> `);
          if (editingAction === action.id) {
            $$renderer2.push("<!--[0-->");
            $$renderer2.push(`<span class="kb-key capturing svelte-692nw1">...</span>`);
          } else {
            $$renderer2.push("<!--[-1-->");
            $$renderer2.push(`<button class="kb-key svelte-692nw1">${escape_html(formatKeyDisplay(bindings[action.id] || ""))}</button>`);
          }
          $$renderer2.push(`<!--]--> <button class="kb-reset svelte-692nw1"${attr("title", t("kb.resetDefault"))}>×</button></div>`);
        }
        $$renderer2.push(`<!--]-->`);
      }
      $$renderer2.push(`<!--]--> <div class="kb-actions svelte-692nw1"><button class="kb-reset-all svelte-692nw1">${escape_html(t("kb.resetAll"))}</button> <button class="save-btn svelte-692nw1">${escape_html(t("kb.save"))}</button></div></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
