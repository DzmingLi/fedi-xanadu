import { c as escape_html, e as ensure_array_like, a as attr_class, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "../../../../chunks/auth.svelte.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import { t } from "../../../../chunks/index.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const notificationsQuery = createQuery({});
    let notifications = derived(() => store_get($$store_subs ??= {}, "$notificationsQuery", notificationsQuery).data ?? []);
    let loading = derived(() => store_get($$store_subs ??= {}, "$notificationsQuery", notificationsQuery).isPending);
    let unreadCount = derived(() => notifications().filter((n) => !n.read).length);
    function actionText(kind) {
      switch (kind) {
        case "comment_reply":
          return t("notification.commentReply");
        case "article_comment":
          return t("notification.articleComment");
        case "new_follower":
          return t("notification.newFollower");
        case "article_fork":
          return t("notification.articleFork");
        case "new_answer":
          return t("notification.newAnswer");
        default:
          return kind;
      }
    }
    function timeAgo(dateStr) {
      const diff = Date.now() - new Date(dateStr).getTime();
      const mins = Math.floor(diff / 6e4);
      if (mins < 1) return t("notification.justNow");
      if (mins < 60) return `${mins}m`;
      const hours = Math.floor(mins / 60);
      if (hours < 24) return `${hours}h`;
      const days = Math.floor(hours / 24);
      return `${days}d`;
    }
    $$renderer2.push(`<h1 class="svelte-104934c">${escape_html(t("nav.notifications"))}</h1> `);
    if (unreadCount() > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<button class="mark-all-btn svelte-104934c">${escape_html(t("notification.markAllRead"))}</button>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else if (notifications().length === 0) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="empty svelte-104934c">${escape_html(t("notification.empty"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="notification-list svelte-104934c"><!--[-->`);
      const each_array = ensure_array_like(notifications());
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let n = each_array[$$index];
        $$renderer2.push(`<div${attr_class("notification-item svelte-104934c", void 0, { "unread": !n.read })}><div class="notification-body svelte-104934c"><span class="actor svelte-104934c">${escape_html(n.actor_handle ? `@${n.actor_handle}` : n.actor_did.slice(0, 20))}</span> <span class="action svelte-104934c">${escape_html(actionText(n.kind))}</span> `);
        if (n.target_title) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="target svelte-104934c">"${escape_html(n.target_title)}"</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div> <span class="time svelte-104934c">${escape_html(timeAgo(n.created_at))}</span></div>`);
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
