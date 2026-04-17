-- Backfill: set title from edition_name for editions that don't have one yet
UPDATE book_editions SET title = edition_name WHERE title IS NULL;
-- Make title required
ALTER TABLE book_editions ALTER COLUMN title SET NOT NULL;
ALTER TABLE book_editions ALTER COLUMN title SET DEFAULT '';
