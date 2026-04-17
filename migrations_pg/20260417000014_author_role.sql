-- Author role: 'author' (default), 'translator', 'editor', etc.
ALTER TABLE article_authors ADD COLUMN IF NOT EXISTS role VARCHAR(30) NOT NULL DEFAULT 'author';
