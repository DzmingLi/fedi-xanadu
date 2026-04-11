-- Convert articles.category from enum to varchar for user-defined categories
ALTER TABLE articles
    ALTER COLUMN category TYPE VARCHAR(50) USING category::text;

ALTER TABLE articles
    ALTER COLUMN category SET DEFAULT 'general';

DROP TYPE IF EXISTS article_category;
