import type {
  Tag, Article, ArticleContent, ArticlePrereqRow, ContentTeachRow, ContentPrereqBulkRow,
  ForkWithTitle, UserSkill, GraphData, TagTreeEntry, CreateArticle, BookmarkWithTitle,
  AuthUser, VoteSummary, Series, SeriesDetail, SeriesTreeNode, ProfileData, SeriesContextItem,
  SkillTree, SkillTreeDetail, SkillTreeEdge, Comment, Draft, CommentVoteResult, MyCommentVote,
  ArticleFullResponse, Notification, QuestionDetail, AccessGrant, UserSettings,
  BlockedUser, Report, Book, BookDetail, BookEdition, BookChapter, ChapterPrereqEntry,
  ArticleVersion, ArticleVersionInfo, ArticleVersionFull, VersionDiff,
} from './types';
import { getToken, setAuth } from './auth.svelte';

const BASE = '/api';

function handleUnauthorized(status: number) {
  if (status === 401) setAuth(null);
}

function authHeaders(): Record<string, string> {
  const token = getToken();
  const headers: Record<string, string> = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  return headers;
}

async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { headers: authHeaders(), signal });
  if (!res.ok) {
    handleUnauthorized(res.status);
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
    handleUnauthorized(res.status);
    if (res.status === 429) throw new Error('请求过于频繁，请稍后再试');
    const text = await res.text();
    throw new Error(text || `${res.status} ${res.statusText}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

async function put<T>(path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: body ? JSON.stringify(body) : undefined,
    signal,
  });
  if (!res.ok) {
    handleUnauthorized(res.status);
    if (res.status === 429) throw new Error('请求过于频繁，请稍后再试');
    const text = await res.text();
    throw new Error(text || `${res.status} ${res.statusText}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

async function del<T>(path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: body ? JSON.stringify(body) : undefined,
    signal,
  });
  if (!res.ok) {
    handleUnauthorized(res.status);
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
export const getTag = (id: string) => get<Tag>(`/tags/${encodeURIComponent(id)}`);

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

// Version history
export const getArticleHistory = (uri: string) => get<ArticleVersionInfo[]>(`/articles/by-uri/history?uri=${encodeURIComponent(uri)}`);
export const getArticleVersion = (uri: string, id: number) => get<ArticleVersionFull>(`/articles/by-uri/version?uri=${encodeURIComponent(uri)}&id=${id}`);
export const getArticleDiff = (uri: string, from: number, to: number) => get<VersionDiff>(`/articles/by-uri/diff?uri=${encodeURIComponent(uri)}&from=${from}&to=${to}`);
export const unrecordArticleChange = (uri: string, version_id: number) => post<void>(`/articles/by-uri/unrecord`, { uri, version_id });

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
  post<VoteSummary>('/votes', { target_uri, value });
export const getArticleVotes = (uri: string) => get<VoteSummary>(`/votes?uri=${encodeURIComponent(uri)}`);
export const getVotesBatch = (uris: string[]) => post<VoteSummary[]>('/votes/batch', uris);
export const getMyVote = (uri: string) => get<{ value: number }>(`/votes/my?uri=${encodeURIComponent(uri)}`);

// Skills
export const listSkills = () => get<UserSkill[]>('/skills');
export const lightSkill = (tag_id: string, status: 'mastered' | 'learning' = 'mastered') =>
  post<void>('/skills', { tag_id, status });
export const unlightSkill = (tag_id: string) => del<void>('/skills/unlight', { tag_id });

// Tag tree
export const getTagTree = () => get<TagTreeEntry[]>('/tag-tree');

// Bookmarks
export const listBookmarks = () => get<BookmarkWithTitle[]>('/bookmarks');
export const addBookmark = (article_uri: string, folder_path?: string) =>
  post<void>('/bookmarks', { article_uri, folder_path });
export const removeBookmark = (uri: string) => del<void>('/bookmarks/remove', { uri });
export const moveBookmark = (article_uri: string, folder_path: string) =>
  post<void>('/bookmarks/move', { article_uri, folder_path });
export const listBookmarkFolders = () => get<string[]>('/bookmarks/folders');
export const listPublicBookmarks = (did: string) => get<BookmarkWithTitle[]>(`/bookmarks/public?did=${encodeURIComponent(did)}`);

// Interests
export const getInterests = () => get<string[]>('/interests');
export const setInterests = (tag_ids: string[]) => put<void>('/interests', { tag_ids });

// Profile
export const getProfile = (did: string) => get<ProfileData>(`/profile?did=${encodeURIComponent(did)}`);
export const updateProfileLinks = (links: { label: string; url: string }[]) =>
  post<void>('/profile/links', { links });

// Series
export const listSeries = () => get<Series[]>('/series');
export const getSeries = (id: string) => get<SeriesDetail>(`/series/${encodeURIComponent(id)}`);
export const createSeries = (data: { title: string; description?: string; long_description?: string; topics?: string[]; category?: string }) =>
  post<Series>('/series', data);
export const addSeriesArticle = (series_id: string, article_uri: string) =>
  post<void>(`/series/${encodeURIComponent(series_id)}/articles`, { article_uri });
export const removeSeriesArticle = (series_id: string, article_uri: string) =>
  del<void>(`/series/${encodeURIComponent(series_id)}/articles/remove`, { article_uri });
export const addSeriesPrereq = (series_id: string, article_uri: string, prereq_article_uri: string) =>
  post<void>(`/series/${encodeURIComponent(series_id)}/prereqs`, { article_uri, prereq_article_uri });
export const removeSeriesPrereq = (series_id: string, article_uri: string, prereq_article_uri: string) =>
  del<void>(`/series/${encodeURIComponent(series_id)}/prereqs/remove`, { article_uri, prereq_article_uri });
export const getSeriesContext = (uri: string) => get<SeriesContextItem[]>(`/series/context?uri=${encodeURIComponent(uri)}`);
export const getAllSeriesArticles = () => get<{ series_id: string; article_uri: string }[]>('/series/all-articles');
export const getSeriesTree = (id: string) => get<SeriesTreeNode>(`/series/${encodeURIComponent(id)}/tree`);
export const reorderSeriesArticles = (series_id: string, article_uris: string[]) =>
  put<void>(`/series/${encodeURIComponent(series_id)}/articles/reorder`, { article_uris });

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
  put<{ bindings: Record<string, string> }>('/keybindings', { bindings });

// Settings
export const getSettings = () => get<UserSettings>('/settings');
export const setSettings = (settings: UserSettings) => put<UserSettings>('/settings', settings);

// Blocks
export const blockUser = (did: string) => post<void>('/blocks', { did });
export const unblockUser = (did: string) => post<void>('/blocks/remove', { did });
export const listBlockedUsers = () => get<BlockedUser[]>('/blocks');
export const listBlockedDids = () => get<string[]>('/blocks/dids');

// Reports
export const createReport = (target_did: string, kind: string, reason: string, target_uri?: string) =>
  post<Report>('/reports', { target_did, target_uri, kind, reason });

// Fork
export const forkArticle = (uri: string, targetFormat?: string) => post<Article>('/articles/fork', { uri, target_format: targetFormat });

// Cross-fork apply
export const applyChange = (data: { source_uri: string; target_uri: string; change_hash: string }) =>
  post<{ has_conflicts: boolean; content: string }>('/articles/apply-change', data);

// Format conversion
export const convertContent = (content: string, from: string, to: string) =>
  post<{ content: string }>('/articles/convert', { content, from, to });

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
  put<Comment>(`/comments/${encodeURIComponent(id)}`, { body });
export const deleteComment = (id: string) =>
  del<void>(`/comments/${encodeURIComponent(id)}`);
export const voteComment = (comment_id: string, value: number) =>
  post<CommentVoteResult>(`/comments/${encodeURIComponent(comment_id)}/vote`, { value });
export const getMyCommentVotes = (uri: string) =>
  get<MyCommentVote[]>(`/comments/my-votes?uri=${encodeURIComponent(uri)}`);

// Article edit/delete
export const updateArticle = (uri: string, data: { title?: string; description?: string; content?: string; commit_message?: string; record?: boolean }) =>
  put<Article>('/articles/update', { uri, ...data });
/** Save content to working copy without creating a pijul change. */
export const saveArticle = (uri: string, data: { title?: string; description?: string; content?: string }) =>
  put<Article>('/articles/update', { uri, ...data, record: false });
/** Explicitly record the current working copy as a pijul change. Returns updated history. */
export const recordArticle = (uri: string, message: string) =>
  post<ArticleVersionInfo[]>('/articles/by-uri/record', { uri, message });
export const deleteArticle = (uri: string) =>
  del<void>('/articles/delete', { uri });

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
  put<Draft>(`/drafts/${encodeURIComponent(id)}`, data);
export const deleteDraft = (id: string) => del<void>(`/drafts/${encodeURIComponent(id)}`);
export const publishDraft = (id: string) => post<Article>(`/drafts/${encodeURIComponent(id)}/publish`);

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
  get<BookDetail>(`/books/${encodeURIComponent(id)}`);
export const createBook = (data: { title: string; authors: string[]; description?: string; cover_url?: string; tags: string[]; prereqs?: string[] }) =>
  post<Book>('/books', data);
export const updateBook = (id: string, data: { title?: string; description?: string; cover_url?: string; edit_summary?: string }) =>
  put<Book>(`/books/${encodeURIComponent(id)}`, data);
export const addBookEdition = (book_id: string, edition: { title: string; lang: string; isbn?: string; publisher?: string; year?: string; translators?: string[]; purchase_links?: { label: string; url: string }[]; cover_url?: string }) =>
  post<BookEdition>(`/books/${encodeURIComponent(book_id)}/editions`, edition);
export const getBookEditHistory = (id: string) =>
  get<any[]>(`/books/${encodeURIComponent(id)}/history`);
export const rateBook = (book_id: string, rating: number) =>
  post<{ avg_rating: number; rating_count: number }>(`/books/${encodeURIComponent(book_id)}/rate`, { rating });
export const setReadingStatus = (book_id: string, status: string, progress: number = 0) =>
  post<void>(`/books/${encodeURIComponent(book_id)}/reading-status`, { status, progress });
export const removeReadingStatus = (book_id: string) =>
  del<void>(`/books/${encodeURIComponent(book_id)}/reading-status`);

// Book chapters
export const listChapters = (book_id: string) =>
  get<BookChapter[]>(`/books/${encodeURIComponent(book_id)}/chapters`);
export const createChapter = (book_id: string, chapter: { title: string; parent_id?: string; order_index: number; article_uri?: string; teaches?: string[]; prereqs?: ChapterPrereqEntry[] }) =>
  post<BookChapter>(`/books/${encodeURIComponent(book_id)}/chapters`, chapter);
export const deleteChapter = (book_id: string, chapter_id: string) =>
  del<void>(`/books/${encodeURIComponent(book_id)}/chapters/delete`, { chapter_id });
export const setChapterProgress = (book_id: string, chapter_id: string, completed: boolean) =>
  post<void>(`/books/${encodeURIComponent(book_id)}/chapters/progress`, { chapter_id, completed });
export const updateChapterTags = (book_id: string, chapter_id: string, teaches: string[], prereqs: ChapterPrereqEntry[]) =>
  put<void>(`/books/${encodeURIComponent(book_id)}/chapters/tags`, { chapter_id, teaches, prereqs });

// Members
export const listMembers = () =>
  get<{ author_did: string; member_did: string; created_at: string }[]>('/members');
export const addMember = (member_did: string) =>
  post<void>('/members', { member_did });
export const removeMember = (member_did: string) =>
  post<void>('/members/remove', { member_did });
export const checkMembership = (author_did: string) =>
  get<{ is_member: boolean }>(`/members/check?author_did=${encodeURIComponent(author_did)}`);

// Typst rendering
export const renderTypstSnippet = (formula: string, display: boolean) =>
  post<{ html: string }>('/render/typst-snippet', { formula, display });
