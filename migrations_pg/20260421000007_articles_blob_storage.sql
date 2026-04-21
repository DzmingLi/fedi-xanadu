-- Add blob-backed content storage for articles whose license forbids collaborative (pijul) hosting.
--
-- `content_storage` selects the backend: 'pijul' (default, legacy) or 'blob' (source files live as
-- blobs on the author's PDS). `content_manifest` carries the per-file CID map when storage='blob',
-- shape: {"entry": "content.typ", "files": [{"path": "...", "cid": "bafkr...", "size": 12345,
-- "mime": "application/pdf"}, ...]}.

ALTER TABLE articles
  ADD COLUMN content_storage VARCHAR(16) NOT NULL DEFAULT 'pijul',
  ADD COLUMN content_manifest JSONB;

ALTER TABLE articles
  ADD CONSTRAINT articles_content_storage_valid
    CHECK (content_storage IN ('pijul', 'blob')),
  ADD CONSTRAINT articles_blob_manifest_required
    CHECK (content_storage <> 'blob' OR content_manifest IS NOT NULL);
