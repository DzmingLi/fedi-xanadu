import type {
  Tag, Article, ArticleContent, ArticlePrereqRow, ContentTeachRow, ContentPrereqBulkRow,
  ForkWithTitle, UserSkill, GraphData, TagTreeEntry, CreateArticle, BookmarkWithTitle,
  AuthUser, VoteSummary, Series, SeriesDetail, SeriesTreeNode, ProfileData, SeriesContextItem,
  SkillTree, SkillTreeDetail, SkillTreeEdge, Comment, Draft, CommentVoteResult, MyCommentVote,
  ArticleFullResponse, Notification, QuestionDetail, AccessGrant, UserSettings,
  BlockedUser, Report, Book, BookDetail, BookEdition,
} from './types';
import { getToken } from './auth';

const BASE = '/api';

function authHeaders(): Record<string, string> {
  const token = getToken();
  const headers: Record<string, string> = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  return headers;
}

async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { headers: authHeaders(), signal });
  if (!res.ok) {
    if (res.status === 429) throw new Error('请求过于频繁，请稍后再试');
    throw new Error(`${res.status} ${res.statusText}`);
  }
  return res.json();
}

async function post<T>(path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: body ? JSON.stringify(body) : undefined,
    signal,
  });
  if (!res.ok) {
    if (res.status === 429) throw new Error('请求过于频繁，请稍后再试');
    const text = await res.text();
    throw new Error(text || `${res.status} ${res.statusText}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

// Auth
export const login = (identifier: string, password: string) =>
  post<AuthUser>('/auth/login', { identifier, password });
export const logout = () => post<void>('/auth/logout');
export const authMe = () => get<AuthUser>('/auth/me');

// Tags
export const listTags = () => get<Tag[]>('/tags');
export const getTag = (id: string) => get<Tag>(`/tags/by-id?id=${encodeURIComponent(id)}`);

// Articles
export const listArticles = () => get<Article[]>('/articles');
export const getArticle = (uri: string) => get<Article>(`/articles/by-uri?uri=${encodeURIComponent(uri)}`);
export const getArticleContent = (uri: string) => get<ArticleContent>(`/articles/by-uri/content?uri=${encodeURIComponent(uri)}`);
export const getArticlePrereqs = (uri: string) => get<ArticlePrereqRow[]>(`/articles/by-uri/prereqs?uri=${encodeURIComponent(uri)}`);
export const getArticleForks = (uri: string) => get<ForkWithTitle[]>(`/articles/by-uri/forks?uri=${encodeURIComponent(uri)}`);
export const createArticle = (data: CreateArticle) => post<Article>('/articles', data);
export const getAllArticleTeaches = () => get<ContentTeachRow[]>('/articles/all-teaches');
export const getAllArticlePrereqs = () => get<ContentPrereqBulkRow[]>('/articles/all-prereqs');
export const getArticlesByTag = (tagId: string) => get<Article[]>(`/articles/by-tag?tag_id=${encodeURIComponent(tagId)}`);
export const getArticlesByDid = (did: string) => get<Article[]>(`/articles/by-did?did=${encodeURIComponent(did)}`);
export const getTranslations = (uri: string) => get<Article[]>(`/articles/translations?uri=${encodeURIComponent(uri)}`);
export const getArticleFull = (uri: string) => get<ArticleFullResponse>(`/articles/full?uri=${encodeURIComponent(uri)}`);

// Questions & Answers
export const listQuestions = (limit = 50, offset = 0) => get<Article[]>(`/questions?limit=${limit}&offset=${offset}`);
export const getQuestionDetail = (uri: string) => get<QuestionDetail>(`/questions/by-uri?uri=${encodeURIComponent(uri)}`);
export const createQuestion = (data: CreateArticle) => post<Article>('/questions', data);
export const postAnswer = (data: CreateArticle) => post<Article>('/questions/answer', data);
export const getQuestionsByDid = (did: string, limit = 50) => get<Article[]>(`/questions/by-did?did=${encodeURIComponent(did)}&limit=${limit}`);
export const getAnswersByDid = (did: string, limit = 50) => get<Article[]>(`/answers/by-did?did=${encodeURIComponent(did)}&limit=${limit}`);
export const getQuestionsByTag = (tagId: string, limit = 50) => get<Article[]>(`/questions/by-tag?tag_id=${encodeURIComponent(tagId)}&limit=${limit}`);

// Votes
export const castVote = (target_uri: string, value: number) =>
  post<VoteSummary>('/vote', { target_uri, value });
export const getArticleVotes = (uri: string) => get<VoteSummary>(`/votes?uri=${encodeURIComponent(uri)}`);
export const getMyVote = (uri: string) => get<{ value: number }>(`/votes/my?uri=${encodeURIComponent(uri)}`);

// Skills
export const listSkills = () => get<UserSkill[]>('/skills');
export const lightSkill = (tag_id: string, status: 'mastered' | 'learning' = 'mastered') =>
  post<void>('/skills', { tag_id, status });
export const unlightSkill = (tag_id: string) => post<void>('/skills/unlight', { tag_id });

// Tag tree
export const getTagTree = () => get<TagTreeEntry[]>('/tag-tree');

// Bookmarks
export const listBookmarks = () => get<BookmarkWithTitle[]>('/bookmarks');
export const addBookmark = (article_uri: string, folder_path?: string) =>
  post<void>('/bookmarks', { article_uri, folder_path });
export const removeBookmark = (uri: string) => post<void>('/bookmarks/remove', { uri });
export const moveBookmark = (article_uri: string, folder_path: string) =>
  post<void>('/bookmarks/move', { article_uri, folder_path });
export const listBookmarkFolders = () => get<string[]>('/bookmarks/folders');
export const listPublicBookmarks = (did: string) => get<BookmarkWithTitle[]>(`/bookmarks/public?did=${encodeURIComponent(did)}`);

// Interests
export const getInterests = () => get<string[]>('/interests');
export const setInterests = (tag_ids: string[]) => post<void>('/interests', { tag_ids });

// Profile
export const getProfile = (did: string) => get<ProfileData>(`/profile?did=${encodeURIComponent(did)}`);
export const updateProfileLinks = (links: { label: string; url: string }[]) =>
  post<void>('/profile/links', { links });

// Series
export const listSeries = () => get<Series[]>('/series');
export const getSeries = (id: string) => get<SeriesDetail>(`/series/by-id?id=${encodeURIComponent(id)}`);
export const createSeries = (data: { title: string; description?: string; topics?: string[]; parent_id?: string; category?: string }) =>
  post<Series>('/series', data);
export const addSeriesArticle = (series_id: string, article_uri: string) =>
  post<void>('/series/articles', { series_id, article_uri });
export const removeSeriesArticle = (series_id: string, article_uri: string) =>
  post<void>('/series/articles/remove', { series_id, article_uri });
export const addSeriesPrereq = (series_id: string, article_uri: string, prereq_article_uri: string) =>
  post<void>('/series/prereqs', { series_id, article_uri, prereq_article_uri });
export const removeSeriesPrereq = (series_id: string, article_uri: string, prereq_article_uri: string) =>
  post<void>('/series/prereqs/remove', { series_id, article_uri, prereq_article_uri });
export const getSeriesContext = (uri: string) => get<SeriesContextItem[]>(`/series/context?uri=${encodeURIComponent(uri)}`);
export const getAllSeriesArticles = () => get<{ series_id: string; article_uri: string }[]>('/series/all-articles');
export const getSeriesTree = (id: string) => get<SeriesTreeNode>(`/series/tree?id=${encodeURIComponent(id)}`);
export const reorderSeriesArticles = (series_id: string, article_uris: string[]) =>
  post<void>('/series/articles/reorder', { series_id, article_uris });
export const reorderSeriesChildren = (parent_id: string, child_ids: string[]) =>
  post<void>('/series/children/reorder', { parent_id, child_ids });

// Skill Trees
export const listSkillTrees = () => get<SkillTree[]>('/skill-trees');
export const getSkillTree = (uri: string) => get<SkillTreeDetail>(`/skill-trees/by-uri?uri=${encodeURIComponent(uri)}`);
export const createSkillTree = (data: { title: string; description?: string; tag_id?: string; edges: SkillTreeEdge[] }) =>
  post<SkillTree>('/skill-trees', data);
export const forkSkillTree = (uri: string) => post<SkillTree>('/skill-trees/fork', { uri });
export const addSkillTreeEdge = (tree_uri: string, parent_tag: string, child_tag: string) =>
  post<void>('/skill-trees/edges', { tree_uri, parent_tag, child_tag });
export const removeSkillTreeEdge = (tree_uri: string, parent_tag: string, child_tag: string) =>
  post<void>('/skill-trees/edges/remove', { tree_uri, parent_tag, child_tag });
export const adoptSkillTree = (tree_uri: string) => post<void>('/skill-trees/adopt', { tree_uri });
export const getActiveTree = () => get<SkillTreeDetail | null>('/skill-trees/active');
export const createTagInline = (id: string, name: string) => post<Tag>('/tags', { id, name });

// Follows
export interface FollowedUser {
  follows_did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
  has_update: boolean;
}
export const listFollows = () => get<FollowedUser[]>('/follows');

export interface FollowEntry {
  did: string;
  handle: string | null;
  display_name: string | null;
  avatar_url: string | null;
}
export const getFollowing = (did: string) => get<FollowEntry[]>(`/follows/following?did=${encodeURIComponent(did)}`);
export const getFollowers = (did: string) => get<FollowEntry[]>(`/follows/followers?did=${encodeURIComponent(did)}`);
export const followUser = (did: string) => post<void>('/follows', { did });
export const unfollowUser = (did: string) => post<void>('/follows/remove', { did });
export const markFollowSeen = (did: string) => post<void>('/follows/seen', { did });

// Keybindings
export const getKeybindings = () => get<{ bindings: Record<string, string> }>('/keybindings');
export const setKeybindings = (bindings: Record<string, string>) =>
  post<{ bindings: Record<string, string> }>('/keybindings', { bindings });

// Settings
export const getSettings = () => get<UserSettings>('/settings');
export const setSettings = (settings: UserSettings) => post<UserSettings>('/settings', settings);

// Blocks
export const blockUser = (did: string) => post<void>('/blocks', { did });
export const unblockUser = (did: string) => post<void>('/blocks/remove', { did });
export const listBlockedUsers = () => get<BlockedUser[]>('/blocks');
export const listBlockedDids = () => get<string[]>('/blocks/dids');

// Reports
export const createReport = (target_did: string, kind: string, reason: string, target_uri?: string) =>
  post<Report>('/reports', { target_did, target_uri, kind, reason });

// Fork
export const forkArticle = (uri: string) => post<Article>('/articles/fork', { uri });

// Image upload
export async function uploadImage(articleUri: string, file: File): Promise<{ filename: string }> {
  const form = new FormData();
  form.append('article_uri', articleUri);
  form.append('file', file);
  const res = await fetch(`${BASE}/articles/upload-image`, {
    method: 'POST',
    headers: authHeaders(),
    body: form,
  });
  if (!res.ok) {
    if (res.status === 429) throw new Error('请求过于频繁，请稍后再试');
    const text = await res.text();
    throw new Error(text || `${res.status} ${res.statusText}`);
  }
  return res.json();
}

// Comments
export const listComments = (uri: string) => get<Comment[]>(`/comments?uri=${encodeURIComponent(uri)}`);
export const createComment = (content_uri: string, body: string, parent_id?: string, quote_text?: string) =>
  post<Comment>('/comments', { content_uri, body, parent_id, quote_text });
export const updateComment = (id: string, body: string) =>
  post<Comment>('/comments/update', { id, body });
export const deleteComment = (id: string) =>
  post<void>('/comments/delete', { id });
export const voteComment = (comment_id: string, value: number) =>
  post<CommentVoteResult>('/comments/vote', { comment_id, value });
export const getMyCommentVotes = (uri: string) =>
  get<MyCommentVote[]>(`/comments/my-votes?uri=${encodeURIComponent(uri)}`);

// Article edit/delete
export const updateArticle = (uri: string, data: { title?: string; description?: string; content?: string }) =>
  post<Article>('/articles/update', { uri, ...data });
export const deleteArticle = (uri: string) =>
  post<void>('/articles/delete', { uri });

// Access control (paywall)
export const setRestricted = (uri: string, restricted: boolean) =>
  post<void>('/articles/restricted', { uri, restricted });
export const grantAccess = (uri: string, grantee_did: string) =>
  post<void>('/articles/access/grant', { uri, grantee_did });
export const revokeAccess = (uri: string, grantee_did: string) =>
  post<void>('/articles/access/revoke', { uri, grantee_did });
export const listAccessGrants = (uri: string) =>
  get<AccessGrant[]>(`/articles/access/list?uri=${encodeURIComponent(uri)}`);

// Drafts
export const listDrafts = () => get<Draft[]>('/drafts');
export const saveDraft = (data: CreateArticle) => post<Draft>('/drafts', data);
export const updateDraft = (id: string, data: Partial<CreateArticle>) =>
  post<Draft>('/drafts/update', { id, ...data });
export const deleteDraft = (id: string) => post<void>('/drafts/delete', { id });
export const publishDraft = (id: string) => post<Article>('/drafts/publish', { id });

// Learned marks
export const markLearned = (article_uri: string) => post<void>('/learned', { article_uri });
export const unmarkLearned = (article_uri: string) => post<void>('/learned/remove', { article_uri });
export const isLearned = (uri: string) => get<{ learned: boolean }>(`/learned/check?uri=${encodeURIComponent(uri)}`);

// Tag search
export const searchTags = (q: string) => get<Tag[]>(`/tags/search?q=${encodeURIComponent(q)}`);

// Notifications
export const listNotifications = (limit = 50, offset = 0) =>
  get<Notification[]>(`/notifications?limit=${limit}&offset=${offset}`);
export const getUnreadCount = () =>
  get<{ count: number }>('/notifications/unread');
export const markNotificationRead = (id: string) =>
  post<void>('/notifications/read', { id });
export const markAllNotificationsRead = () =>
  post<void>('/notifications/read-all');

// Graph
export const getGraph = () => get<GraphData>('/graph');

// Books
export const listBooks = (limit = 50, offset = 0) =>
  get<Book[]>(`/books?limit=${limit}&offset=${offset}`);
export const getBook = (id: string) =>
  get<BookDetail>(`/books/by-id?id=${encodeURIComponent(id)}`);
export const createBook = (data: { title: string; authors: string[]; description?: string; cover_url?: string; tags: string[]; prereqs?: string[] }) =>
  post<Book>('/books', data);
export const updateBook = (id: string, data: { title?: string; description?: string; cover_url?: string; edit_summary?: string }) =>
  post<Book>('/books/update', { id, ...data });
export const addBookEdition = (book_id: string, edition: { title: string; lang: string; isbn?: string; publisher?: string; year?: string; translators?: string[]; purchase_links?: { label: string; url: string }[] }) =>
  post<BookEdition>('/books/editions', { book_id, ...edition });
export const getBookEditHistory = (id: string) =>
  get<any[]>(`/books/history?id=${encodeURIComponent(id)}`);
