import type { Article, Series } from './types';
import { getLocale } from './i18n/index.svelte';
import { getLangPrefs } from './langPrefs.svelte';
import { getBlockedDids } from './blocklist.svelte';

export function authorName(a: Pick<Article, 'author_handle' | 'did'>): string {
  if (a.author_handle) return `@${a.author_handle}`;
  return a.did.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
}

/**
 * Pick the best variant from a translation group.
 * If user has langPrefs and prefer_native is true, prefer native_lang.
 * Otherwise prefer the original (first by date).
 */
function pickBest<T extends { lang: string }>(variants: T[], locale: string): T {
  const prefs = getLangPrefs();
  if (prefs?.prefer_native) {
    const native = variants.find(v => v.lang === prefs.native_lang);
    if (native) return native;
  }
  return variants.find(v => v.lang === locale)
    || variants.find(v => v.lang === 'zh')
    || variants[0];
}

/**
 * If hide_unknown is set, filter out articles in languages the user doesn't know.
 */
function filterByKnownLangs<T extends { lang: string }>(items: T[]): T[] {
  const prefs = getLangPrefs();
  if (!prefs?.hide_unknown) return items;
  const known = new Set(prefs.known_langs);
  return items.filter(i => known.has(i.lang));
}

/**
 * Filter out content from blocked users.
 */
function filterBlocked<T extends { did?: string; created_by?: string }>(items: T[]): T[] {
  const blocked = getBlockedDids();
  if (blocked.size === 0) return items;
  return items.filter(i => {
    const d = (i as any).did || (i as any).created_by;
    return !d || !blocked.has(d);
  });
}

/**
 * Deduplicate articles by translation_group, preferring the version
 * matching the given locale or user preference.
 */
export function deduplicateByTranslation(articles: Article[], locale: string): Article[] {
  const filtered = filterByKnownLangs(filterBlocked(articles));
  const groups = new Map<string, Article[]>();

  for (const a of filtered) {
    // null translation_group means original — use own URI as group key
    const key = a.translation_group || a.at_uri;
    const arr = groups.get(key) || [];
    arr.push(a);
    groups.set(key, arr);
  }

  return [...groups.values()].map(variants => pickBest(variants, locale));
}

/**
 * Deduplicate series by translation_group, preferring the locale match.
 */
export function deduplicateSeriesByTranslation(series: Series[], locale: string): Series[] {
  const filtered = filterByKnownLangs(filterBlocked(series));
  const groups = new Map<string, Series[]>();

  for (const s of filtered) {
    // null translation_group means original — use own ID as group key
    const key = s.translation_group || s.id;
    const arr = groups.get(key) || [];
    arr.push(s);
    groups.set(key, arr);
  }

  return [...groups.values()].map(variants => pickBest(variants, locale));
}

export function tagName(
  names: Record<string, string> | null | undefined,
  name: string,
  id: string,
): string {
  if (names) {
    const l = getLocale();
    return names[l] || names['en'] || name || id;
  }
  return name || id;
}
