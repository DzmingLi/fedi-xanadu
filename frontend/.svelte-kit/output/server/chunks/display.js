import { g as getLocale } from "./index.svelte.js";
import "clsx";
import "./auth.svelte.js";
let blocked = /* @__PURE__ */ new Set();
function getBlockedDids() {
  return blocked;
}
function isBlocked(did) {
  return blocked.has(did);
}
function authorName(a) {
  if (a.author_handle) return `@${a.author_handle}`;
  return a.did.replace("did:plc:", "").replace("did:web:", "").slice(0, 16);
}
function pickBest(variants, locale) {
  return variants.find((v) => v.lang === locale) || variants.find((v) => v.lang === "zh") || variants[0];
}
function filterByKnownLangs(items) {
  return items;
}
function filterBlocked(items) {
  const blocked2 = getBlockedDids();
  if (blocked2.size === 0) return items;
  return items.filter((i) => {
    const d = i.did || i.created_by;
    return !d || !blocked2.has(d);
  });
}
function deduplicateByTranslation(articles, locale) {
  const filtered = filterByKnownLangs(filterBlocked(articles));
  const groups = /* @__PURE__ */ new Map();
  for (const a of filtered) {
    const key = a.translation_group || a.at_uri;
    const arr = groups.get(key) || [];
    arr.push(a);
    groups.set(key, arr);
  }
  return [...groups.values()].map((variants) => pickBest(variants, locale));
}
function deduplicateSeriesByTranslation(series, locale) {
  const filtered = filterByKnownLangs(filterBlocked(series));
  const groups = /* @__PURE__ */ new Map();
  for (const s of filtered) {
    const key = s.translation_group || s.id;
    const arr = groups.get(key) || [];
    arr.push(s);
    groups.set(key, arr);
  }
  return [...groups.values()].map((variants) => pickBest(variants, locale));
}
function tagName(names, name, id) {
  if (names) {
    const l = getLocale();
    return names[l] || names["en"] || name || id;
  }
  return name || id;
}
export {
  authorName as a,
  deduplicateSeriesByTranslation as b,
  deduplicateByTranslation as d,
  isBlocked as i,
  tagName as t
};
