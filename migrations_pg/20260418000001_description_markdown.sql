-- Description becomes a rendered source field (markdown/typst, follows content_format).
-- description_html: cache of inline-rendered HTML for list views (no pijul read on homepage).

ALTER TABLE articles
    ADD COLUMN description_html TEXT NOT NULL DEFAULT '';

ALTER TABLE series
    ADD COLUMN description_html TEXT NOT NULL DEFAULT '';
