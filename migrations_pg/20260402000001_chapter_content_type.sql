-- Allow 'chapter' as a content_type so chapters can have content_teaches/content_prereqs
ALTER TABLE content DROP CONSTRAINT IF EXISTS content_content_type_check;
ALTER TABLE content ADD CONSTRAINT content_content_type_check
  CHECK (content_type = ANY (ARRAY['article','series','question','answer','book','chapter']));
