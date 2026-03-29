export interface Route {
  pattern: string;
  page: string;
  params?: string[];
}

export const routes: Route[] = [
  { pattern: '/', page: 'home' },
  { pattern: '/tags', page: 'skills' },
  { pattern: '/article', page: 'article' },
  { pattern: '/new', page: 'new' },
  { pattern: '/skills', page: 'skills' },
  { pattern: '/graph', page: 'skills' },
  { pattern: '/tag', page: 'tag' },
  { pattern: '/about', page: 'about' },
  { pattern: '/roadmap', page: 'roadmap' },
  { pattern: '/guide', page: 'guide' },
  { pattern: '/library', page: 'library' },
  { pattern: '/series', page: 'series' },
  { pattern: '/new-series', page: 'new-series' },
  { pattern: '/profile', page: 'profile' },
  { pattern: '/skill-trees', page: 'skills' },
  { pattern: '/skill-tree/new', page: 'skill-tree-new' },
  { pattern: '/skill-tree', page: 'skill-tree' },
  { pattern: '/forks', page: 'forks' },
  { pattern: '/drafts', page: 'drafts' },
  { pattern: '/notifications', page: 'notifications' },
  { pattern: '/questions', page: 'questions' },
  { pattern: '/question', page: 'question' },
  { pattern: '/new-question', page: 'new-question' },
  { pattern: '/settings', page: 'settings' },
  { pattern: '/books', page: 'books' },
  { pattern: '/book-edition', page: 'book-edition' },
  { pattern: '/book', page: 'book' },
];

export interface MatchResult {
  page: string;
  params: Record<string, string>;
}

export function matchRoute(hash: string): MatchResult {
  const path = hash.slice(1) || '/'; // remove '#'
  const [base, query] = path.split('?');
  const params: Record<string, string> = {};

  if (query) {
    for (const part of query.split('&')) {
      const [k, v] = part.split('=');
      params[decodeURIComponent(k)] = decodeURIComponent(v || '');
    }
  }

  const normalizedBase = base === '' ? '/' : base;

  // Try exact match first (longest patterns first for specificity)
  for (const route of routes) {
    if (route.pattern === normalizedBase) {
      return { page: route.page, params };
    }
  }

  // Try param-based pattern match (e.g. /tags/:id)
  for (const route of routes) {
    if (!route.params || route.params.length === 0) continue;
    const patternParts = route.pattern.split('/');
    const baseParts = normalizedBase.split('/');
    if (patternParts.length !== baseParts.length) continue;

    let matched = true;
    for (let i = 0; i < patternParts.length; i++) {
      if (patternParts[i].startsWith(':')) {
        const paramName = patternParts[i].slice(1);
        params[paramName] = decodeURIComponent(baseParts[i]);
      } else if (patternParts[i] !== baseParts[i]) {
        matched = false;
        break;
      }
    }
    if (matched) {
      return { page: route.page, params };
    }
  }

  // Fallback
  return { page: 'home', params };
}
