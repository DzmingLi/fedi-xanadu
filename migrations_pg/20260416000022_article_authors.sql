-- Article authorship with verification status.
-- Each author must verify their authorship (via PDS record or platform confirmation).
CREATE TABLE article_authors (
    article_uri  VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    author_did   VARCHAR(255) NOT NULL,
    position     SMALLINT,                          -- NULL = unordered, 0/1/2... = ordered
    status       VARCHAR(20) NOT NULL DEFAULT 'pending',  -- 'pending', 'verified', 'rejected'
    authorship_uri VARCHAR(512),                    -- AT Protocol authorship record URI (verification proof)
    added_by     VARCHAR(255) NOT NULL,             -- DID of who listed this author
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified_at  TIMESTAMPTZ,
    PRIMARY KEY (article_uri, author_did)
);

CREATE INDEX idx_article_authors_did ON article_authors(author_did);
CREATE INDEX idx_article_authors_status ON article_authors(article_uri, status);

-- Backfill: existing articles get their creator as a verified author (position 0).
INSERT INTO article_authors (article_uri, author_did, position, status, added_by, verified_at)
SELECT at_uri, did, 0, 'verified', did, created_at
FROM articles
ON CONFLICT DO NOTHING;
