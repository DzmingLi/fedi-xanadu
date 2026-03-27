-- Rename article_tags → article_teaches (semantic clarity: what an article teaches)
ALTER TABLE article_tags RENAME TO article_teaches;
ALTER INDEX idx_article_tags_tag RENAME TO idx_article_teaches_tag;

-- Learned marks: user declares they've learned an article's content
CREATE TABLE IF NOT EXISTS learned_marks (
    did VARCHAR(255) NOT NULL,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    learned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, article_uri)
);

CREATE INDEX IF NOT EXISTS idx_learned_marks_did ON learned_marks(did);
