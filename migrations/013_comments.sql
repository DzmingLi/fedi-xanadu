CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    article_uri TEXT NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    did TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_comments_article ON comments(article_uri);
CREATE INDEX IF NOT EXISTS idx_comments_did ON comments(did);
