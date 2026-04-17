-- Rename edition title → edition_name, add proper title field
ALTER TABLE book_editions RENAME COLUMN title TO edition_name;
ALTER TABLE book_editions ADD COLUMN IF NOT EXISTS title VARCHAR(500);
