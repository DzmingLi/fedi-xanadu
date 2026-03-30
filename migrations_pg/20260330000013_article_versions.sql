-- Article version history: stores source snapshot + metadata for each pijul record
CREATE TABLE IF NOT EXISTS article_versions (
    id SERIAL PRIMARY KEY,
    article_uri VARCHAR(255) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    change_hash VARCHAR(64) NOT NULL,
    editor_did VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    source_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_article_versions_uri ON article_versions(article_uri, created_at DESC);
