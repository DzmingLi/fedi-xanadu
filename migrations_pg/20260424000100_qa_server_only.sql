-- Q&A goes server-only: questions and answers are NightBoat-local, not
-- published to PDS and not stored in pijul. This lets admins freely merge
-- duplicate questions (cross-author edits a pijul repo can't cleanly express).
--
-- Storage changes:
--   1. `article_localizations.content_storage` gains a third value,
--      'server_db', meaning "content is stored inline in content_body".
--   2. `article_localizations.content_body` is a new nullable TEXT column
--      that holds the source for server-stored localizations.
--   3. For Q&A, `repo_uri` is a synthetic `server://qa/{rkey}`
--      (not an at_uri), `at_uri` on the localization is NULL, and
--      `content_storage = 'server_db'`.
--
-- No pijul repo is created for Q&A; no ATProto record is published.
-- The `question_merges` / `merge_questions` flow stays the same but now
-- operates on rows whose identities are purely internal.

ALTER TABLE article_localizations
    DROP CONSTRAINT IF EXISTS localizations_storage_valid;

ALTER TABLE article_localizations
    ADD CONSTRAINT localizations_storage_valid
    CHECK (content_storage IN ('pijul', 'blob', 'server_db'));

ALTER TABLE article_localizations
    ADD COLUMN IF NOT EXISTS content_body TEXT;

-- server_db rows MUST have content_body; pijul/blob rows should leave it NULL.
ALTER TABLE article_localizations
    ADD CONSTRAINT localizations_server_db_body_required
    CHECK (content_storage <> 'server_db' OR content_body IS NOT NULL);
