-- Fedi-Xanadu PostgreSQL schema

-- Global tags
CREATE TABLE IF NOT EXISTS tags (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Articles
CREATE TABLE IF NOT EXISTS articles (
    at_uri VARCHAR(512) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    content_hash VARCHAR(128),
    content_format VARCHAR(50) NOT NULL DEFAULT 'typst',
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    translation_group VARCHAR(512),
    license VARCHAR(100) NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    prereq_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_articles_did ON articles(did);
CREATE INDEX IF NOT EXISTS idx_articles_translation_group ON articles(translation_group);

-- User private tag nesting
CREATE TABLE IF NOT EXISTS user_tag_tree (
    did VARCHAR(255) NOT NULL,
    parent_tag VARCHAR(255) NOT NULL REFERENCES tags(id),
    child_tag VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, parent_tag, child_tag)
);

-- Article-tag association
CREATE TABLE IF NOT EXISTS article_tags (
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (article_uri, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_article_tags_tag ON article_tags(tag_id);

-- Article prereqs
CREATE TABLE IF NOT EXISTS article_prereqs (
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    prereq_type VARCHAR(50) NOT NULL DEFAULT 'required',
    PRIMARY KEY (article_uri, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_article_prereqs_tag ON article_prereqs(tag_id);

-- Fork graph
CREATE TABLE IF NOT EXISTS forks (
    fork_uri VARCHAR(512) PRIMARY KEY,
    source_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    forked_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    pijul_patch_hash VARCHAR(128),
    vote_score INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_forks_source_uri ON forks(source_uri);
CREATE INDEX IF NOT EXISTS idx_forks_forked_uri ON forks(forked_uri);

-- User skills
CREATE TABLE IF NOT EXISTS user_skills (
    did VARCHAR(255) NOT NULL,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    status VARCHAR(50) NOT NULL DEFAULT 'mastered',
    lit_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_user_skills_did ON user_skills(did);

-- Votes
CREATE TABLE IF NOT EXISTS votes (
    at_uri VARCHAR(512) PRIMARY KEY,
    target_uri VARCHAR(512) NOT NULL,
    did VARCHAR(255) NOT NULL,
    value INTEGER NOT NULL,
    UNIQUE(target_uri, did)
);

CREATE INDEX IF NOT EXISTS idx_votes_target ON votes(target_uri);
CREATE INDEX IF NOT EXISTS idx_votes_did ON votes(did);

-- Bookmarks
CREATE TABLE IF NOT EXISTS user_bookmarks (
    did VARCHAR(255) NOT NULL,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    folder_path VARCHAR(255) NOT NULL DEFAULT '/',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, article_uri)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_did ON user_bookmarks(did);
CREATE INDEX IF NOT EXISTS idx_bookmarks_folder ON user_bookmarks(did, folder_path);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    token VARCHAR(128) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    handle VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    avatar_url TEXT,
    pds_url TEXT NOT NULL,
    access_jwt TEXT NOT NULL,
    refresh_jwt TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '7 days'
);

CREATE INDEX IF NOT EXISTS idx_sessions_did ON sessions(did);

-- User interests
CREATE TABLE IF NOT EXISTS user_interests (
    did VARCHAR(255) NOT NULL,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, tag_id)
);

-- Series
CREATE TABLE IF NOT EXISTS series (
    id VARCHAR(255) PRIMARY KEY,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_series_tag ON series(tag_id);

CREATE TABLE IF NOT EXISTS series_articles (
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    PRIMARY KEY (series_id, article_uri)
);

CREATE INDEX IF NOT EXISTS idx_series_articles_series ON series_articles(series_id);

CREATE TABLE IF NOT EXISTS series_article_prereqs (
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    article_uri VARCHAR(512) NOT NULL,
    prereq_article_uri VARCHAR(512) NOT NULL,
    PRIMARY KEY (series_id, article_uri, prereq_article_uri)
);

-- Profiles
CREATE TABLE IF NOT EXISTS profiles (
    did VARCHAR(255) PRIMARY KEY,
    handle VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    avatar_url TEXT,
    links TEXT NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Skill trees
CREATE TABLE IF NOT EXISTS skill_trees (
    at_uri VARCHAR(512) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    field VARCHAR(100),
    forked_from VARCHAR(512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_skill_trees_did ON skill_trees(did);
CREATE INDEX IF NOT EXISTS idx_skill_trees_field ON skill_trees(field);

CREATE TABLE IF NOT EXISTS skill_tree_edges (
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE,
    parent_tag VARCHAR(255) NOT NULL,
    child_tag VARCHAR(255) NOT NULL,
    PRIMARY KEY (tree_uri, parent_tag, child_tag)
);

CREATE INDEX IF NOT EXISTS idx_skill_tree_edges_tree ON skill_tree_edges(tree_uri);

CREATE TABLE IF NOT EXISTS user_active_tree (
    did VARCHAR(255) PRIMARY KEY,
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE
);

-- Keybindings
CREATE TABLE IF NOT EXISTS user_keybindings (
    did VARCHAR(255) PRIMARY KEY,
    bindings TEXT NOT NULL DEFAULT '{}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Follows
CREATE TABLE IF NOT EXISTS user_follows (
    did VARCHAR(255) NOT NULL,
    follows_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, follows_did)
);

CREATE INDEX IF NOT EXISTS idx_user_follows_did ON user_follows(did);

CREATE TABLE IF NOT EXISTS follow_seen (
    did VARCHAR(255) NOT NULL,
    follows_did VARCHAR(255) NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, follows_did)
);

-- Comments
CREATE TABLE IF NOT EXISTS comments (
    id VARCHAR(255) PRIMARY KEY,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    parent_id VARCHAR(255) REFERENCES comments(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_comments_article_uri ON comments(article_uri);
CREATE INDEX IF NOT EXISTS idx_comments_did ON comments(did);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_id);

-- Comment votes
CREATE TABLE IF NOT EXISTS comment_votes (
    comment_id VARCHAR(255) NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    value INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (comment_id, did)
);

CREATE INDEX IF NOT EXISTS idx_comment_votes_comment ON comment_votes(comment_id);

-- Drafts
CREATE TABLE IF NOT EXISTS drafts (
    id VARCHAR(255) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    content_format VARCHAR(50) NOT NULL DEFAULT 'typst',
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    license VARCHAR(100) NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    tags TEXT NOT NULL DEFAULT '[]',
    prereqs TEXT NOT NULL DEFAULT '[]',
    at_uri VARCHAR(512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_drafts_did ON drafts(did);

-- Full-text search using PostgreSQL tsvector + GIN
ALTER TABLE articles ADD COLUMN IF NOT EXISTS search_vector tsvector;

CREATE INDEX IF NOT EXISTS idx_articles_fts ON articles USING GIN(search_vector);

-- Trigger to auto-update search vector
CREATE OR REPLACE FUNCTION articles_search_update() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector := to_tsvector('simple', COALESCE(NEW.title, '') || ' ' || COALESCE(NEW.description, ''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS articles_search_trigger ON articles;
CREATE TRIGGER articles_search_trigger
    BEFORE INSERT OR UPDATE ON articles
    FOR EACH ROW EXECUTE FUNCTION articles_search_update();
