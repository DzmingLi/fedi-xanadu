ALTER TABLE books ADD COLUMN IF NOT EXISTS abbreviation VARCHAR(50);
CREATE INDEX IF NOT EXISTS idx_books_abbreviation ON books(abbreviation) WHERE abbreviation IS NOT NULL;
