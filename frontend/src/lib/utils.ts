/**
 * Build the canonical URL for a content record. Questions and answers go to
 * the `/question` aggregation route (question body + answers thread); answers
 * anchor to themselves inside the thread. Everything else (articles, thoughts)
 * goes to `/article`.
 */
export function contentHref(
  atUri: string,
  kind?: string | null,
  questionUri?: string | null,
): string {
  if (kind === 'question') {
    return `/question?uri=${encodeURIComponent(atUri)}`;
  }
  if (kind === 'answer' && questionUri) {
    return `/question?uri=${encodeURIComponent(questionUri)}#${encodeURIComponent(atUri)}`;
  }
  return `/article?uri=${encodeURIComponent(atUri)}`;
}

/** Format a datetime string as relative time (e.g. "3 小时前") or absolute date. */
export function timeAgo(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  if (diff < 60_000) return '刚刚';
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  if (diff < 7 * 86400_000) return `${Math.floor(diff / 86400_000)} 天前`;
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
}
