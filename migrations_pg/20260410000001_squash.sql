-- Squashed migration: consolidates all prior migrations into one canonical schema.
-- All 33 previous migrations have been applied to the production database.
-- New databases will be built from this single file.

-- Extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;

-- Enum types
CREATE TYPE content_kind AS ENUM ('article', 'question', 'answer');
CREATE TYPE content_format AS ENUM ('typst', 'markdown', 'html', 'tex');
CREATE TYPE article_category AS ENUM ('general', 'lecture', 'paper', 'review');

-- Functions
CREATE FUNCTION articles_search_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.search_vector := to_tsvector('simple', COALESCE(NEW.title, '') || ' ' || COALESCE(NEW.description, ''));
    RETURN NEW;
END;
$$;

CREATE FUNCTION content_insert_article() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO content (uri, content_type)
    VALUES (NEW.at_uri, NEW.kind)
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE FUNCTION content_insert_series() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO content (uri, content_type) VALUES (NEW.id, 'series') ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE FUNCTION content_delete_article() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM content WHERE uri = OLD.at_uri;
    RETURN OLD;
END;
$$;

CREATE FUNCTION content_delete_series() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM content WHERE uri = OLD.id;
    RETURN OLD;
END;
$$;

CREATE FUNCTION update_answer_count() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.kind = 'answer' AND NEW.question_uri IS NOT NULL THEN
        UPDATE articles SET answer_count = answer_count + 1 WHERE at_uri = NEW.question_uri;
    ELSIF TG_OP = 'DELETE' AND OLD.kind = 'answer' AND OLD.question_uri IS NOT NULL THEN
        UPDATE articles SET answer_count = answer_count - 1 WHERE at_uri = OLD.question_uri;
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$;

-- Tags
CREATE TABLE tags (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    names JSONB NOT NULL DEFAULT '{}'
);

-- Platform users
CREATE TABLE platform_users (
    did VARCHAR(255) PRIMARY KEY,
    handle VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(255),
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_banned BOOLEAN NOT NULL DEFAULT FALSE,
    banned_at TIMESTAMPTZ,
    ban_reason TEXT,
    phone VARCHAR(20) UNIQUE,
    phone_verified_at TIMESTAMPTZ,
    school VARCHAR(255),
    school_verified BOOLEAN NOT NULL DEFAULT FALSE,
    school_verified_at TIMESTAMPTZ,
    education JSONB NOT NULL DEFAULT '[]',
    affiliation VARCHAR(255),
    credentials_verified BOOLEAN NOT NULL DEFAULT FALSE,
    credentials_verified_at TIMESTAMPTZ
);

-- Profiles
CREATE TABLE profiles (
    did VARCHAR(255) PRIMARY KEY,
    handle VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    avatar_url TEXT,
    links TEXT NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    school VARCHAR(255),
    school_verified BOOLEAN NOT NULL DEFAULT FALSE,
    education JSONB NOT NULL DEFAULT '[]',
    affiliation VARCHAR(255),
    credentials_verified BOOLEAN NOT NULL DEFAULT FALSE,
    credentials_verified_at TIMESTAMPTZ
);

