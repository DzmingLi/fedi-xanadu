-- Widen book_edit_log to cover chapter (TOC) edits in addition to
-- book-metadata edits. Before this migration only title/subtitle/
-- description/abbreviation changes were audit-logged; chapter
-- create/update/delete slipped through silently.
--
-- `action` discriminates what the row describes. Existing rows all
-- describe book metadata edits, so the default backfills them.
-- `target_id` points at the affected sub-entity (currently chapter_id
-- for chapter_* actions, NULL for book_update).

ALTER TABLE book_edit_log
    ADD COLUMN IF NOT EXISTS action TEXT NOT NULL DEFAULT 'book_update';

ALTER TABLE book_edit_log
    ADD COLUMN IF NOT EXISTS target_id VARCHAR(64);

CREATE INDEX IF NOT EXISTS idx_book_edit_log_original_book_created
    ON book_edit_log(original_book_id, created_at DESC);
