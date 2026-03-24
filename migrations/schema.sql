-- Fedi-Xanadu schema

-- Global tags
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- User private tag nesting
CREATE TABLE IF NOT EXISTS user_tag_tree (
    did TEXT NOT NULL,
    parent_tag TEXT NOT NULL REFERENCES tags(id),
    child_tag TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, parent_tag, child_tag)
);

-- Articles
CREATE TABLE IF NOT EXISTS articles (
    at_uri TEXT PRIMARY KEY,
    did TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    content_hash TEXT,
    content_format TEXT NOT NULL DEFAULT 'typst',
    lang TEXT NOT NULL DEFAULT 'zh',
    translation_group TEXT,
    license TEXT NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    prereq_threshold REAL NOT NULL DEFAULT 0.8,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_articles_did ON articles(did);
CREATE INDEX IF NOT EXISTS idx_articles_translation_group ON articles(translation_group);

-- Article-tag association
CREATE TABLE IF NOT EXISTS article_tags (
    article_uri TEXT NOT NULL REFERENCES articles(at_uri),
    tag_id TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY (article_uri, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_article_tags_tag ON article_tags(tag_id);

-- Article prereqs
CREATE TABLE IF NOT EXISTS article_prereqs (
    article_uri TEXT NOT NULL REFERENCES articles(at_uri),
    tag_id TEXT NOT NULL REFERENCES tags(id),
    prereq_type TEXT NOT NULL DEFAULT 'required',
    PRIMARY KEY (article_uri, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_article_prereqs_tag ON article_prereqs(tag_id);

-- Fork graph
CREATE TABLE IF NOT EXISTS forks (
    fork_uri TEXT PRIMARY KEY,
    source_uri TEXT NOT NULL REFERENCES articles(at_uri),
    forked_uri TEXT NOT NULL REFERENCES articles(at_uri),
    pijul_patch_hash TEXT,
    vote_score INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_forks_source ON forks(source_uri);

-- User skills
CREATE TABLE IF NOT EXISTS user_skills (
    did TEXT NOT NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id),
    status TEXT NOT NULL DEFAULT 'mastered',
    lit_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (did, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_user_skills_did ON user_skills(did);

-- Votes
CREATE TABLE IF NOT EXISTS votes (
    at_uri TEXT PRIMARY KEY,
    target_uri TEXT NOT NULL,
    did TEXT NOT NULL,
    value INTEGER NOT NULL,
    UNIQUE(target_uri, did)
);

CREATE INDEX IF NOT EXISTS idx_votes_target ON votes(target_uri);

-- Bookmarks
CREATE TABLE IF NOT EXISTS user_bookmarks (
    did TEXT NOT NULL,
    article_uri TEXT NOT NULL REFERENCES articles(at_uri),
    folder_path TEXT NOT NULL DEFAULT '/',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (did, article_uri)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_did ON user_bookmarks(did);
CREATE INDEX IF NOT EXISTS idx_bookmarks_folder ON user_bookmarks(did, folder_path);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    token TEXT PRIMARY KEY,
    did TEXT NOT NULL,
    handle TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    pds_url TEXT NOT NULL,
    access_jwt TEXT NOT NULL,
    refresh_jwt TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL DEFAULT (datetime('now', '+7 days'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_did ON sessions(did);

-- User interests
CREATE TABLE IF NOT EXISTS user_interests (
    did TEXT NOT NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, tag_id)
);

-- Series
CREATE TABLE IF NOT EXISTS series (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    tag_id TEXT NOT NULL REFERENCES tags(id),
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_series_tag ON series(tag_id);

CREATE TABLE IF NOT EXISTS series_articles (
    series_id TEXT NOT NULL REFERENCES series(id),
    article_uri TEXT NOT NULL REFERENCES articles(at_uri),
    PRIMARY KEY (series_id, article_uri)
);

CREATE INDEX IF NOT EXISTS idx_series_articles_series ON series_articles(series_id);

CREATE TABLE IF NOT EXISTS series_article_prereqs (
    series_id TEXT NOT NULL REFERENCES series(id),
    article_uri TEXT NOT NULL,
    prereq_article_uri TEXT NOT NULL,
    PRIMARY KEY (series_id, article_uri, prereq_article_uri)
);

-- Profiles
CREATE TABLE IF NOT EXISTS profiles (
    did TEXT PRIMARY KEY,
    handle TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    links TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Skill trees
CREATE TABLE IF NOT EXISTS skill_trees (
    at_uri TEXT PRIMARY KEY,
    did TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    field TEXT,
    forked_from TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skill_trees_did ON skill_trees(did);
CREATE INDEX IF NOT EXISTS idx_skill_trees_field ON skill_trees(field);

CREATE TABLE IF NOT EXISTS skill_tree_edges (
    tree_uri TEXT NOT NULL REFERENCES skill_trees(at_uri),
    parent_tag TEXT NOT NULL,
    child_tag TEXT NOT NULL,
    PRIMARY KEY (tree_uri, parent_tag, child_tag)
);

CREATE INDEX IF NOT EXISTS idx_skill_tree_edges_tree ON skill_tree_edges(tree_uri);

CREATE TABLE IF NOT EXISTS user_active_tree (
    did TEXT PRIMARY KEY,
    tree_uri TEXT NOT NULL REFERENCES skill_trees(at_uri)
);

-- Keybindings
CREATE TABLE IF NOT EXISTS user_keybindings (
    did TEXT PRIMARY KEY,
    bindings TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Follows
CREATE TABLE IF NOT EXISTS user_follows (
    did TEXT NOT NULL,
    follows_did TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (did, follows_did)
);

CREATE INDEX IF NOT EXISTS idx_user_follows_did ON user_follows(did);

CREATE TABLE IF NOT EXISTS follow_seen (
    did TEXT NOT NULL,
    follows_did TEXT NOT NULL,
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (did, follows_did)
);

-- Comments
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    article_uri TEXT NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    did TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_comments_article ON comments(article_uri);
CREATE INDEX IF NOT EXISTS idx_comments_did ON comments(did);

-- FTS5 full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS articles_fts USING fts5(
    title,
    content_hash,
    content='articles',
    content_rowid='rowid'
);
