-- Optional cover image for articles and series. Rendered as a small
-- thumbnail on the right side of cards; null = no image (no placeholder).
ALTER TABLE articles ADD COLUMN IF NOT EXISTS cover_url TEXT;
ALTER TABLE series   ADD COLUMN IF NOT EXISTS cover_url TEXT;
