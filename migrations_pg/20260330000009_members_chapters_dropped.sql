-- 1. Member system: user-level access grants for restricted content
CREATE TABLE IF NOT EXISTS user_members (
    author_did VARCHAR(255) NOT NULL,
    member_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (author_did, member_did)
);
CREATE INDEX IF NOT EXISTS idx_user_members_member ON user_members(member_did);

-- 2. Book chapters (table of contents)
CREATE TABLE IF NOT EXISTS book_chapters (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    parent_id VARCHAR(64) REFERENCES book_chapters(id) ON DELETE CASCADE,
    title VARCHAR(512) NOT NULL,
    order_index INT NOT NULL DEFAULT 0,
    article_uri VARCHAR(512)
);
CREATE INDEX IF NOT EXISTS idx_book_chapters_book ON book_chapters(book_id);

-- 3. Per-chapter reading progress
CREATE TABLE IF NOT EXISTS book_chapter_progress (
    book_id VARCHAR(64) NOT NULL,
    chapter_id VARCHAR(64) NOT NULL REFERENCES book_chapters(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (chapter_id, user_did)
);
CREATE INDEX IF NOT EXISTS idx_chapter_progress_user ON book_chapter_progress(user_did, book_id);
