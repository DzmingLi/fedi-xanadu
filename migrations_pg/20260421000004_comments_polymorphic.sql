-- Comments were originally tied to articles (comments.content_uri FK →
-- articles.at_uri). Book/chapter/series "comments" have been written with
-- pseudo-URIs (series:id, discussion:id, etc.) that technically violated the
-- FK. Move the FK to the polymorphic content table so all entity types can
-- host comments cleanly.

-- Clean out any comments whose content_uri has no corresponding content row.
-- The preceding migration (20260421000003_book_series.sql) backfilled content
-- for existing books/chapters, so the only comments this will drop are ones
-- pointing at entities that no longer exist.
DELETE FROM comments
 WHERE content_uri NOT IN (SELECT uri FROM content);

ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_article_uri_fkey;
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_content_uri_fkey;
ALTER TABLE comments
    ADD CONSTRAINT comments_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;

-- Legacy index name still reads "article_uri"; rename to reflect what it
-- actually indexes now.
ALTER INDEX IF EXISTS idx_comments_article_uri RENAME TO idx_comments_content_uri;
