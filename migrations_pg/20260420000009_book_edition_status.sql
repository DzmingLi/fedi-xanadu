-- Mark an edition as a draft (includes preprints / self-published drafts
-- that don't have an ISBN and aren't finalized). Existing editions default
-- to 'published' so nothing changes for them.

ALTER TABLE book_editions
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'published';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'book_editions_status_check'
    ) THEN
        ALTER TABLE book_editions
            ADD CONSTRAINT book_editions_status_check
            CHECK (status IN ('draft', 'published'));
    END IF;
END $$;
