-- Book cover is derived from editions, not stored on the book itself.
ALTER TABLE books DROP COLUMN IF EXISTS cover_url;
