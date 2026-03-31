-- Series redesign: unified repo architecture + heading-based article slicing
--
-- 1. Article can only belong to one series
ALTER TABLE series_articles
  ADD CONSTRAINT unique_article_in_series UNIQUE (article_uri);

-- 2. Split level for heading extraction (which heading level = article boundary)
ALTER TABLE series ADD COLUMN split_level INTEGER NOT NULL DEFAULT 1;

-- 3. Heading pointer fields on series_articles (for compile-generated articles)
ALTER TABLE series_articles ADD COLUMN heading_title TEXT;
ALTER TABLE series_articles ADD COLUMN heading_anchor TEXT;

-- 4. Full heading tree for TOC rendering
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
