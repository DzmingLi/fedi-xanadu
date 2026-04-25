// Canonical exam-prep tags. Stored verbatim in `books.exam_tags`. Tags
// are self-describing (`<exam>-<subject>`); pick a meaningful prefix when
// adding a new exam family. Human labels resolve through i18n via the
// `books.examTag.<code>` keys; an unrecognized code falls back to its
// raw string.

export const EXAM_TAGS = [
  'kaoyan-math-1',
  'kaoyan-math-2',
  'kaoyan-math-3',
  'kaoyan-math-agri',
  'kaoyan-408',
  'kaoyan-english-1',
  'kaoyan-english-2',
  'kaoyan-politics',
] as const;

export type ExamTag = typeof EXAM_TAGS[number];
