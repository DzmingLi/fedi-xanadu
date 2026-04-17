-- Rename article/series/draft "description" → "summary": the field is a short
-- abstract shown at the top of the body and on card previews, not a generic
-- description. Books, tags, courses, events etc. keep their "description"
-- columns (those are genuine descriptions, not content summaries).
--
-- Also add summary_html: cache of inline-rendered HTML for list views so the
-- homepage can render card previews without reading from pijul.

ALTER TABLE articles RENAME COLUMN description TO summary;
ALTER TABLE articles ADD COLUMN summary_html TEXT NOT NULL DEFAULT '';

ALTER TABLE series RENAME COLUMN description TO summary;
ALTER TABLE series ADD COLUMN summary_html TEXT NOT NULL DEFAULT '';

ALTER TABLE drafts RENAME COLUMN description TO summary;
