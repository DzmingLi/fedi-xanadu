-- The articles full-text search trigger still references NEW.description,
-- which was renamed to NEW.summary in 20260418000001. Every UPDATE on the
-- articles table therefore 500s with:
--   record "new" has no field "description"
-- Fix the trigger function so it uses the new column name.
CREATE OR REPLACE FUNCTION articles_search_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector := to_tsvector('simple', COALESCE(NEW.title, '') || ' ' || COALESCE(NEW.summary, ''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
