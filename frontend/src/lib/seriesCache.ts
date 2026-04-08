import type { SeriesDetail } from './types';

interface CacheEntry {
  detail: SeriesDetail;
  ts: number;
}

const cache = new Map<string, CacheEntry>();
const TTL = 5 * 60 * 1000; // 5 minutes

export function getCachedSeries(id: string): CacheEntry | null {
  const entry = cache.get(id);
  if (entry && Date.now() - entry.ts < TTL) return entry;
  if (entry) cache.delete(id);
  return null;
}

export function setCachedSeries(id: string, detail: SeriesDetail) {
  cache.set(id, { detail, ts: Date.now() });
}
