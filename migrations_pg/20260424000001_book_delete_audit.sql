-- Allow book_edit_log entries to survive a hard DELETE of their book so the
-- audit trail (who deleted what, when, and what the metadata looked like)
-- remains queryable. Without this, DELETE FROM books cascades the log away
-- and we lose the one record the user cares about: the deletion itself.

ALTER TABLE book_edit_log DROP CONSTRAINT book_edit_log_book_id_fkey;
ALTER TABLE book_edit_log ALTER COLUMN book_id DROP NOT NULL;
ALTER TABLE book_edit_log
    ADD CONSTRAINT book_edit_log_book_id_fkey
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE SET NULL;

-- Preserve the original book id on every log row so post-deletion queries
-- (e.g. "show me all activity on bk-xxx, even though it's gone") keep
-- working. Backfilled from the current book_id; new rows should mirror
-- it. Left nullable so older INSERT statements that haven't been updated
-- to pass this column continue to work — they'll just leave it NULL and
-- those rows will not surface in post-delete history queries.
ALTER TABLE book_edit_log ADD COLUMN IF NOT EXISTS original_book_id VARCHAR(64);
UPDATE book_edit_log SET original_book_id = book_id WHERE original_book_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_book_edit_log_original_book_id
    ON book_edit_log(original_book_id);
