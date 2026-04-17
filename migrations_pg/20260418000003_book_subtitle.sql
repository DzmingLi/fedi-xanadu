-- Books get their own locale-keyed subtitle (separate from per-edition subtitles).
-- Editions can still override via book_editions.subtitle when the localized
-- wording differs between printings/translations; this column is the default.
ALTER TABLE books ADD COLUMN IF NOT EXISTS subtitle JSONB NOT NULL DEFAULT '{}';
