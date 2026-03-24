import type { Article } from './types';

export function authorName(a: Pick<Article, 'author_handle' | 'did'>): string {
  if (a.author_handle) return `@${a.author_handle}`;
  return a.did.replace('did:plc:', '').replace('did:web:', '').slice(0, 16);
}
