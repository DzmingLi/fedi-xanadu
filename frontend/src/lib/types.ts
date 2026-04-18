export type ContentFormat = 'typst' | 'markdown' | 'html';
export type ContentKind = 'article' | 'question' | 'answer' | 'thought';
export type Category = string;
export type PrereqType = 'required' | 'recommended' | 'suggested';

export interface Tag {
  id: string;
  name: string;
  names: Record<string, string>;
  description: string | null;
  created_by: string;
  created_at: string;
}

export interface Article {
  at_uri: string;
  did: string;
  author_handle: string | null;
  author_display_name: string | null;
  author_avatar: string | null;
  author_reputation: number;
  kind: ContentKind;
  title: string;
  summary: string;
  summary_html: string;
  cover_url: string | null;
  paper_venue: string | null;
  paper_year: number | null;
  paper_accepted: boolean | null;
  content_hash: string | null;
  content_format: ContentFormat;
  lang: string;
  translation_group: string | null;
  license: string;
  prereq_threshold: number;
  category: Category;
  restricted: boolean;
  question_uri: string | null;
  book_id: string | null;
  edition_id: string | null;
  answer_count: number;
  vote_score: number;
  bookmark_count: number;
  comment_count: number;
  fork_count: number;
  created_at: string;
  updated_at: string;
}

export interface ArticleContent {
  source: string;
  html: string;
}

export interface ArticlePrereqRow {
  tag_id: string;
  prereq_type: PrereqType;
  tag_name: string;
  tag_names: Record<string, string>;
}

export interface ForkWithTitle {
  fork_uri: string;
  forked_uri: string;
  vote_score: number;
  title: string;
  did: string;
  author_handle: string | null;
}

export interface UserSkill {
  did: string;
  tag_id: string;
  status: 'mastered' | 'learning';
  lit_at: string;
}

export interface GraphNode {
  id: string;
  name: string;
  names: Record<string, string>;
  lit: boolean;
}

