export interface Route {
  pattern: string;
  page: string;
  params?: string[];
}

export const routes: Route[] = [
  { pattern: '/', page: 'home' },
  { pattern: '/article', page: 'article' },
  { pattern: '/new', page: 'new' },
  { pattern: '/skills', page: 'skills' },
  { pattern: '/tag', page: 'tag' },
  { pattern: '/about', page: 'about' },
  { pattern: '/roadmap', page: 'roadmap' },
  { pattern: '/guide', page: 'guide' },
  { pattern: '/library', page: 'library' },
  { pattern: '/series', page: 'series' },
  { pattern: '/new-series', page: 'new-series' },
  { pattern: '/series-editor', page: 'series-editor' },
  { pattern: '/profile', page: 'profile' },
  { pattern: '/skill-tree/new', page: 'skill-tree-new' },
  { pattern: '/skill-tree', page: 'skill-tree' },
  { pattern: '/discussion', page: 'discussion' },
  { pattern: '/forks', page: 'forks' },
  { pattern: '/drafts', page: 'drafts' },
  { pattern: '/creator', page: 'creator' },
  { pattern: '/notifications', page: 'notifications' },
  { pattern: '/questions', page: 'questions' },
  { pattern: '/question', page: 'question' },
  { pattern: '/new-question', page: 'new-question' },
  { pattern: '/settings', page: 'settings' },
  { pattern: '/books', page: 'books' },
  { pattern: '/book-edition', page: 'book-edition' },
  { pattern: '/book', page: 'book' },
  { pattern: '/thoughts', page: 'thoughts' },
  { pattern: '/listings', page: 'listings' },
  { pattern: '/listing', page: 'listing-detail' },
  { pattern: '/new-listing', page: 'new-listing' },
  { pattern: '/events', page: 'events' },
  { pattern: '/event', page: 'event-detail' },
  { pattern: '/new-event', page: 'new-event' },
  { pattern: '/courses', page: 'courses' },
  { pattern: '/course', page: 'course-detail' },
  { pattern: '/new-course', page: 'new-course' },
  { pattern: '/author', page: 'author' },
  { pattern: '/guidelines', page: 'guidelines' },
  { pattern: '/feedback', page: 'feedback' },
  { pattern: '/admin', page: 'admin' },
];

export interface MatchResult {
  page: string;
  params: Record<string, string>;
}

export function matchRoute(url: string): MatchResult {
  let path: string;
  try {
    const u = new URL(url, 'http://localhost');
    path = u.pathname + u.search;
  } catch {
    path = url || '/';
  }

  const [base, query] = path.split('?');
  const params: Record<string, string> = {};

  if (query) {
    for (const part of query.split('&')) {
      const [k, v] = part.split('=');
      params[decodeURIComponent(k)] = decodeURIComponent(v || '');
    }
  }

  const normalizedBase = base === '' ? '/' : base;

  for (const route of routes) {
    if (route.pattern === normalizedBase) {
      return { page: route.page, params };
    }
  }

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

  return { page: 'home', params };
}

/** Navigate programmatically using the history API. */
export function navigate(path: string) {
  history.pushState(null, '', path);
  window.dispatchEvent(new PopStateEvent('popstate'));
}
