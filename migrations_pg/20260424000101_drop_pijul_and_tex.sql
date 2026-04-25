-- Final cleanup of the pijul + tex removal.
--
-- Preconditions: every `article_localizations` row has been flipped off
-- `content_storage = 'pijul'` via the admin /migrate-pijul-to-blob endpoint,
-- and every row's `content_format` is now in {'typst','markdown','html'}. If
-- any rows still hold 'pijul' or 'tex', this migration will fail at the CHECK
-- / enum cutover — that's intentional.

-- 1. content_storage: drop 'pijul' from the allowed set.
ALTER TABLE article_localizations
    DROP CONSTRAINT IF EXISTS localizations_storage_valid;
ALTER TABLE article_localizations
    ADD CONSTRAINT localizations_storage_valid
    CHECK (content_storage IN ('blob', 'server_db'));

-- 2. series.pijul_node_id is obsolete under the blob-bundle model.
ALTER TABLE series DROP COLUMN IF EXISTS pijul_node_id;

-- 3. content_format ENUM: remove 'tex'. Postgres does not allow removing a
--    value from an enum in place, so we rebuild the type. Every column
--    using the old type must be migrated before the old type can be dropped.
--    Pre-flight: rewrite any 'tex' rows to 'markdown' (closest plain-text
--    family); we no longer support tex rendering.
UPDATE article_localizations SET content_format = 'markdown' WHERE content_format::text = 'tex';
UPDATE drafts                SET content_format = 'markdown' WHERE content_format::text = 'tex';
UPDATE articles_legacy       SET content_format = 'markdown' WHERE content_format::text = 'tex';

CREATE TYPE content_format_new AS ENUM ('typst', 'markdown', 'html');

ALTER TABLE article_localizations
    ALTER COLUMN content_format DROP DEFAULT;
ALTER TABLE article_localizations
    ALTER COLUMN content_format TYPE content_format_new
    USING content_format::text::content_format_new;
ALTER TABLE article_localizations
    ALTER COLUMN content_format SET DEFAULT 'typst'::content_format_new;

ALTER TABLE drafts
    ALTER COLUMN content_format DROP DEFAULT;
ALTER TABLE drafts
    ALTER COLUMN content_format TYPE content_format_new
    USING content_format::text::content_format_new;
ALTER TABLE drafts
    ALTER COLUMN content_format SET DEFAULT 'typst'::content_format_new;

ALTER TABLE articles_legacy
    ALTER COLUMN content_format DROP DEFAULT;
ALTER TABLE articles_legacy
    ALTER COLUMN content_format TYPE content_format_new
    USING content_format::text::content_format_new;

DROP TYPE content_format;
ALTER TYPE content_format_new RENAME TO content_format;
