import type { Article } from './types';
import { getLocale } from './i18n';

export function authorName(a: Pick<Article, 'author_handle' | 'did'>): string {
  if (a.author_handle) return `@${a.author_handle}`;
  return a.did.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
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
