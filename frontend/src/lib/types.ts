export interface Tag {
  id: string;
  name: string;
  description: string | null;
  created_by: string;
  created_at: string;
}

export interface Article {
  at_uri: string;
  did: string;
  author_handle: string | null;
  title: string;
  description: string;
  content_hash: string | null;
  content_format: string;
  lang: string;
  translation_group: string | null;
  license: string;
  prereq_threshold: number;
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

export interface ArticleTagRow {
  article_uri: string;
  tag_id: string;
  tag_name: string;
}

export interface ArticlePrereqBulkRow {
  article_uri: string;
  tag_id: string;
  prereq_type: string;
  tag_name: string;
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
  tag_id: string;
  tag_name?: string;
  created_by: string;
  created_at: string;
}

export interface SeriesArticle {
  series_id: string;
  article_uri: string;
  title: string;
  description: string;
  lang: string;
}

export interface SeriesArticlePrereq {
  article_uri: string;
  prereq_article_uri: string;
}

export interface SeriesDetail {
  series: Series;
  articles: SeriesArticle[];
  prereqs: SeriesArticlePrereq[];
}

export interface SkillTree {
  at_uri: string;
  did: string;
  author_handle?: string;
  title: string;
  description: string | null;
  field: string | null;
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
  tag_names: Record<string, string>;
}

export interface SeriesContextItem {
  series_id: string;
  series_title: string;
  total: number;
  prev: { article_uri: string; title: string }[];
  next: { article_uri: string; title: string }[];
}

export interface ProfileLink {
  label: string;
  url: string;
}

export interface ProfileData {
  did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  article_count: number;
  series_count: number;
  links: ProfileLink[];
}

export interface Comment {
  id: string;
  article_uri: string;
  did: string;
  author_handle: string | null;
  parent_id: string | null;
  body: string;
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

export interface CreateArticle {
  title: string;
  description?: string;
  content: string;
  content_format: string;
  lang?: string;
  license?: string;
  translation_of?: string;
  tags: string[];
  prereqs: { tag_id: string; prereq_type: string }[];
}
