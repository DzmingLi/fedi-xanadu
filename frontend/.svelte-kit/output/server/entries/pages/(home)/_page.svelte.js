import { i as attr, b as stringify, c as escape_html, e as ensure_array_like, a as attr_class, j as derived, k as store_get, u as unsubscribe_stores } from "../../../chunks/index2.js";
import "../../../chunks/auth.svelte.js";
import { t as tagName, a as authorName, d as deduplicateByTranslation, b as deduplicateSeriesByTranslation } from "../../../chunks/display.js";
import { t, g as getLocale } from "../../../chunks/index.svelte.js";
import { b as buildArticleRowMap, a as buildSeriesArticleMaps } from "../../../chunks/series.js";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../chunks/root.js";
import "../../../chunks/state.svelte.js";
/* empty css                                                     */
import { c as createQuery } from "../../../chunks/createQuery.js";
function PostCard($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      article = void 0,
      series = void 0,
      articleCount = 0,
      articleTeaches = [],
      articlePrereqs = [],
      variant = "home"
    } = $$props;
    function seriesAuthor(s) {
      if (s.author_handle) return `@${s.author_handle}`;
      return s.created_by.replace("did:plc:", "").replace("did:web:", "").slice(0, 16);
    }
    if (article) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<a${attr("href", `/article?uri=${stringify(encodeURIComponent(article.at_uri))}`)} class="post-card svelte-podw4w"><div class="card-top svelte-podw4w">`);
      if (article.kind === "question") {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<span class="question-badge svelte-podw4w">${escape_html(t("qa.questionBadge"))}</span>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <span class="post-title svelte-podw4w">${escape_html(article.title)}</span></div> `);
      if (articleTeaches.length > 0 || articlePrereqs.length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="card-tags svelte-podw4w"><!--[-->`);
        const each_array = ensure_array_like(articleTeaches);
        for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
          let t2 = each_array[$$index];
          $$renderer2.push(`<span class="tag" role="link" tabindex="0">${escape_html(tagName(t2.tag_names, t2.tag_name, t2.tag_id))}</span>`);
        }
        $$renderer2.push(`<!--]--> <!--[-->`);
        const each_array_1 = ensure_array_like(articlePrereqs);
        for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
          let p = each_array_1[$$index_1];
          $$renderer2.push(`<span${attr_class(`tag ${stringify(p.prereq_type)}`, "svelte-podw4w")} role="link" tabindex="0">${escape_html(tagName(p.tag_names, p.tag_name, p.tag_id))}</span>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (article.description) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="post-desc svelte-podw4w">${escape_html(article.description)}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="card-bottom svelte-podw4w"><span class="post-meta svelte-podw4w">`);
      if (variant === "home") {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`${escape_html(authorName(article))} ·`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]-->${escape_html(article.created_at.split(" ")[0])}</span> `);
      if (variant === "home") {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<span class="card-stats svelte-podw4w">`);
        if (article.vote_score !== 0) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="stat svelte-podw4w"${attr("title", t("home.votes"))}>▲ ${escape_html(article.vote_score)}</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--> `);
        if (article.bookmark_count > 0) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<span class="stat svelte-podw4w"${attr("title", t("home.bookmarks"))}>★ ${escape_html(article.bookmark_count)}</span>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></span>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div></a>`);
    } else if (series) {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<a${attr("href", `/series?id=${stringify(encodeURIComponent(series.id))}`)} class="post-card series-card svelte-podw4w"><div class="card-top svelte-podw4w"><span class="post-title svelte-podw4w">${escape_html(series.title)}</span> <span class="series-badge svelte-podw4w">${escape_html(t("home.series"))}</span></div> `);
      if (series.description) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="post-desc svelte-podw4w">${escape_html(series.description)}</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="card-bottom svelte-podw4w"><span class="post-meta svelte-podw4w">`);
      if (variant === "home") {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`${escape_html(seriesAuthor(series))} ·`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]-->${escape_html(series.created_at.split(" ")[0])}</span> <span class="card-stats svelte-podw4w"><span class="stat svelte-podw4w">${escape_html(articleCount)} ${escape_html(variant === "home" ? t("home.lectures") : t("profile.lectureCount"))}</span></span></div></a>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let locale = getLocale();
    const articlesQuery = createQuery({});
    const teachesQuery = createQuery({});
    const prereqsQuery = createQuery({});
    const tagsQuery = createQuery({});
    const tagTreeQuery = createQuery({});
    const seriesQuery = createQuery({});
    const seriesArtsQuery = createQuery({});
    let loading = derived(() => store_get($$store_subs ??= {}, "$articlesQuery", articlesQuery).isLoading || store_get($$store_subs ??= {}, "$tagsQuery", tagsQuery).isLoading || store_get($$store_subs ??= {}, "$tagTreeQuery", tagTreeQuery).isLoading || store_get($$store_subs ??= {}, "$seriesQuery", seriesQuery).isLoading || store_get($$store_subs ??= {}, "$seriesArtsQuery", seriesArtsQuery).isLoading || store_get($$store_subs ??= {}, "$teachesQuery", teachesQuery).isLoading || store_get($$store_subs ??= {}, "$prereqsQuery", prereqsQuery).isLoading);
    let articles = derived(() => store_get($$store_subs ??= {}, "$articlesQuery", articlesQuery).data ?? []);
    let allTags = derived(() => store_get($$store_subs ??= {}, "$tagsQuery", tagsQuery).data ?? []);
    let tagTree = derived(() => store_get($$store_subs ??= {}, "$tagTreeQuery", tagTreeQuery).data ?? []);
    let allSeries = derived(() => store_get($$store_subs ??= {}, "$seriesQuery", seriesQuery).data ?? []);
    let articleTeaches = derived(() => buildArticleRowMap(store_get($$store_subs ??= {}, "$teachesQuery", teachesQuery).data ?? []));
    let articlePrereqs = derived(() => buildArticleRowMap(store_get($$store_subs ??= {}, "$prereqsQuery", prereqsQuery).data ?? []));
    let seriesMaps = derived(() => buildSeriesArticleMaps(store_get($$store_subs ??= {}, "$seriesArtsQuery", seriesArtsQuery).data ?? []));
    let seriesArticleUris = derived(() => seriesMaps().seriesArticleUris);
    let seriesArticleMap = derived(() => seriesMaps().seriesArticleMap);
    const STORAGE_KEY = "fx_interests";
    let interests = loadLocalInterests();
    function loadLocalInterests() {
      try {
        const s = localStorage.getItem(STORAGE_KEY);
        return s ? JSON.parse(s) : [];
      } catch {
        return [];
      }
    }
    const FIELD_IDS = ["math", "physics", "cs", "economics"];
    function fieldFallback(id) {
      return t(`field.${id}`) !== `field.${id}` ? t(`field.${id}`) : id;
    }
    let topCategories = derived(() => {
      const tagMap = new Map(allTags().map((t2) => [t2.id, t2]));
      const hasParent = /* @__PURE__ */ new Set();
      const isParent = /* @__PURE__ */ new Set();
      for (const e of tagTree()) {
        hasParent.add(e.child_tag);
        isParent.add(e.parent_tag);
      }
      const extraRoots = Array.from(isParent).filter((id) => !hasParent.has(id) && !FIELD_IDS.includes(id));
      const allRoots = [...FIELD_IDS, ...extraRoots];
      return allRoots.map((id) => tagMap.get(id) ?? {
        id,
        name: fieldFallback(id),
        description: null,
        created_by: "system",
        created_at: ""
      }).filter((t2) => !!t2);
    });
    let activeTab = "all";
    function trendingScore(a) {
      const score = a.vote_score || 0;
      const created = new Date(a.created_at).getTime();
      const now = Date.now();
      const ageHours = Math.max(1, (now - created) / (1e3 * 60 * 60));
      return (score + 1) / Math.pow(ageHours, 1.5);
    }
    function buildFeed(arts) {
      const items = [];
      const artUriSet = new Set(arts.map((a) => a.at_uri));
      for (const a of arts) {
        if (!seriesArticleUris().has(a.at_uri)) {
          items.push({ type: "article", article: a, sortKey: trendingScore(a) });
        }
      }
      const dedupedSeries = deduplicateSeriesByTranslation(allSeries(), locale);
      const childSeriesOf = /* @__PURE__ */ new Map();
      for (const s of dedupedSeries) {
        if (s.parent_id) {
          const arr = childSeriesOf.get(s.parent_id) || [];
          arr.push(s.id);
          childSeriesOf.set(s.parent_id, arr);
        }
      }
      for (const s of dedupedSeries) {
        if (s.parent_id) continue;
        const allMemberUris = [];
        const stack = [s.id];
        while (stack.length > 0) {
          const sid = stack.pop();
          const uris = seriesArticleMap().get(sid) || [];
          allMemberUris.push(...uris);
          for (const child of childSeriesOf.get(sid) || []) {
            stack.push(child);
          }
        }
        if (allMemberUris.length === 0) continue;
        const memberArts = allMemberUris.map((uri) => articles().find((a) => a.at_uri === uri)).filter((a) => !!a);
        const hasMatch = memberArts.some((a) => artUriSet.has(a.at_uri));
        if (!hasMatch) continue;
        const maxScore = memberArts.reduce((acc, a) => Math.max(acc, trendingScore(a)), 0);
        items.push({
          type: "series",
          series: s,
          articleCount: allMemberUris.length,
          sortKey: maxScore
        });
      }
      items.sort((a, b) => b.sortKey - a.sortKey);
      return items;
    }
    let filteredFeed = derived(() => {
      let candidateArticles;
      {
        candidateArticles = [...articles()].sort((a, b) => trendingScore(b) - trendingScore(a));
      }
      candidateArticles = deduplicateByTranslation(candidateArticles, locale);
      return buildFeed(candidateArticles);
    });
    let visibleTabs = derived(() => topCategories().filter((c) => interests.includes(c.id)));
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="home-header svelte-1eb4uvf"><h1 class="svelte-1eb4uvf">${escape_html(interests.length === 0 ? t("home.trending") : t("home.recent"))}</h1> <button class="edit-interests svelte-1eb4uvf"${attr("title", t("home.selectInterests"))}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"></path></svg></button></div> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      if (visibleTabs().length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="tab-bar svelte-1eb4uvf"><button${attr_class("tab svelte-1eb4uvf", void 0, { "active": activeTab === "all" })}>${escape_html(t("home.all"))}</button> <!--[-->`);
        const each_array_1 = ensure_array_like(visibleTabs());
        for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
          let cat = each_array_1[$$index_1];
          $$renderer2.push(`<button${attr_class("tab svelte-1eb4uvf", void 0, { "active": activeTab === cat.id })}>${escape_html(cat.name)}</button>`);
        }
        $$renderer2.push(`<!--]--></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (filteredFeed().length === 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="empty"><p>${escape_html(t("home.noArticles"))}</p> <p class="meta"><a href="/new">${escape_html(t("home.writeOne"))}</a></p></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<!--[-->`);
        const each_array_2 = ensure_array_like(filteredFeed());
        for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
          let item = each_array_2[$$index_2];
          if (item.type === "article" && item.article) {
            $$renderer2.push("<!--[0-->");
            PostCard($$renderer2, {
              article: item.article,
              articleTeaches: articleTeaches().get(item.article.at_uri) || [],
              articlePrereqs: articlePrereqs().get(item.article.at_uri) || [],
              variant: "home"
            });
          } else if (item.type === "series" && item.series) {
            $$renderer2.push("<!--[1-->");
            PostCard($$renderer2, {
              series: item.series,
              articleCount: item.articleCount,
              variant: "home"
            });
          } else {
            $$renderer2.push("<!--[-1-->");
          }
          $$renderer2.push(`<!--]-->`);
        }
        $$renderer2.push(`<!--]-->`);
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]-->`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
