-- Cover image by-reference: the relative path inside the pijul repo to
-- use as the cover. NULL keeps the current scan-for-cover.{ext} fallback.
-- Lets authors reuse a body image as the cover without re-uploading it.
ALTER TABLE articles ADD COLUMN IF NOT EXISTS cover_file TEXT;
ALTER TABLE series   ADD COLUMN IF NOT EXISTS cover_file TEXT;