-- Sessions
CREATE TABLE sessions (
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

CREATE INDEX idx_sessions_did ON sessions(did);

-- Articles
CREATE TABLE articles (
    at_uri VARCHAR(512) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    content_hash VARCHAR(128),
    content_format content_format NOT NULL DEFAULT 'typst',
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    translation_group VARCHAR(512),
    license VARCHAR(100) NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    prereq_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    search_vector TSVECTOR,
    visibility VARCHAR(20) NOT NULL DEFAULT 'public',
    removed_at TIMESTAMPTZ,
    remove_reason TEXT,
    kind content_kind NOT NULL DEFAULT 'article',
    question_uri VARCHAR(512) REFERENCES articles(at_uri) ON DELETE CASCADE,
    answer_count INTEGER NOT NULL DEFAULT 0,
    restricted BOOLEAN NOT NULL DEFAULT FALSE,
    category article_category NOT NULL DEFAULT 'general',
    book_id VARCHAR(64),
    edition_id VARCHAR(64)
);

CREATE INDEX idx_articles_did ON articles(did);
CREATE INDEX idx_articles_translation_group ON articles(translation_group);
CREATE INDEX idx_articles_visibility ON articles(visibility);
CREATE INDEX idx_articles_kind ON articles(kind);
CREATE INDEX idx_articles_question ON articles(question_uri) WHERE question_uri IS NOT NULL;
CREATE INDEX idx_articles_book_id ON articles(book_id) WHERE book_id IS NOT NULL;
CREATE INDEX idx_articles_edition_id ON articles(edition_id) WHERE edition_id IS NOT NULL;
CREATE INDEX idx_articles_fts ON articles USING GIN(search_vector);
CREATE INDEX idx_articles_title_trgm ON articles USING GIN(title gin_trgm_ops);
CREATE INDEX idx_articles_description_trgm ON articles USING GIN(description gin_trgm_ops);

CREATE TRIGGER articles_search_trigger
    BEFORE INSERT OR UPDATE ON articles
    FOR EACH ROW EXECUTE FUNCTION articles_search_update();

CREATE TRIGGER trg_answer_count
    AFTER INSERT OR DELETE ON articles
    FOR EACH ROW EXECUTE FUNCTION update_answer_count();

-- Content identity table (unified URI registry)
CREATE TABLE content (
    uri TEXT PRIMARY KEY,
    content_type TEXT NOT NULL,
    CONSTRAINT content_content_type_check
        CHECK (content_type = ANY (ARRAY['article','series','question','answer','book','chapter']))
);

-- Populate content from articles
CREATE TRIGGER trg_article_content_insert
    AFTER INSERT ON articles FOR EACH ROW EXECUTE FUNCTION content_insert_article();

CREATE TRIGGER trg_article_content_delete
    BEFORE DELETE ON articles FOR EACH ROW EXECUTE FUNCTION content_delete_article();

-- Content tag relationships
CREATE TABLE content_teaches (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_teaches_tag ON content_teaches(tag_id);

CREATE TABLE content_topics (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_topics_tag ON content_topics(tag_id);

CREATE TABLE content_prereqs (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    prereq_type VARCHAR(50) NOT NULL DEFAULT 'required',
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_prereqs_tag ON content_prereqs(tag_id);

-- Series
CREATE TABLE series (
    id VARCHAR(255) PRIMARY KEY,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    order_index INTEGER NOT NULL DEFAULT 0,
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    translation_group VARCHAR(255),
    category VARCHAR(20) NOT NULL DEFAULT 'general',
    long_description TEXT,
    pijul_node_id VARCHAR(255),
    split_level INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_series_translation_group ON series(translation_group);

CREATE TRIGGER trg_series_content_insert
    AFTER INSERT ON series FOR EACH ROW EXECUTE FUNCTION content_insert_series();

CREATE TRIGGER trg_series_content_delete
    BEFORE DELETE ON series FOR EACH ROW EXECUTE FUNCTION content_delete_series();

CREATE TABLE series_articles (
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    order_index INTEGER NOT NULL DEFAULT 0,
    heading_title TEXT,
    heading_anchor TEXT,
    PRIMARY KEY (series_id, article_uri),
    CONSTRAINT unique_article_in_series UNIQUE (article_uri)
);

CREATE INDEX idx_series_articles_series ON series_articles(series_id);

CREATE TABLE series_article_prereqs (
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    article_uri VARCHAR(512) NOT NULL,
    prereq_article_uri VARCHAR(512) NOT NULL,
    PRIMARY KEY (series_id, article_uri, prereq_article_uri)
);

CREATE TABLE series_headings (
    id SERIAL PRIMARY KEY,
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    level INTEGER NOT NULL,
    title TEXT NOT NULL,
    anchor TEXT NOT NULL,
    article_uri VARCHAR(512) REFERENCES articles(at_uri) ON DELETE SET NULL,
    parent_heading_id INTEGER REFERENCES series_headings(id) ON DELETE CASCADE,
    order_index INTEGER NOT NULL DEFAULT 0,
    UNIQUE(series_id, anchor)
);

CREATE INDEX idx_series_headings_series ON series_headings(series_id);

-- Article version history
CREATE TABLE article_versions (
    id SERIAL PRIMARY KEY,
    article_uri VARCHAR(255) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    change_hash VARCHAR(64) NOT NULL,
    editor_did VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    source_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_article_versions_uri ON article_versions(article_uri, created_at DESC);

-- Drafts
CREATE TABLE drafts (
    id VARCHAR(255) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    content_format content_format NOT NULL DEFAULT 'typst',
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    license VARCHAR(100) NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    tags TEXT NOT NULL DEFAULT '[]',
    prereqs TEXT NOT NULL DEFAULT '[]',
    at_uri VARCHAR(512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_drafts_did ON drafts(did);

-- Votes
CREATE TABLE votes (
    at_uri VARCHAR(512) PRIMARY KEY,
    target_uri VARCHAR(512) NOT NULL,
    did VARCHAR(255) NOT NULL,
    value INTEGER NOT NULL,
    UNIQUE(target_uri, did)
);

CREATE INDEX idx_votes_target ON votes(target_uri);
CREATE INDEX idx_votes_did ON votes(did);

-- Bookmarks
CREATE TABLE user_bookmarks (
    did VARCHAR(255) NOT NULL,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    folder_path VARCHAR(255) NOT NULL DEFAULT '/',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, article_uri)
);

CREATE INDEX idx_bookmarks_did ON user_bookmarks(did);
CREATE INDEX idx_bookmarks_folder ON user_bookmarks(did, folder_path);

-- Comments
CREATE TABLE comments (
    id VARCHAR(255) PRIMARY KEY,
    content_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    parent_id VARCHAR(255) REFERENCES comments(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    quote_text TEXT
);

CREATE INDEX idx_comments_article_uri ON comments(content_uri);
CREATE INDEX idx_comments_did ON comments(did);
CREATE INDEX idx_comments_parent ON comments(parent_id);

CREATE TABLE comment_votes (
    comment_id VARCHAR(255) NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    value INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (comment_id, did)
);

CREATE INDEX idx_comment_votes_comment ON comment_votes(comment_id);

-- Skill trees
CREATE TABLE skill_trees (
    at_uri VARCHAR(512) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    tag_id VARCHAR(100),
    forked_from VARCHAR(512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_skill_trees_did ON skill_trees(did);
CREATE INDEX idx_skill_trees_tag_id ON skill_trees(tag_id);

CREATE TABLE skill_tree_edges (
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE,
    parent_tag VARCHAR(255) NOT NULL,
    child_tag VARCHAR(255) NOT NULL,
    PRIMARY KEY (tree_uri, parent_tag, child_tag)
);

CREATE INDEX idx_skill_tree_edges_tree ON skill_tree_edges(tree_uri);

CREATE TABLE user_active_tree (
    did VARCHAR(255) PRIMARY KEY,
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE
);

-- User skills & interests
CREATE TABLE user_skills (
    did VARCHAR(255) NOT NULL,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    status VARCHAR(50) NOT NULL DEFAULT 'mastered',
    lit_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, tag_id)
);

CREATE INDEX idx_user_skills_did ON user_skills(did);

CREATE TABLE user_interests (
    did VARCHAR(255) NOT NULL,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, tag_id)
);

CREATE TABLE user_tag_tree (
    did VARCHAR(255) NOT NULL,
    parent_tag VARCHAR(255) NOT NULL REFERENCES tags(id),
    child_tag VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (did, parent_tag, child_tag)
);

CREATE TABLE learned_marks (
    did VARCHAR(255) NOT NULL,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    learned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, article_uri)
);

CREATE INDEX idx_learned_marks_did ON learned_marks(did);

-- Social
CREATE TABLE user_follows (
    did VARCHAR(255) NOT NULL,
    follows_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, follows_did)
);

CREATE INDEX idx_user_follows_did ON user_follows(did);

CREATE TABLE follow_seen (
    did VARCHAR(255) NOT NULL,
    follows_did VARCHAR(255) NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, follows_did)
);

CREATE TABLE user_blocks (
    did VARCHAR(255) NOT NULL,
    blocked_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, blocked_did)
);

CREATE INDEX idx_user_blocks_did ON user_blocks(did);

CREATE TABLE user_members (
    author_did VARCHAR(255) NOT NULL,
    member_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (author_did, member_did)
);

CREATE INDEX idx_user_members_member ON user_members(member_did);

-- Forks
CREATE TABLE forks (
    fork_uri VARCHAR(512) PRIMARY KEY,
    source_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    forked_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    pijul_patch_hash VARCHAR(128),
    vote_score INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_forks_source_uri ON forks(source_uri);
CREATE INDEX idx_forks_forked_uri ON forks(forked_uri);

-- Notifications
CREATE TABLE notifications (
    id VARCHAR(255) PRIMARY KEY,
    recipient_did VARCHAR(255) NOT NULL,
    actor_did VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    target_uri VARCHAR(512),
    context_id VARCHAR(512),
    read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_recipient ON notifications(recipient_did, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications(recipient_did) WHERE read = FALSE;

-- Moderation & appeals
CREATE TABLE appeals (
    id VARCHAR(255) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    target_uri VARCHAR(512),
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    admin_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_appeals_did ON appeals(did, created_at DESC);
CREATE INDEX idx_appeals_status ON appeals(status) WHERE status = 'pending';

CREATE TABLE reports (
    id VARCHAR(64) PRIMARY KEY,
    reporter_did VARCHAR(255) NOT NULL,
    target_did VARCHAR(255) NOT NULL,
    target_uri VARCHAR(512),
    kind VARCHAR(32) NOT NULL,
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    admin_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_target ON reports(target_did);

-- Access control
CREATE TABLE article_access_grants (
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    grantee_did VARCHAR(255) NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (article_uri, grantee_did)
);

CREATE INDEX idx_access_grants_grantee ON article_access_grants(grantee_did);

-- User settings & keybindings
CREATE TABLE user_settings (
    did VARCHAR(255) PRIMARY KEY,
    native_lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    known_langs JSONB NOT NULL DEFAULT '["zh"]',
    prefer_native BOOLEAN NOT NULL DEFAULT TRUE,
    hide_unknown BOOLEAN NOT NULL DEFAULT FALSE,
    default_format VARCHAR(20) NOT NULL DEFAULT 'typst',
    email VARCHAR(255),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    bookmarks_public BOOLEAN NOT NULL DEFAULT FALSE,
    public_folders JSONB NOT NULL DEFAULT '[]'
);

CREATE TABLE user_keybindings (
    did VARCHAR(255) PRIMARY KEY,
    bindings TEXT NOT NULL DEFAULT '{}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Q&A
CREATE TABLE question_merges (
    from_uri VARCHAR(512) NOT NULL PRIMARY KEY,
    into_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri),
    merged_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Books
CREATE TABLE books (
    id VARCHAR(64) PRIMARY KEY,
    title VARCHAR(512) NOT NULL,
    authors TEXT[] NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '',
    cover_url VARCHAR(1024),
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE book_editions (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    title VARCHAR(512) NOT NULL,
    lang VARCHAR(10) NOT NULL DEFAULT 'en',
    isbn VARCHAR(20),
    publisher VARCHAR(255),
    year VARCHAR(10),
    translators TEXT[] NOT NULL DEFAULT '{}',
    purchase_links JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cover_url VARCHAR(1024)
);

CREATE INDEX idx_book_editions_book_id ON book_editions(book_id);

CREATE TABLE book_ratings (
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    rating SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (book_id, user_did)
);

CREATE TABLE book_reading_status (
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('want_to_read', 'reading', 'finished')),
    progress SMALLINT NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (book_id, user_did)
);

CREATE TABLE book_chapters (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    parent_id VARCHAR(64) REFERENCES book_chapters(id) ON DELETE CASCADE,
    title VARCHAR(512) NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    article_uri VARCHAR(512)
);

CREATE INDEX idx_book_chapters_book ON book_chapters(book_id);

CREATE TABLE book_chapter_progress (
    book_id VARCHAR(64) NOT NULL,
    chapter_id VARCHAR(64) NOT NULL REFERENCES book_chapters(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (chapter_id, user_did)
);

CREATE INDEX idx_chapter_progress_user ON book_chapter_progress(user_did, book_id);

CREATE TABLE book_edit_log (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    editor_did VARCHAR(255) NOT NULL,
    old_data JSONB NOT NULL DEFAULT '{}',
    new_data JSONB NOT NULL DEFAULT '{}',
    summary TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_book_edit_log_book_id ON book_edit_log(book_id);