export interface GraphEdge {
  from: string;
  to: string;
  type: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface TagTreeEntry {
  parent_tag: string;
  child_tag: string;
}

export interface ContentTeachRow {
  content_uri: string;
  tag_id: string;
  tag_name: string;
  tag_names: Record<string, string>;
}


export interface ContentPrereqBulkRow {
  content_uri: string;
  tag_id: string;
  prereq_type: PrereqType;
  tag_name: string;
  tag_names: Record<string, string>;
}

export interface BookmarkWithTitle {
  article_uri: string;
  folder_path: string;
  created_at: string;
  title: string;
  summary: string;
}

export interface AuthUser {
  token: string;
  did: string;
  handle: string;
  display_name: string | null;
  avatar: string | null;
}

export interface VoteSummary {
  target_uri: string;
  score: number;
  upvotes: number;
  downvotes: number;
}

export interface Series {
  id: string;
  title: string;
  summary: string | null;
  summary_html: string;
  long_description: string | null;
  order_index: number;
  created_by: string;
  author_handle?: string | null;
  author_display_name?: string | null;
  author_avatar?: string | null;
  created_at: string;
  lang: string;
  translation_group: string | null;
  category: Category;
  vote_score: number;
  bookmark_count: number;
  cover_url?: string | null;
}

export interface Book {
  id: string;
  title: L;
  subtitle?: L;
  abbreviation: string | null;
  authors: string[];
  description: L;
  cover_url: string | null;
  default_edition_id: string | null;
  created_by: string;
  created_at: string;
  avg_rating?: number;
  rating_count?: number;
  reader_count?: number;
  tags?: string[];
}

export interface PurchaseLink {
  label: string;
  url: string;
}

export interface BookEdition {
  id: string;
  book_id: string;
  edition_name: string | null;
  title: string;
  subtitle: string | null;
  lang: string;
  isbn: string | null;
  publisher: string | null;
  year: string | null;
  translators: string[];
  purchase_links: PurchaseLink[];
  cover_url: string | null;
  created_at: string;
}

export interface BookRatingStats {
  avg_rating: number;
  rating_count: number;
}

export interface ReadingStatus {
  book_id: string;
  user_did: string;
  status: 'want_to_read' | 'reading' | 'finished' | 'dropped';
  progress: number;
  updated_at: string;
}

export interface BookChapter {
  id: string;
  book_id: string;
  parent_id: string | null;
  title: string;
  title_i18n: Record<string, string>;
  order_index: number;
  article_uri: string | null;
  teaches: string[];
  prereqs: ChapterPrereqEntry[];
}

export interface ChapterPrereqEntry {
  tag_id: string;
  prereq_type: 'required' | 'recommended';
}

export interface ChapterProgress {
  book_id: string;
  chapter_id: string;
  user_did: string;
  completed: boolean;
  completed_at: string | null;
}

export interface LinkedAuthor {
  id: string;
  name: string;
  did: string | null;
  orcid: string | null;
  affiliation: string | null;
  homepage: string | null;
}

export interface BookDetail {
  book: Book;
  linked_authors: LinkedAuthor[];
  editions: BookEdition[];
  chapters: BookChapter[];
  reviews: Article[];
  review_count: number;
  rating: BookRatingStats;
  my_rating: number | null;
  my_reading_status: ReadingStatus | null;
  my_chapter_progress: ChapterProgress[];
  tags: string[];
  prereqs: string[];
  topics: string[];
}

export interface SeriesArticle {
  series_id: string;
  article_uri: string;
  title: string;
  summary: string;
  lang: string;
  order_index: number;
}

export interface SeriesArticlePrereq {
  article_uri: string;
  prereq_article_uri: string;
}

export interface SeriesDetail {
  series: Series;
  articles: SeriesArticle[];
  prereqs: SeriesArticlePrereq[];
  translations: Series[];
}

export interface SeriesTreeNode {
  series: Series;
  articles: SeriesArticle[];
}

export interface SeriesHeading {
  id: number;
  series_id: string;
  level: number;
  title: string;
  anchor: string;
  article_uri: string | null;
  parent_heading_id: number | null;
  order_index: number;
}

export interface SkillTree {
  at_uri: string;
  did: string;
  author_handle?: string;
  title: string;
  description: string | null;
  tag_id: string | null;
  tag_name?: string | null;
  tag_names?: Record<string, string> | null;
  forked_from: string | null;
  created_at: string;
  score?: number;
  edge_count?: number;
  adopt_count?: number;
}

export interface SkillTreeEdge {
  parent_tag: string;
  child_tag: string;
}

export interface SkillTreePrereq {
  from_tag: string;
  to_tag: string;
  prereq_type: 'required' | 'recommended';
}

export interface SkillTreeDetail {
  tree: SkillTree;
  edges: SkillTreeEdge[];
  prereqs: SkillTreePrereq[];
  tag_names_map: Record<string, string>;
  tag_names_i18n: Record<string, Record<string, string>>;
}

export interface UserTagPrereq {
  from_tag: string;
  to_tag: string;
  prereq_type: 'required' | 'recommended';
}

export interface FrontierSkill {
  tag_id: string;
  tag_name: string;
  tag_names: Record<string, string>;
  article_count: number;
}

export interface SeriesContextItem {
  series_id: string;
  series_title: string;
  total: number;
  prev: { article_uri: string; title: string }[];
  next: { article_uri: string; title: string }[];
}

export interface ForkSourceInfo {
  source_uri: string;
  title: string;
  license: string;
  lang: string;
  did: string;
  author_handle: string | null;
  author_display_name: string | null;
  author_avatar: string | null;
}

export interface ArticleFullResponse {
  article: Article;
  content: ArticleContent;
  prereqs: ArticlePrereqRow[];
  forks: ForkWithTitle[];
  fork_source: ForkSourceInfo | null;
  votes: VoteSummary;
  series_context: SeriesContextItem[];
  translations: Article[];
  my_vote: number;
  is_bookmarked: boolean;
  learned: boolean;
  access_denied: boolean;
}

export interface AccessGrant {
  article_uri: string;
  grantee_did: string;
  granted_at: string;
}

/** Fixed contact fields shown on the profile card, plus free-form extras.
 * Undefined/empty fields should be stripped before sending to the server.
 *
 * `tangled` and `bilibili` store `{url, username}` since neither platform
 * maps usernames to a canonical URL pattern — the URL is what we link to,
 * the username is what we display. */
export interface Contacts {
  website?: string | null;
  email?: string | null;
  telegram?: string | null;
  matrix?: string | null;
  github?: string | null;
  codeberg?: string | null;
  tangled?: LinkedHandle | null;
  youtube?: string | null;
  bilibili?: LinkedHandle | null;
  custom?: CustomLink[];
}

export interface LinkedHandle {
  url: string;
  username: string;
}

export interface CustomLink {
  label: string;
  url: string;
}

export const CONTACT_KINDS = [
  'website', 'email', 'telegram', 'matrix',
  'github', 'codeberg', 'tangled',
  'youtube', 'bilibili',
] as const;
export type ContactKind = typeof CONTACT_KINDS[number];

export interface EducationTranslation {
  school?: string | null;
  department?: string | null;
  major?: string | null;
}

export interface EducationEntry {
  degree: string;
  school: string;
  department?: string | null;
  major?: string | null;
  start_date?: string | null;
  end_date?: string | null;
  current?: boolean;
  translations?: Record<string, EducationTranslation>;
}

export interface WorkExperienceEntry {
  company: string;
  department?: string | null;
  title?: string | null;
  location?: string | null;
  start_date?: string | null;
  end_date?: string | null;
  current?: boolean;
  description?: string | null;
  translations?: Record<string, WorkExperienceTranslation>;
}

export interface WorkExperienceTranslation {
  company?: string | null;
  department?: string | null;
  title?: string | null;
  location?: string | null;
  description?: string | null;
}

/** Locale -> text map, e.g. {"en": "NightBoat", "zh": "夜舟"} */
export type L = Record<string, string>;

export interface PublicationEntry {
  title: L;
  authors: string[];
  venue: L;
  year: number;
  url: string | null;
  doi: string | null;
  abstract_text: L | null;
}

export interface ProjectEntry {
  title: L;
  description: L;
  url: string | null;
  status: 'active' | 'completed' | 'archived';
}

export interface TeachingEntry {
  course_name: L;
  role: L;
  institution: L;
  year: number;
  description: L | null;
}

export interface ProfileData {
  did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  banner_url: string | null;
  bio: string;
  reputation: number;
  article_count: number;
  series_count: number;
  contacts: Contacts;
  email: string | null;
  education: EducationEntry[];
  experience: WorkExperienceEntry[];
  publications: PublicationEntry[];
  projects: ProjectEntry[];
  teaching: TeachingEntry[];
  affiliation: string | null;
  credentials_verified: boolean;
}

export interface UserSettings {
  native_lang: string;
  known_langs: string[];
  prefer_native: boolean;
  hide_unknown: boolean;
  default_format: ContentFormat;
  email: string | null;
  bookmarks_public: boolean;
  public_folders: string[];
}

export interface Comment {
  id: string;
  content_uri: string;
  did: string;
  author_handle: string | null;
  parent_id: string | null;
  body: string;
  quote_text: string | null;
  section_ref: string | null;
  vote_score: number;
  created_at: string;
  updated_at: string;
}

export interface CommentVoteResult {
  comment_id: string;
  score: number;
  my_vote: number;
}

export interface MyCommentVote {
  comment_id: string;
  value: number;
}

export interface Draft {
  id: string;
  did: string;
  title: string;
  summary: string;
  content: string;
  content_format: ContentFormat;
  lang: string;
  license: string;
  tags: string;
  prereqs: string;
  at_uri: string | null;
  created_at: string;
  updated_at: string;
}

export interface Notification {
  id: string;
  recipient_did: string;
  actor_did: string;
  actor_handle: string | null;
  kind: 'comment_reply' | 'article_comment' | 'new_follower' | 'article_fork' | 'new_answer';
  target_uri: string | null;
  target_title: string | null;
  context_id: string | null;
  read: boolean;
  created_at: string;
}

export interface QuestionDetail {
  question: Article;
  answers: Article[];
}

export interface BlockedUser {
  blocked_did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  created_at: string;
}

export interface Report {
  id: string;
  reporter_did: string;
  target_did: string;
  target_uri: string | null;
  kind: string;
  reason: string;
  status: string;
  admin_note: string | null;
  created_at: string;
  resolved_at: string | null;
}

export interface CreateArticle {
  title: string;
  summary?: string;
  content: string;
  content_format: ContentFormat;
  lang?: string;
  license?: string;
  translation_of?: string;
  restricted?: boolean;
  category?: Category;
  book_id?: string;
  edition_id?: string;
  tags: string[];
  prereqs: { tag_id: string; prereq_type: PrereqType }[];
}

// --- Version History ---

export interface ArticleVersion {
  id: number;
  article_uri: string;
  change_hash: string;
  editor_did: string;
  message: string;
  created_at: string;
}

export interface ArticleVersionInfo extends ArticleVersion {
  unrecordable: boolean;
}

export interface ArticleVersionFull extends ArticleVersion {
  source_text: string;
}

export interface VersionDiff {
  from_version: number;
  to_version: number;
  hunks: DiffHunk[];
}

export interface DiffHunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  lines: DiffLine[];
}

