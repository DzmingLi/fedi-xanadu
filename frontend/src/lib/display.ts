import type { Article, Series } from './types';
import { getLocale } from './i18n';

export function authorName(a: Pick<Article, 'author_handle' | 'did'>): string {
  if (a.author_handle) return `@${a.author_handle}`;
  return a.did.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
}

/**
 * Deduplicate articles by translation_group, preferring the version
 * matching the given locale. Falls back to any available version.
 */
export function deduplicateByTranslation(articles: Article[], locale: string): Article[] {
  const groups = new Map<string, Article[]>();
  const ungrouped: Article[] = [];

  for (const a of articles) {
    const key = a.translation_group;
    if (key) {
      const arr = groups.get(key) || [];
      arr.push(a);
      groups.set(key, arr);
    } else {
      ungrouped.push(a);
    }
  }

  const result: Article[] = [...ungrouped];
  for (const [, variants] of groups) {
    const match = variants.find(a => a.lang === locale)
      || variants.find(a => a.lang === 'zh')
      || variants[0];
    result.push(match);
  }

  return result;
}

/**
 * Deduplicate series by translation_group, preferring the locale match.
 */
export function deduplicateSeriesByTranslation(series: Series[], locale: string): Series[] {
  const groups = new Map<string, Series[]>();
  const ungrouped: Series[] = [];

  for (const s of series) {
    const key = s.translation_group;
    if (key) {
      const arr = groups.get(key) || [];
      arr.push(s);
      groups.set(key, arr);
    } else {
      ungrouped.push(s);
    }
  }

  const result: Series[] = [...ungrouped];
  for (const [, variants] of groups) {
    const match = variants.find(s => s.lang === locale)
      || variants.find(s => s.lang === 'zh')
      || variants[0];
    result.push(match);
  }

  return result;
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
