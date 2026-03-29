import { c as escape_html, i as attr, e as ensure_array_like, d as bind_props, j as derived, l as attr_style, b as stringify, a as attr_class, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import { p as page } from "../../../../chunks/stores.js";
import { a as getToken, g as getAuth } from "../../../../chunks/auth.svelte.js";
import { i as isBlocked, a as authorName } from "../../../../chunks/display.js";
import { t } from "../../../../chunks/index.svelte.js";
/* empty css                                                             */
/* empty css                                                     */
import { c as createQuery } from "../../../../chunks/createQuery.js";
function html(value) {
  var html2 = String(value ?? "");
  var open = "<!---->";
  return open + html2 + "<!---->";
}
const BASE = "/api/v1";
function authHeaders() {
  const token = getToken();
  const headers = {};
  if (token) headers["Authorization"] = `Bearer ${token}`;
  return headers;
}
async function get(path, signal) {
  const res = await fetch(`${BASE}${path}`, { headers: authHeaders(), signal });
  if (!res.ok) {
    if (res.status === 429) throw new Error("请求过于频繁，请稍后再试");
    throw new Error(`${res.status} ${res.statusText}`);
  }
  return res.json();
}
const listComments = (uri) => get(`/comments?uri=${encodeURIComponent(uri)}`);
const getMyCommentVotes = (uri) => get(`/comments/my-votes?uri=${encodeURIComponent(uri)}`);
function CommentThread($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { contentUri, contentEl = void 0 } = $$props;
    let comments = [];
    let commentBody = "";
    let editingCommentId = null;
    let editingCommentBody = "";
    let replyingToId = null;
    let replyBody = "";
    let quoteText = null;
    let myCommentVotes = {};
    let isLoggedIn = derived(() => !!getAuth());
    let visibleComments = derived(() => comments.filter((c) => !isBlocked(c.did)));
    let rootComments = derived(() => visibleComments().filter((c) => !c.parent_id));
    function getReplies(parentId) {
      return visibleComments().filter((c) => c.parent_id === parentId);
    }
    function setQuoteText(text) {
      quoteText = text;
    }
    async function loadComments() {
      try {
        comments = await listComments(contentUri);
        if (getAuth()) {
          getMyCommentVotes(contentUri).then((votes) => {
            const map = {};
            for (const v of votes) map[v.comment_id] = v.value;
            myCommentVotes = map;
          }).catch(() => {
          });
        }
      } catch {
      }
    }
    function getCommentCount() {
      return comments.length;
    }
    function commentNode($$renderer3, c, depth) {
      $$renderer3.push(`<div class="comment-item svelte-1wjobo5"${attr_style("", { "margin-left": `${stringify(depth * 24)}px` })}><div class="comment-header svelte-1wjobo5"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(c.did))}`)} class="comment-author svelte-1wjobo5">${escape_html(c.author_handle ? `@${c.author_handle}` : c.did.slice(0, 20) + "…")}</a> <span class="comment-date svelte-1wjobo5">${escape_html(c.created_at.split("T")[0])}</span> `);
      if (getAuth()?.did === c.did) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<button class="comment-action svelte-1wjobo5"${attr("title", t("comments.edit"))}><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg></button> <button class="comment-action danger svelte-1wjobo5"${attr("title", t("comments.delete"))}><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg></button>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></div> `);
      if (editingCommentId === c.id) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<div class="comment-edit svelte-1wjobo5"><textarea rows="3" class="svelte-1wjobo5">`);
        const $$body = escape_html(editingCommentBody);
        if ($$body) {
          $$renderer3.push(`${$$body}`);
        }
        $$renderer3.push(`</textarea> <div class="comment-edit-actions svelte-1wjobo5"><button class="comment-submit svelte-1wjobo5">${escape_html(t("comments.save"))}</button> <button class="comment-cancel svelte-1wjobo5">${escape_html(t("comments.cancel"))}</button></div></div>`);
      } else {
        $$renderer3.push("<!--[-1-->");
        if (c.quote_text) {
          $$renderer3.push("<!--[0-->");
          $$renderer3.push(`<blockquote class="comment-quote svelte-1wjobo5" role="button" tabindex="0">${escape_html(c.quote_text)}</blockquote>`);
        } else {
          $$renderer3.push("<!--[-1-->");
        }
        $$renderer3.push(`<!--]--> <div class="comment-body svelte-1wjobo5">${escape_html(c.body)}</div>`);
      }
      $$renderer3.push(`<!--]--> <div class="comment-footer svelte-1wjobo5"><div class="comment-vote-btns svelte-1wjobo5"><button${attr_class("vote-btn svelte-1wjobo5", void 0, { "active": myCommentVotes[c.id] === 1 })}${attr("title", t("common.upvote"))}><svg width="14" height="14" viewBox="0 0 24 24"${attr("fill", myCommentVotes[c.id] === 1 ? "currentColor" : "none")} stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14z"></path><path d="M7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3"></path></svg></button> <span${attr_class("vote-count svelte-1wjobo5", void 0, { "positive": c.vote_score > 0, "negative": c.vote_score < 0 })}>${escape_html(c.vote_score)}</span> <button${attr_class("vote-btn svelte-1wjobo5", void 0, { "active": myCommentVotes[c.id] === -1 })}${attr("title", t("common.downvote"))}><svg width="14" height="14" viewBox="0 0 24 24"${attr("fill", myCommentVotes[c.id] === -1 ? "currentColor" : "none")} stroke="currentColor" stroke-width="2"><path d="M10 15v4a3 3 0 003 3l4-9V2H5.72a2 2 0 00-2 1.7l-1.38 9a2 2 0 002 2.3H10z"></path><path d="M17 2h3a2 2 0 012 2v7a2 2 0 01-2 2h-3"></path></svg></button></div> `);
      if (isLoggedIn() && depth < 3) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<button class="reply-btn svelte-1wjobo5">${escape_html(t("common.reply"))}</button>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></div> `);
      if (replyingToId === c.id) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<div class="reply-form svelte-1wjobo5"><textarea rows="2"${attr("placeholder", t("article.writeReply"))} class="svelte-1wjobo5">`);
        const $$body_1 = escape_html(replyBody);
        if ($$body_1) {
          $$renderer3.push(`${$$body_1}`);
        }
        $$renderer3.push(`</textarea> <div class="reply-actions svelte-1wjobo5"><button class="comment-submit svelte-1wjobo5"${attr("disabled", !replyBody.trim(), true)}>${escape_html(t("common.send"))}</button> <button class="comment-cancel svelte-1wjobo5">${escape_html(t("common.cancel"))}</button></div></div>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--> <!--[-->`);
      const each_array = ensure_array_like(getReplies(c.id));
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let reply = each_array[$$index];
        commentNode($$renderer3, reply, depth + 1);
      }
      $$renderer3.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<div class="comments-section svelte-1wjobo5"><h3 class="comments-title svelte-1wjobo5">${escape_html(t("article.comments"))} (${escape_html(comments.length)})</h3> `);
    if (isLoggedIn()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="comment-form svelte-1wjobo5">`);
      if (quoteText) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="quote-preview svelte-1wjobo5"><blockquote class="svelte-1wjobo5">${escape_html(quoteText)}</blockquote> <button class="quote-remove svelte-1wjobo5"${attr("title", t("common.remove"))}>×</button></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <textarea${attr("placeholder", t("article.writeComment"))} rows="3" class="svelte-1wjobo5">`);
      const $$body_2 = escape_html(commentBody);
      if ($$body_2) {
        $$renderer2.push(`${$$body_2}`);
      }
      $$renderer2.push(`</textarea> <button class="comment-submit svelte-1wjobo5"${attr("disabled", !commentBody.trim(), true)}>${escape_html(t("article.submit"))}</button> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("article.loginToComment"))}</p>`);
    }
    $$renderer2.push(`<!--]--> `);
    if (comments.length === 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta comment-empty svelte-1wjobo5">${escape_html(t("article.noComments"))}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="comment-list svelte-1wjobo5"><!--[-->`);
      const each_array_1 = ensure_array_like(rootComments());
      for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
        let c = each_array_1[$$index_1];
        commentNode($$renderer2, c, 0);
      }
      $$renderer2.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
    bind_props($$props, { setQuoteText, loadComments, getCommentCount });
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let uri = derived(() => store_get($$store_subs ??= {}, "$page", page).url.searchParams.get("uri") ?? "");
    let questionQuery = createQuery({});
    let detail = derived(() => store_get($$store_subs ??= {}, "$questionQuery", questionQuery).data ?? null);
    let loading = derived(() => store_get($$store_subs ??= {}, "$questionQuery", questionQuery).isPending);
    let error = derived(() => store_get($$store_subs ??= {}, "$questionQuery", questionQuery).error?.message ?? "");
    let answerContents = /* @__PURE__ */ new Map();
    let myVote = 0;
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">${escape_html(t("common.loading"))}</p>`);
    } else if (error()) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<p class="error svelte-1jrfby">${escape_html(error())}</p>`);
    } else if (detail()) {
      $$renderer2.push("<!--[2-->");
      const q = detail().question;
      $$renderer2.push(`<div class="question-section svelte-1jrfby"><div class="q-header svelte-1jrfby"><span class="q-badge svelte-1jrfby">${escape_html(t("qa.questionBadge"))}</span> <h1 class="q-title svelte-1jrfby">${escape_html(q.title)}</h1></div> <div class="q-meta svelte-1jrfby"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(q.did))}`)} class="author svelte-1jrfby">${escape_html(authorName(q))}</a> <span>·</span> <span>${escape_html(q.created_at.split(" ")[0])}</span> <span>·</span> <span>${escape_html(t("qa.answerCount", q.answer_count))}</span></div> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="q-actions svelte-1jrfby"><button${attr_class("vote-btn svelte-1jrfby", void 0, { "active": myVote === 1 })}>▲ ${escape_html(t("article.upvote"))}</button> <span class="vote-score svelte-1jrfby">${escape_html(q.vote_score)}</span> <button${attr_class("vote-btn svelte-1jrfby", void 0, { "active": myVote === -1 })}>▼ ${escape_html(t("article.downvote"))}</button> `);
      if (getAuth() && q.did === getAuth()?.did) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<a${attr("href", `/new?edit=${stringify(encodeURIComponent(q.at_uri))}`)} class="edit-link svelte-1jrfby">${escape_html(t("common.edit"))}</a>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div></div> <div class="answers-section svelte-1jrfby"><div class="answers-header svelte-1jrfby"><h2 class="svelte-1jrfby">${escape_html(t("qa.answerCount", detail().answers.length))}</h2> `);
      if (getAuth()) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<button class="btn-answer svelte-1jrfby">${escape_html(t("qa.writeAnswer"))}</button>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (detail().answers.length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="empty svelte-1jrfby">${escape_html(t("qa.noAnswers"))}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<!--[-->`);
        const each_array = ensure_array_like(detail().answers);
        for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
          let answer = each_array[$$index];
          const content = answerContents.get(answer.at_uri);
          $$renderer2.push(`<div class="answer-card svelte-1jrfby"><div class="answer-meta svelte-1jrfby"><a${attr("href", `/profile?did=${stringify(encodeURIComponent(answer.did))}`)} class="author svelte-1jrfby">${escape_html(authorName(answer))}</a> <span>·</span> <span>${escape_html(answer.created_at.split(" ")[0])}</span></div> `);
          if (content) {
            $$renderer2.push("<!--[0-->");
            $$renderer2.push(`<div class="content-body rendered-html svelte-1jrfby">${html(content.html)}</div>`);
          } else {
            $$renderer2.push("<!--[-1-->");
            $$renderer2.push(`<p class="meta">${escape_html(t("common.loading"))}</p>`);
          }
          $$renderer2.push(`<!--]--> <div class="answer-actions svelte-1jrfby"><span class="vote-score svelte-1jrfby">▲ ${escape_html(answer.vote_score)}</span></div></div>`);
        }
        $$renderer2.push(`<!--]-->`);
      }
      $$renderer2.push(`<!--]--></div> <div class="comments-section svelte-1jrfby">`);
      CommentThread($$renderer2, { contentUri: uri() });
      $$renderer2.push(`<!----></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