export interface DiffLine {
  kind: 'context' | 'add' | 'remove';
  content: string;
}

// Listings (academic recruitment)
export type ListingKind = 'phd' | 'masters' | 'ra' | 'postdoc' | 'intern' | 'faculty' | 'other';

export interface Listing {
  id: string;
  did: string;
  author_handle: string | null;
  author_reputation: number;
  title: string;
  description: string;
  kind: ListingKind;
  institution: string;
  department: string | null;
  location: string | null;
  contact_email: string | null;
  contact_url: string | null;
  compensation: string | null;
  deadline: string | null;
  is_open: boolean;
  required_tags: string[];
  preferred_tags: string[];
  created_at: string;
  updated_at: string;
}

// Courses
export interface Course {
  id: string;
  did: string;
  title: string;
  code: string | null;
  description: string;
  institution: string | null;
  department: string | null;
  semester: string | null;
  lang: string;
  license: string;
  source_url: string | null;
  source_attribution: string | null;
  is_published: boolean;
  created_at: string;
  updated_at: string;
}

export interface CourseListItem {
  id: string;
  did: string;
  author_handle: string | null;
  title: string;
  code: string | null;
  description: string;
  institution: string | null;
  semester: string | null;
  lang: string;
  is_published: boolean;
  series_count: number;
  staff_count: number;
  session_count: number;
  avg_rating: number;
  rating_count: number;
  created_at: string;
}

