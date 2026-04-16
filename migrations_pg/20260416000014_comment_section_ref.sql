-- Allow comments to reference a specific section (e.g. course session, book chapter)
ALTER TABLE comments ADD COLUMN IF NOT EXISTS section_ref VARCHAR(255);
CREATE INDEX IF NOT EXISTS idx_comments_section_ref ON comments(content_uri, section_ref);
