-- Allow article authors without a NightBoat account (just a name string)
ALTER TABLE article_authors ALTER COLUMN author_did DROP NOT NULL;
ALTER TABLE article_authors ADD COLUMN IF NOT EXISTS author_name VARCHAR(255);
-- At least one of author_did or author_name should be set
ALTER TABLE article_authors DROP CONSTRAINT IF EXISTS article_authors_pkey;
ALTER TABLE article_authors ADD COLUMN IF NOT EXISTS id SERIAL;
ALTER TABLE article_authors ADD CONSTRAINT article_authors_pkey PRIMARY KEY (id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_article_authors_unique_did ON article_authors(article_uri, author_did) WHERE author_did IS NOT NULL;
