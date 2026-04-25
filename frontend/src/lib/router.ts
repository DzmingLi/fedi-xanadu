export interface Route {
  pattern: string;
  page: string;
  params?: string[];
}

export const routes: Route[] = [
  { pattern: '/', page: 'home' },
  { pattern: '/search', page: 'search' },
  { pattern: '/hierarchy', page: 'hierarchy' },
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
  { pattern: '/book-series', page: 'book-series-list' },
  { pattern: '/book-series-detail', page: 'book-series-detail' },
  { pattern: '/thoughts', page: 'thoughts' },
  { pattern: '/listings', page: 'listings' },
  { pattern: '/listing', page: 'listing-detail' },
  { pattern: '/new-listing', page: 'new-listing' },
  { pattern: '/courses', page: 'courses' },
  { pattern: '/course-reviews', page: 'course-reviews' },
  { pattern: '/course-notes', page: 'course-notes' },
  { pattern: '/course-discussions', page: 'course-discussions' },
  { pattern: '/course', page: 'course-detail' },
  { pattern: '/new-course', page: 'new-course' },
  { pattern: '/author', page: 'author' },
  { pattern: '/publications', page: 'publications' },
  { pattern: '/publication', page: 'publication-detail' },
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

  // /@handle/slug[.lang] — canonical article URL. The last segment may
  // carry an explicit locale suffix (e.g. `quantum.zh-CN`); if absent the
  // server redirects to the Accept-Language-negotiated variant before the
  // SPA sees it, but we still accept bare slugs here to stay robust.
  const atMatch = normalizedBase.match(/^\/@([^/]+)\/([^/]+)$/);
  if (atMatch) {
    const handle = decodeURIComponent(atMatch[1]);
    const rest = decodeURIComponent(atMatch[2]);
    const { slug, lang } = splitSlugLang(rest);
    return {
      page: 'article',
      params: { ...params, handle, slug, lang: lang ?? '' },
    };
  }

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

/**
 * Split `rest` into `{slug, lang}`. Matches the server's `split_slug_lang`:
 * only split on the last `.` if the trailing token looks like a BCP 47 tag
 * (letters + optional hyphens, ≥ 2 chars). This keeps file-like slugs such
 * as `foo.md` from being parsed as `slug=foo, lang=md`.
 */
function splitSlugLang(rest: string): { slug: string; lang: string | null } {
  const idx = rest.lastIndexOf('.');
  if (idx <= 0 || idx === rest.length - 1) return { slug: rest, lang: null };
  const tail = rest.slice(idx + 1);
  if (!/^[A-Za-z]{2,3}(?:-[A-Za-z0-9]+)*$/.test(tail)) return { slug: rest, lang: null };
  return { slug: rest.slice(0, idx), lang: tail };
}

/** Build the canonical article URL. Bare (no lang) triggers server-side
 *  Accept-Language negotiation; explicit lang is shareable + cacheable. */
export function articleUrl(handle: string, slug: string, lang?: string | null): string {
  const safeHandle = encodeURIComponent(handle);
  const safeSlug = encodeURIComponent(slug);
  return lang
    ? `/@${safeHandle}/${safeSlug}.${encodeURIComponent(lang)}`
    : `/@${safeHandle}/${safeSlug}`;
}

/** Navigate programmatically using the history API. */
export function navigate(path: string) {
  history.pushState(null, '', path);
  window.dispatchEvent(new PopStateEvent('popstate'));
}
