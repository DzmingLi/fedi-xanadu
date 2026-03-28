-- Edition-specific cover images
ALTER TABLE book_editions ADD COLUMN IF NOT EXISTS cover_url VARCHAR(1024);

-- Link reviews to specific editions
ALTER TABLE articles ADD COLUMN IF NOT EXISTS edition_id VARCHAR(64);
CREATE INDEX IF NOT EXISTS idx_articles_edition_id ON articles(edition_id) WHERE edition_id IS NOT NULL;
