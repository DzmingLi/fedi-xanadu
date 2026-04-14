-- Cache rendered HTML alongside source text in article_versions.
-- Used by thoughts (no pijul repo) to avoid re-rendering on every request.
ALTER TABLE article_versions ADD COLUMN rendered_html TEXT;