export interface CourseSeries {
  series_id: string;
  title: string;
  summary: string | null;
  role: string;
  sort_order: number;
}

export interface CourseStaff {
  user_did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  role: string;
  sort_order: number;
}

export interface CourseSkillTree {
  tree_uri: string;
  title: string;
  role: string;
}

export interface CoursePrereq {
  prereq_course_id: string;
  title: string;
  code: string | null;
  institution: string | null;
}

export interface SessionResource {
  type: string;  // "video" | "notes" | "hw" | "discussion"
  url: string;
  label: string;
}

export interface CourseSession {
  id: string;
  course_id: string;
  sort_order: number;
  topic?: string | null;
  date?: string | null;
  readings?: string | null;
  resources: SessionResource[];
  tags: CourseTag[];
  prereqs: CourseTag[];
}

export interface CourseTextbook {
  book_id: string;
  title: L;
  authors: string[];
  cover_url: string | null;
  role: string;
  sort_order: number;
}

export interface CourseTag {
  tag_id: string;
  tag_name: string;
}

export interface CourseRatingStats {
  avg_rating: number;
  rating_count: number;
}

export interface CourseReview {
  at_uri: string;
  title: string;
  summary: string;
  did: string;
  author_handle: string | null;
  author_display_name: string | null;
  created_at: string;
  vote_score: number;
  comment_count: number;
}

export interface CourseDetail {
  course: Course;
  syllabus: string;
  sessions: CourseSession[];
  textbooks: CourseTextbook[];
  tags: CourseTag[];
  series: CourseSeries[];
  staff: CourseStaff[];
  skill_trees: CourseSkillTree[];
  prerequisites: CoursePrereq[];
  rating: CourseRatingStats;
  reviews: CourseReview[];
}

// ---- Publications (专栏) ----

export interface Publication {
  id: string;
  title_i18n: L;
  description_i18n: L;
  cover_url: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
  at_uri: string | null;
}

export interface PublicationSummary {
  id: string;
  title_i18n: L;
  description_i18n: L;
  cover_url: string | null;
  created_by: string;
  created_at: string;
  member_count: number;
  content_count: number;
  follower_count: number;
}

export interface PublicationMember {
  publication_id: string;
  did: string;
  role: 'owner' | 'editor' | 'writer';
  added_at: string;
  added_by: string;
  membership_at_uri: string | null;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
}

/** Mixed feed entry — either an article or a series. Exactly one of
 * `article` / `series` is non-null. */
export interface PublicationContentItem {
  kind: 'article' | 'series';
  added_at: string;
  article: Article | null;
  series: SeriesListRow | null;
}

export interface SeriesListRow {
  id: string;
  title: string;
  summary: string | null;
  summary_html: string;
  long_description: string | null;
  order_index: number;
  created_by: string;
  author_handle: string | null;
  author_display_name: string | null;
  author_avatar: string | null;
  created_at: string;
  lang: string;
  translation_group: string | null;
  category: string;
  split_level: number;
  is_published: boolean;
  vote_score: number;
  bookmark_count: number;
  cover_url?: string | null;
}

export interface PublicationViewerState {
  role: 'owner' | 'editor' | 'writer' | null;
  is_following: boolean;
  membership_confirmed: boolean;
}
