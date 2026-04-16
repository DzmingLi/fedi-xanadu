-- Convert books.title and books.description to JSONB locale maps.
-- Migrate existing data: wrap plain text as {"en": "..."}

-- Title
ALTER TABLE books ADD COLUMN IF NOT EXISTS title_l JSONB;
UPDATE books SET title_l = jsonb_build_object('en', title) WHERE title_l IS NULL;
ALTER TABLE books DROP COLUMN title;
ALTER TABLE books RENAME COLUMN title_l TO title;
ALTER TABLE books ALTER COLUMN title SET NOT NULL;
ALTER TABLE books ALTER COLUMN title SET DEFAULT '{}';

-- Description
ALTER TABLE books ADD COLUMN IF NOT EXISTS description_l JSONB;
UPDATE books SET description_l = jsonb_build_object('en', description) WHERE description_l IS NULL;
ALTER TABLE books DROP COLUMN description;
ALTER TABLE books RENAME COLUMN description_l TO description;
ALTER TABLE books ALTER COLUMN description SET NOT NULL;
ALTER TABLE books ALTER COLUMN description SET DEFAULT '{}';
