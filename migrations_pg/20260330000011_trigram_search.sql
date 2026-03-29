-- Enable trigram extension for CJK fuzzy search (ships with PostgreSQL)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram GIN indexes on title and description for Chinese/fuzzy matching
CREATE INDEX IF NOT EXISTS idx_articles_title_trgm
  ON articles USING GIN(title gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_articles_description_trgm
  ON articles USING GIN(description gin_trgm_ops);
