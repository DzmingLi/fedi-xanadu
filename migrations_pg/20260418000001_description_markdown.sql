-- Description becomes a rendered source field (markdown/typst, follows content_format).
-- description_html: cache of inline-rendered HTML for list views (no pijul read on homepage).
-- auto_description: when true, description source is auto-extracted from content at publish.

ALTER TABLE articles
    ADD COLUMN description_html TEXT NOT NULL DEFAULT '',
    ADD COLUMN auto_description BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE series
    ADD COLUMN description_html TEXT NOT NULL DEFAULT '',
    ADD COLUMN auto_description BOOLEAN NOT NULL DEFAULT TRUE;
