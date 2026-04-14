import type {
  Tag, Article, ArticleContent, ArticlePrereqRow, ContentTeachRow, ContentPrereqBulkRow,
  ForkWithTitle, UserSkill, GraphData, TagTreeEntry, CreateArticle, BookmarkWithTitle,
  AuthUser, VoteSummary, Series, SeriesDetail, SeriesTreeNode, SeriesHeading, ProfileData, SeriesContextItem,
  SkillTree, SkillTreeDetail, SkillTreeEdge, SkillTreePrereq, UserTagPrereq, FrontierSkill, Comment, Draft, CommentVoteResult, MyCommentVote,
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
  const res = await fetch(`${BASE}${path}`, { headers: authHeaders(), credentials: 'same-origin', signal });
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
    credentials: 'same-origin',
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
    credentials: 'same-origin',
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
    credentials: 'same-origin',
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

// Auth — platform-local login & registration
export const login = (identifier: string, password: string) =>
  post<AuthUser>('/auth/login', { identifier, password });
export const register = (handle: string, password: string, display_name?: string) =>
  post<AuthUser>('/auth/register', { handle, password, display_name });
export const logout = () => post<void>('/auth/logout');
export const authMe = () => get<AuthUser>('/auth/me');

// Auth — AT Protocol OAuth login (redirect-based)
export function startOAuthLogin(handle: string) {
  window.location.href = `/oauth/login?handle=${encodeURIComponent(handle)}`;
}
// OAuth me (cookie-based)
export const oauthMe = async (): Promise<{ did: string; handle: string | null } | null> => {
  try {
    const res = await fetch('/oauth/me', { credentials: 'same-origin' });
    if (!res.ok) return null;
    return res.json();
  } catch { return null; }
};

// Tags
export const listTags = () => get<Tag[]>('/tags');
export const getTag = (id: string) => get<Tag>(`/tags/${encodeURIComponent(id)}`);

// Articles
export const listArticles = () => get<Article[]>('/articles');
export const getArticle = (uri: string) => get<Article>(`/articles/by-uri?uri=${encodeURIComponent(uri)}`);
export const getArticleContent = (uri: string) => get<ArticleContent>(`/articles/by-uri/content?uri=${encodeURIComponent(uri)}`);
export const getArticlePrereqs = (uri: string) => get<ArticlePrereqRow[]>(`/articles/by-uri/prereqs?uri=${encodeURIComponent(uri)}`);
export const getArticleForks = (uri: string) => get<ForkWithTitle[]>(`/articles/by-uri/forks?uri=${encodeURIComponent(uri)}`);
export const getForkAhead = (forkUri: string, originalUri: string) =>
  get<string[]>(`/articles/by-uri/fork-ahead?fork_uri=${encodeURIComponent(forkUri)}&original_uri=${encodeURIComponent(originalUri)}`);
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
export const getQuestionsByBook = (bookId: string, limit = 50) => get<Article[]>(`/questions/by-book?book_id=${encodeURIComponent(bookId)}&limit=${limit}`);
export const getRelatedQuestions = (uri: string) => get<Article[]>(`/questions/related?uri=${encodeURIComponent(uri)}`);

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

// Tag tree & prereqs
export const getTagTree = () => get<TagTreeEntry[]>('/tag-tree');
export const getTagPrereqs = () => get<UserTagPrereq[]>('/tag-prereqs');
export const addTagPrereq = (from_tag: string, to_tag: string, prereq_type: string = 'required') =>
  post<void>('/tag-prereqs', { from_tag, to_tag, prereq_type });
export const removeTagPrereq = (from_tag: string, to_tag: string) =>
  del<void>('/tag-prereqs', { from_tag, to_tag });
export const addTagChild = (parent_tag: string, child_tag: string) =>
  post<void>('/tag-tree', { parent_tag, child_tag });

// Recommendations
export const getRecommendations = (limit = 30, offset = 0, category?: string) => {
  let path = `/recommendations?limit=${limit}&offset=${offset}`;
  if (category) path += `&category=${encodeURIComponent(category)}`;
  return get<Article[]>(path);
};
export const getRecommendedQuestions = (limit = 8) => get<Article[]>(`/recommended-questions?limit=${limit}`);
export const getFrontierSkills = () => get<FrontierSkill[]>('/frontier-skills');

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
export const updateBio = (bio: string) => put<void>('/profile/bio', { bio });
export const updatePublications = (pubs: import('./types').PublicationEntry[]) => put<void>('/profile/publications', pubs);
export const updateProjects = (projects: import('./types').ProjectEntry[]) => put<void>('/profile/projects', projects);
export const updateTeaching = (teaching: import('./types').TeachingEntry[]) => put<void>('/profile/teaching', teaching);
export const getUserListings = (did: string) => get<import('./types').Listing[]>(`/profile/listings?did=${encodeURIComponent(did)}`);
export const uploadAvatar = async (file: File): Promise<string> => {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${BASE}/profile/avatar`, { method: 'POST', headers: authHeaders(), body: form });
  if (!res.ok) throw new Error(`${res.status}`);
  const data = await res.json();
  return data.avatar_url;
};

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
export const getSeriesHeadings = (id: string) => get<SeriesHeading[]>(`/series/${encodeURIComponent(id)}/headings`);
export const reorderSeriesArticles = (series_id: string, article_uris: string[]) =>
  put<void>(`/series/${encodeURIComponent(series_id)}/articles/reorder`, { article_uris });
export const compileSeries = (id: string) => post<{ articles_created: number; articles_updated: number; total_headings: number }>(`/series/${encodeURIComponent(id)}/compile`, {});
export const forkSeries = (id: string) => post<Series>(`/series/${encodeURIComponent(id)}/fork`, {});
export const listSeriesFiles = (id: string) => get<{ path: string; size: number }[]>(`/series/${encodeURIComponent(id)}/files`);
export const readSeriesFile = (id: string, path: string) => get<string>(`/series/${encodeURIComponent(id)}/file?path=${encodeURIComponent(path)}`);
export const writeSeriesFile = (id: string, path: string, content: string, message?: string) =>
  put<void>(`/series/${encodeURIComponent(id)}/file`, { path, content, message });
export const deleteSeriesFile = (id: string, path: string) =>
  del<void>(`/series/${encodeURIComponent(id)}/file?path=${encodeURIComponent(path)}`);

// Collaboration
export interface Collaborator {
  series_id: string;
  user_did: string;
  channel_name: string;
  role: string;
  invited_by: string | null;
  created_at: string;
}
export interface ChannelDiffResult {
  only_in_a: string[];
  only_in_b: string[];
}
export const listCollaborators = (id: string) => get<Collaborator[]>(`/series/${encodeURIComponent(id)}/collaborators`);
export const inviteCollaborator = (id: string, user_did: string, role?: string) =>
  post<Collaborator>(`/series/${encodeURIComponent(id)}/collaborators`, { user_did, role });
export const removeCollaborator = (id: string, did: string) =>
  del<void>(`/series/${encodeURIComponent(id)}/collaborators/${encodeURIComponent(did)}`);
export const listChannels = (id: string) => get<string[]>(`/series/${encodeURIComponent(id)}/channels`);
export const readChannelFile = (id: string, channel: string, path: string) =>
  get<{ content: string }>(`/series/${encodeURIComponent(id)}/channel/${encodeURIComponent(channel)}/file?path=${encodeURIComponent(path)}`);
export const writeChannelFile = (id: string, channel: string, path: string, content: string, message?: string) =>
  put<{ change_hash: string; merkle: string }>(`/series/${encodeURIComponent(id)}/channel/${encodeURIComponent(channel)}/file`, { path, content, message });
export const channelLog = (id: string, channel: string) =>
  get<string[]>(`/series/${encodeURIComponent(id)}/channel/${encodeURIComponent(channel)}/log`);
export interface ChangeInfo { hash: string; message: string; author_did?: string }
export interface ChangeLine { kind: 'add' | 'del' | 'same'; content: string }
export interface ChangeDetail { lines: ChangeLine[] }
export const channelLogDetails = (id: string, channel: string) =>
  get<ChangeInfo[]>(`/series/${encodeURIComponent(id)}/channel/${encodeURIComponent(channel)}/log-details`);
export const getSeriesChangeDetail = (id: string, hash: string) =>
  get<ChangeDetail>(`/series/${encodeURIComponent(id)}/change/${encodeURIComponent(hash)}`);
export const unrecordSeriesChange = (id: string, hash: string) =>
  post<void>(`/series/${encodeURIComponent(id)}/change/${encodeURIComponent(hash)}/unrecord`, {});
export const applyChannelChange = (id: string, targetChannel: string, sourceChannel: string, changeHash: string) =>
  post<void>(`/series/${encodeURIComponent(id)}/channel/${encodeURIComponent(targetChannel)}/apply`, { source_channel: sourceChannel, change_hash: changeHash });
export const channelDiff = (id: string, a: string, b: string) =>
  get<ChannelDiffResult>(`/series/${encodeURIComponent(id)}/channel-diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);

// Article Collaboration
export const listArticleCollaborators = (uri: string) => get<ArticleCollaborator[]>(`/articles/collaborators?uri=${encodeURIComponent(uri)}`);
export const inviteArticleCollaborator = (uri: string, user_did: string, role?: string) =>
  post<ArticleCollaborator>('/articles/collaborators', { uri, user_did, role });
export const removeArticleCollaborator = (uri: string, user_did: string) =>
  del<void>('/articles/collaborators/remove', { uri, user_did });
export const listArticleChannels = (uri: string) => get<string[]>(`/articles/channels?uri=${encodeURIComponent(uri)}`);
export const readArticleChannelFile = (uri: string, channel: string, path?: string) =>
  get<{ content: string }>(`/articles/channel/file?uri=${encodeURIComponent(uri)}&channel=${encodeURIComponent(channel)}${path ? `&path=${encodeURIComponent(path)}` : ''}`);
export const writeArticleChannelFile = (uri: string, channel: string, content: string, message?: string, path?: string) =>
  put<{ change_hash: string; merkle: string }>('/articles/channel/file', { uri, channel, content, message, path });
export const articleChannelLog = (uri: string, channel: string) =>
  get<string[]>(`/articles/channel/log?uri=${encodeURIComponent(uri)}&channel=${encodeURIComponent(channel)}`);
export const applyArticleChannelChange = (uri: string, targetChannel: string, changeHash: string) =>
  post<void>('/articles/channel/apply', { uri, target_channel: targetChannel, change_hash: changeHash });
export const articleChannelDiff = (uri: string, a: string, b: string) =>
  get<ChannelDiffResult>(`/articles/channel-diff?uri=${encodeURIComponent(uri)}&a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);

export interface ArticleCollaborator {
  article_uri: string;
  user_did: string;
  channel_name: string;
  role: string;
  invited_by: string | null;
  created_at: string;
}

// Discussions
export interface Discussion {
  id: string;
  target_uri: string;
  source_uri: string;
  author_did: string;
  title: string;
  body: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}
export interface DiscussionChange {
  id: number;
  discussion_id: string;
  change_hash: string;
  added_by: string;
  added_at: string;
  applied: boolean;
  applied_at: string | null;
}
export interface DiscussionDetail {
  discussion: Discussion;
  changes: DiscussionChange[];
}
export const createDiscussion = (data: { target_uri: string; source_uri: string; title: string; body?: string; change_hashes: string[] }) =>
  post<Discussion>('/discussions', data);
export const listDiscussions = (uri: string) => get<Discussion[]>(`/discussions?uri=${encodeURIComponent(uri)}`);
export const getDiscussion = (id: string) => get<DiscussionDetail>(`/discussions/${encodeURIComponent(id)}`);
export const updateDiscussionStatus = (id: string, status: string) =>
  put<void>(`/discussions/${encodeURIComponent(id)}/status`, { status });
export const applyDiscussionChange = (id: string, changeHash: string) =>
  post<{ has_conflicts: boolean }>(`/discussions/${encodeURIComponent(id)}/apply`, { change_hash: changeHash });
export const applyAllDiscussionChanges = (id: string) =>
  post<{ has_conflicts: boolean; applied: number }>(`/discussions/${encodeURIComponent(id)}/apply-all`, {});

// Skill Trees
export const listSkillTrees = () => get<SkillTree[]>('/skill-trees');
export const getSkillTree = (uri: string) => get<SkillTreeDetail>(`/skill-trees/by-uri?uri=${encodeURIComponent(uri)}`);
export const createSkillTree = (data: { title: string; description?: string; tag_id?: string; edges: SkillTreeEdge[]; prereqs?: SkillTreePrereq[] }) =>
  post<SkillTree>('/skill-trees', data);
export const forkSkillTree = (uri: string) => post<SkillTree>('/skill-trees/fork', { uri });
export const addSkillTreeEdge = (tree_uri: string, parent_tag: string, child_tag: string) =>
  post<void>('/skill-trees/edges', { tree_uri, parent_tag, child_tag });
export const removeSkillTreeEdge = (tree_uri: string, parent_tag: string, child_tag: string) =>
  post<void>('/skill-trees/edges/remove', { tree_uri, parent_tag, child_tag });
export const addSkillTreePrereq = (tree_uri: string, from_tag: string, to_tag: string, prereq_type: string = 'required') =>
  post<void>('/skill-trees/prereqs', { tree_uri, from_tag, to_tag, prereq_type });
export const removeSkillTreePrereq = (tree_uri: string, from_tag: string, to_tag: string) =>
  del<void>('/skill-trees/prereqs/remove', { tree_uri, from_tag, to_tag });
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

// Article search
export const searchArticles = (q: string, limit = 20) =>
  get<Article[]>(`/search?q=${encodeURIComponent(q)}&limit=${limit}`);

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

// Creator Dashboard
export interface CreatorStats {
  total_articles: number;
  total_series: number;
  total_drafts: number;
  total_views: number;
  total_comments: number;
  total_bookmarks: number;
}
export interface ArticleStats {
  at_uri: string;
  title: string;
  content_format: string;
  created_at: string;
  views: number;
  comments: number;
  bookmarks: number;
  votes: number;
}
export interface TimelinePoint { day: string; views: number; comments: number; bookmarks: number }
export const getCreatorStats = () => get<CreatorStats>('/creator/stats');
export const getCreatorArticles = () => get<ArticleStats[]>('/creator/articles');
export const getCreatorSeries = () => get<any[]>('/creator/series');
export const getCreatorTimeline = () => get<TimelinePoint[]>('/creator/timeline');
export const publishSeries = (id: string) => post<void>(`/series/${encodeURIComponent(id)}/publish`, {});
export const unpublishSeries = (id: string) => post<void>(`/series/${encodeURIComponent(id)}/unpublish`, {});
export const recordArticleView = (uri: string, viewer_did?: string) =>
  post<void>('/articles/view', { uri, viewer_did });

// Thoughts
export const listThoughts = (limit = 50, offset = 0) =>
  get<Article[]>(`/thoughts?limit=${limit}&offset=${offset}`);
export const createThought = (data: CreateArticle) =>
  post<Article>('/thoughts', data);

// Listings (academic recruitment)
import type { Listing } from './types';
export const listListings = (kind?: string, tag?: string, limit = 30, offset = 0) => {
  const params = new URLSearchParams();
  if (kind) params.set('kind', kind);
  if (tag) params.set('tag', tag);
  params.set('limit', String(limit));
  params.set('offset', String(offset));
  return get<Listing[]>(`/listings?${params}`);
};
export const getListing = (id: string) => get<Listing>(`/listings/${encodeURIComponent(id)}`);
export const createListing = (data: Omit<Listing, 'id' | 'did' | 'author_handle' | 'author_reputation' | 'is_open' | 'created_at' | 'updated_at'>) =>
  post<Listing>('/listings', data);
export const updateListing = (id: string, data: Omit<Listing, 'id' | 'did' | 'author_handle' | 'author_reputation' | 'is_open' | 'created_at' | 'updated_at'>) =>
  put<Listing>(`/listings/${encodeURIComponent(id)}`, data);
export const closeListing = (id: string) => post<void>(`/listings/${encodeURIComponent(id)}/close`, {});
export const reopenListing = (id: string) => post<void>(`/listings/${encodeURIComponent(id)}/reopen`, {});
export const deleteListing = (id: string) => del<void>(`/listings/${encodeURIComponent(id)}`);
export const myListings = () => get<Listing[]>('/listings/mine');
export const matchedListings = (limit = 20) => get<Listing[]>(`/listings/matched?limit=${limit}`);

// --- Admin API ---
// All admin endpoints require x-admin-secret header

function adminHeaders(secret: string): Record<string, string> {
  return { 'x-admin-secret': secret, 'Content-Type': 'application/json' };
}

async function adminGet<T>(path: string, secret: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { headers: adminHeaders(secret) });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

async function adminPost<T>(path: string, secret: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: adminHeaders(secret),
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export interface AdminReport {
  id: string;
  reporter_did: string;
  reporter_handle: string | null;
  target_did: string;
  target_handle: string | null;
  target_uri: string | null;
  kind: string;
  reason: string;
  status: string;
  admin_note: string | null;
  created_at: string;
  resolved_at: string | null;
}

export interface AdminAppeal {
  id: string;
  did: string;
  kind: string;
  target_uri: string | null;
  reason: string;
  status: string;
  admin_response: string | null;
  created_at: string;
  resolved_at: string | null;
}

export interface AdminBannedUser {
  did: string;
  handle: string;
  display_name: string | null;
  banned_at: string | null;
  ban_reason: string | null;
}

export interface AdminPlatformUser {
  did: string;
  handle: string;
  display_name: string | null;
  created_at: string;
}

// Reports
export const adminListReports = (secret: string, status?: string) =>
  adminGet<AdminReport[]>(`/admin/reports${status ? `?status=${status}` : ''}`, secret);
export const adminResolveReport = (secret: string, id: string, status: string, admin_note?: string) =>
  adminPost<unknown>('/admin/reports/resolve', secret, { id, status, admin_note });

// Appeals
export const adminListAppeals = (secret: string) =>
  adminGet<AdminAppeal[]>('/admin/appeals', secret);
export const adminResolveAppeal = (secret: string, id: string, status: string, response?: string) =>
  adminPost<AdminAppeal>('/admin/appeals/resolve', secret, { id, status, response });

// Bans
export const adminListBannedUsers = (secret: string) =>
  adminGet<AdminBannedUser[]>('/admin/banned-users', secret);
export const adminBanUser = (secret: string, did: string, reason?: string) =>
  adminPost<unknown>('/admin/ban-user', secret, { did, reason });
export const adminUnbanUser = (secret: string, did: string) =>
  adminPost<unknown>('/admin/unban-user', secret, { did });

// Users
export const adminListUsers = (secret: string) =>
  adminGet<AdminPlatformUser[]>('/admin/platform-users', secret);

// Articles
export const adminDeleteArticle = (secret: string, uri: string, reason?: string) =>
  adminPost<unknown>('/admin/articles/delete', secret, { uri, reason });
export const adminSetVisibility = (secret: string, uri: string, visibility: string, reason?: string) =>
  adminPost<unknown>('/admin/articles/visibility', secret, { uri, visibility, reason });
