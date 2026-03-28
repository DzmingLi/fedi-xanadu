-- Convert VARCHAR columns to native PostgreSQL enum types.
-- This ensures Rust sqlx::Type enums decode correctly from the DB.

-- 1. Create enum types
CREATE TYPE content_kind AS ENUM ('article', 'question', 'answer');
CREATE TYPE content_format AS ENUM ('typst', 'markdown', 'html', 'tex');
CREATE TYPE article_category AS ENUM ('general', 'lecture', 'paper', 'review');

-- 2. Drop defaults before type conversion (PG cannot auto-cast varchar defaults)
ALTER TABLE articles ALTER COLUMN kind DROP DEFAULT;
ALTER TABLE articles ALTER COLUMN content_format DROP DEFAULT;
ALTER TABLE articles ALTER COLUMN category DROP DEFAULT;

-- 3. Convert articles columns
ALTER TABLE articles
    ALTER COLUMN kind TYPE content_kind USING kind::content_kind,
    ALTER COLUMN content_format TYPE content_format USING content_format::content_format,
    ALTER COLUMN category TYPE article_category USING category::article_category;

-- 4. Restore defaults as enum values
ALTER TABLE articles ALTER COLUMN kind SET DEFAULT 'article'::content_kind;
ALTER TABLE articles ALTER COLUMN content_format SET DEFAULT 'typst'::content_format;
ALTER TABLE articles ALTER COLUMN category SET DEFAULT 'general'::article_category;

-- 5. Convert drafts.content_format (check for default first)
ALTER TABLE drafts ALTER COLUMN content_format DROP DEFAULT;
ALTER TABLE drafts
    ALTER COLUMN content_format TYPE content_format USING content_format::content_format;
ALTER TABLE drafts ALTER COLUMN content_format SET DEFAULT 'typst'::content_format;
