// Query key factories
export const keys = {
  articles: {
    all: ['articles'] as const,
    full: (uri: string) => ['articles', 'full', uri] as const,
    byTag: (tagId: string) => ['articles', 'byTag', tagId] as const,
    byDid: (did: string) => ['articles', 'byDid', did] as const,
    teaches: ['articles', 'teaches'] as const,
    prereqs: ['articles', 'prereqs'] as const,
  },
  tags: {
    all: ['tags'] as const,
    byId: (id: string) => ['tags', id] as const,
    tree: ['tags', 'tree'] as const,
    search: (q: string) => ['tags', 'search', q] as const,
  },
  votes: {
    byUri: (uri: string) => ['votes', uri] as const,
    myVote: (uri: string) => ['votes', 'my', uri] as const,
    batch: (uris: string[]) => ['votes', 'batch', ...uris] as const,
  },
  skills: {
    all: ['skills'] as const,
  },
  bookmarks: {
    all: ['bookmarks'] as const,
    folders: ['bookmarks', 'folders'] as const,
    public: (did: string) => ['bookmarks', 'public', did] as const,
  },
  interests: {
    all: ['interests'] as const,
  },
  profile: {
    byDid: (did: string) => ['profile', did] as const,
  },
  series: {
    all: ['series'] as const,
    byId: (id: string) => ['series', id] as const,
    tree: (id: string) => ['series', 'tree', id] as const,
    context: (uri: string) => ['series', 'context', uri] as const,
    allArticles: ['series', 'allArticles'] as const,
  },
  skillTrees: {
    all: ['skillTrees'] as const,
    byUri: (uri: string) => ['skillTrees', uri] as const,
    active: ['skillTrees', 'active'] as const,
  },
  follows: {
    all: ['follows'] as const,
    following: (did: string) => ['follows', 'following', did] as const,
    followers: (did: string) => ['follows', 'followers', did] as const,
  },
  comments: {
    byUri: (uri: string) => ['comments', uri] as const,
    myVotes: (uri: string) => ['comments', 'myVotes', uri] as const,
  },
  drafts: {
    all: ['drafts'] as const,
  },
  notifications: {
    all: ['notifications'] as const,
    unread: ['notifications', 'unread'] as const,
  },
  graph: {
    all: ['graph'] as const,
  },
  books: {
    all: ['books'] as const,
    byId: (id: string) => ['books', id] as const,
    chapters: (bookId: string) => ['books', 'chapters', bookId] as const,
    history: (id: string) => ['books', 'history', id] as const,
  },
  settings: {
    all: ['settings'] as const,
  },
  keybindings: {
    all: ['keybindings'] as const,
  },
  blocks: {
    all: ['blocks'] as const,
    dids: ['blocks', 'dids'] as const,
  },
  members: {
    all: ['members'] as const,
    check: (did: string) => ['members', 'check', did] as const,
  },
  questions: {
    all: (limit?: number, offset?: number) => ['questions', limit, offset] as const,
    byUri: (uri: string) => ['questions', uri] as const,
    byTag: (tagId: string) => ['questions', 'byTag', tagId] as const,
    byDid: (did: string) => ['questions', 'byDid', did] as const,
  },
  answers: {
    byDid: (did: string) => ['answers', 'byDid', did] as const,
  },
  access: {
    grants: (uri: string) => ['access', 'grants', uri] as const,
  },
  learned: {
    check: (uri: string) => ['learned', uri] as const,
  },
};
