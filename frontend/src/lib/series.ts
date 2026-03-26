/**
 * Shared series article resolution logic.
 * Used by Home, Profile, and Library to build series membership maps.
 */

export interface SeriesArticleRow {
  series_id: string;
  article_uri: string;
}

export interface SeriesArticleMaps {
  /** All article URIs that belong to any series */
  seriesArticleUris: Set<string>;
  /** series_id -> article_uri[] */
  seriesArticleMap: Map<string, string[]>;
}

export function buildSeriesArticleMaps(rows: SeriesArticleRow[]): SeriesArticleMaps {
  const uriSet = new Set<string>();
  const saMap = new Map<string, string[]>();
  for (const sa of rows) {
    uriSet.add(sa.article_uri);
    const arr = saMap.get(sa.series_id) || [];
    arr.push(sa.article_uri);
    saMap.set(sa.series_id, arr);
  }
  return { seriesArticleUris: uriSet, seriesArticleMap: saMap };
}

/**
 * Build a map of article_uri -> tag rows from a flat list.
 */
export function buildArticleTagMap<T extends { article_uri: string }>(rows: T[]): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const t of rows) {
    const arr = map.get(t.article_uri) || [];
    arr.push(t);
    map.set(t.article_uri, arr);
  }
  return map;
}
