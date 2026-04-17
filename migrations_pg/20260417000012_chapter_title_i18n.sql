-- Convert chapter title from plain text to JSONB for i18n support.
-- Existing titles are wrapped as {"en": "..."} (or {"zh": "..."} for Chinese).
ALTER TABLE book_chapters ADD COLUMN IF NOT EXISTS title_i18n JSONB;

-- Migrate existing titles: detect language by checking if title contains CJK characters
UPDATE book_chapters SET title_i18n =
  CASE WHEN title ~ '[\x{4e00}-\x{9fff}]' THEN jsonb_build_object('zh', title)
  ELSE jsonb_build_object('en', title)
  END
WHERE title_i18n IS NULL;

ALTER TABLE book_chapters ALTER COLUMN title_i18n SET NOT NULL;
ALTER TABLE book_chapters ALTER COLUMN title_i18n SET DEFAULT '{}';
