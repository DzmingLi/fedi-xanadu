-- Article/series category: general, lecture, paper, review
ALTER TABLE articles ADD COLUMN IF NOT EXISTS category VARCHAR(20) NOT NULL DEFAULT 'general';
ALTER TABLE series ADD COLUMN IF NOT EXISTS category VARCHAR(20) NOT NULL DEFAULT 'general';

-- Book reference for reviews
ALTER TABLE articles ADD COLUMN IF NOT EXISTS book_id VARCHAR(64);

-- Books
CREATE TABLE IF NOT EXISTS books (
    id VARCHAR(64) PRIMARY KEY,
    title VARCHAR(512) NOT NULL,
    authors TEXT[] NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '',
    cover_url VARCHAR(1024),
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Book editions (translations, reprints, etc.)
CREATE TABLE IF NOT EXISTS book_editions (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    title VARCHAR(512) NOT NULL,
    lang VARCHAR(10) NOT NULL DEFAULT 'en',
    isbn VARCHAR(20),
    publisher VARCHAR(255),
    year VARCHAR(10),
    translators TEXT[] NOT NULL DEFAULT '{}',
    purchase_links JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_book_editions_book_id ON book_editions(book_id);

-- Book tags (reuse content_teaches)
-- Books will use content_teaches with content_uri = 'book:<id>'

-- FK for reviews -> books (soft, no constraint since book_id is optional)
CREATE INDEX IF NOT EXISTS idx_articles_book_id ON articles(book_id) WHERE book_id IS NOT NULL;

-- Edit history for books (wiki-style collaborative editing)
CREATE TABLE IF NOT EXISTS book_edit_log (
    id VARCHAR(64) PRIMARY KEY,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    editor_did VARCHAR(255) NOT NULL,
    old_data JSONB NOT NULL DEFAULT '{}',
    new_data JSONB NOT NULL DEFAULT '{}',
    summary TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_book_edit_log_book_id ON book_edit_log(book_id);
