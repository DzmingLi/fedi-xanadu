-- Convert VARCHAR columns to native PostgreSQL enum types.
-- Idempotent: safe to re-run if types already exist.

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'content_kind') THEN
        CREATE TYPE content_kind AS ENUM ('article', 'question', 'answer');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'content_format') THEN
        CREATE TYPE content_format AS ENUM ('typst', 'markdown', 'html', 'tex');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'article_category') THEN
        CREATE TYPE article_category AS ENUM ('general', 'lecture', 'paper', 'review');
    END IF;
END $$;

-- Only alter if still varchar (idempotent)
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'articles' AND column_name = 'kind' AND data_type = 'character varying'
    ) THEN
        ALTER TABLE articles ALTER COLUMN kind DROP DEFAULT;
        ALTER TABLE articles ALTER COLUMN content_format DROP DEFAULT;
        ALTER TABLE articles ALTER COLUMN category DROP DEFAULT;

        ALTER TABLE articles
            ALTER COLUMN kind TYPE content_kind USING kind::content_kind,
            ALTER COLUMN content_format TYPE content_format USING content_format::content_format,
            ALTER COLUMN category TYPE article_category USING category::article_category;

        ALTER TABLE articles ALTER COLUMN kind SET DEFAULT 'article'::content_kind;
        ALTER TABLE articles ALTER COLUMN content_format SET DEFAULT 'typst'::content_format;
        ALTER TABLE articles ALTER COLUMN category SET DEFAULT 'general'::article_category;
    END IF;
END $$;

DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'drafts' AND column_name = 'content_format' AND data_type = 'character varying'
    ) THEN
        ALTER TABLE drafts ALTER COLUMN content_format DROP DEFAULT;
        ALTER TABLE drafts ALTER COLUMN content_format TYPE content_format USING content_format::content_format;
        ALTER TABLE drafts ALTER COLUMN content_format SET DEFAULT 'typst'::content_format;
    END IF;
END $$;
