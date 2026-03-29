-- Remove 'tex' from content_format enum (no articles use it, server rejects tex uploads)
ALTER TYPE content_format RENAME TO content_format_old;
CREATE TYPE content_format AS ENUM ('typst', 'markdown', 'html');
ALTER TABLE articles ALTER COLUMN content_format TYPE content_format USING content_format::text::content_format;
ALTER TABLE drafts ALTER COLUMN content_format TYPE content_format USING content_format::text::content_format;
DROP TYPE content_format_old;
