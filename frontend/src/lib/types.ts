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
  kind: string;
  title: string;
  description: string;
  content_hash: string | null;
  content_format: string;
  lang: string;
  translation_group: string | null;
  license: string;
  prereq_threshold: number;
  category: string;
  restricted: boolean;
  question_uri: string | null;
  book_id: string | null;
  answer_count: number;
  vote_score: number;
  bookmark_count: number;
  created_at: string;
  updated_at: string;
}

export interface ArticleContent {
  source: string;
  html: string;
}

export interface ArticlePrereqRow {
  tag_id: string;
  prereq_type: string;
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
  prereq_type: string;
  tag_name: string;
  tag_names: Record<string, string>;
}

export interface BookmarkWithTitle {
  article_uri: string;
  folder_path: string;
  created_at: string;
  title: string;
  description: string;
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
  description: string | null;
  parent_id: string | null;
  order_index: number;
  created_by: string;
  author_handle?: string | null;
  created_at: string;
  lang: string;
  translation_group: string | null;
  category: string;
}

export interface Book {
  id: string;
  title: string;
  authors: string[];
  description: string;
  cover_url: string | null;
  created_by: string;
  created_at: string;
}

export interface PurchaseLink {
  label: string;
  url: string;
}

export interface BookEdition {
  id: string;
  book_id: string;
  title: string;
  lang: string;
  isbn: string | null;
  publisher: string | null;
  year: string | null;
  translators: string[];
  purchase_links: PurchaseLink[];
  created_at: string;
}

export interface BookRatingStats {
  avg_rating: number;
  rating_count: number;
}

export interface ReadingStatus {
  book_id: string;
  user_did: string;
  status: 'want_to_read' | 'reading' | 'finished';
  progress: number;
  updated_at: string;
}

export interface BookDetail {
  book: Book;
  editions: BookEdition[];
  reviews: Article[];
  review_count: number;
  rating: BookRatingStats;
  my_rating: number | null;
  my_reading_status: ReadingStatus | null;
}

export interface SeriesArticle {
  series_id: string;
  article_uri: string;
  title: string;
  description: string;
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
  children: Series[];
  translations: Series[];
}

export interface SeriesTreeNode {
  series: Series;
  articles: SeriesArticle[];
  children: SeriesTreeNode[];
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

export interface SkillTreeDetail {
  tree: SkillTree;
  edges: SkillTreeEdge[];
  tag_names_map: Record<string, string>;
  tag_names_i18n: Record<string, Record<string, string>>;
}

export interface SeriesContextItem {
  series_id: string;
  series_title: string;
  total: number;
  prev: { article_uri: string; title: string }[];
  next: { article_uri: string; title: string }[];
}

export interface ArticleFullResponse {
  article: Article;
  content: ArticleContent;
  prereqs: ArticlePrereqRow[];
  forks: ForkWithTitle[];
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

export interface ProfileLink {
  label: string;
  url: string;
}

export interface EducationEntry {
  degree: string;
  school: string;
  year: string;
  current?: boolean;
}

export interface ProfileData {
  did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  article_count: number;
  series_count: number;
  links: ProfileLink[];
  email: string | null;
  education: EducationEntry[];
  affiliation: string | null;
  credentials_verified: boolean;
}

export interface UserSettings {
  native_lang: string;
  known_langs: string[];
  prefer_native: boolean;
  hide_unknown: boolean;
  default_format: string;
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
  description: string;
  content: string;
  content_format: string;
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

export interface Course {
  id: string;
  title: string;
  description: string;
  instructor_did: string;
  cover_url: string | null;
  schedule_type: string;
  term: string | null;
  year: number | null;
  canonical_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CourseUnit {
  id: string;
  course_id: string;
  sort_order: number;
  title: string;
  description: string;
  available_from: string | null;
}

export interface CourseItem {
  id: string;
  unit_id: string;
  sort_order: number;
  role: string;
  target_uri: string | null;
  external_url: string | null;
  title: string;
  note: string;
  due_date: string | null;
}

export interface UnitWithItems {
  unit: CourseUnit;
  items: CourseItem[];
}

export interface CourseDetail {
  course: Course;
  units: UnitWithItems[];
  offerings: Course[];
}

export interface CreateArticle {
  title: string;
  description?: string;
  content: string;
  content_format: string;
  lang?: string;
  license?: string;
  translation_of?: string;
  restricted?: boolean;
  category?: string;
  book_id?: string;
  tags: string[];
  prereqs: { tag_id: string; prereq_type: string }[];
}
