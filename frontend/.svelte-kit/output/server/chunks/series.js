function buildSeriesArticleMaps(rows) {
  const uriSet = /* @__PURE__ */ new Set();
  const saMap = /* @__PURE__ */ new Map();
  for (const sa of rows) {
    uriSet.add(sa.article_uri);
    const arr = saMap.get(sa.series_id) || [];
    arr.push(sa.article_uri);
    saMap.set(sa.series_id, arr);
  }
  return { seriesArticleUris: uriSet, seriesArticleMap: saMap };
}
function buildArticleRowMap(rows) {
  const map = /* @__PURE__ */ new Map();
  for (const t of rows) {
    const arr = map.get(t.content_uri) || [];
    arr.push(t);
    map.set(t.content_uri, arr);
  }
  return map;
}
export {
  buildSeriesArticleMaps as a,
  buildArticleRowMap as b
};
