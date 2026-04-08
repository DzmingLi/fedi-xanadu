// Client-side cache for stable, rarely-changing API responses
// (graph, tag-tree). TTL = 5 minutes.

const TTL_MS = 5 * 60 * 1000;

interface CacheEntry<T> {
  data: T;
  at: number;
}

const store = new Map<string, CacheEntry<unknown>>();

export function cacheGet<T>(key: string): T | null {
  const entry = store.get(key) as CacheEntry<T> | undefined;
  if (!entry) return null;
  if (Date.now() - entry.at > TTL_MS) { store.delete(key); return null; }
  return entry.data;
}

export function cacheSet<T>(key: string, data: T): void {
  store.set(key, { data, at: Date.now() });
}

export function cacheInvalidate(key: string): void {
  store.delete(key);
}
