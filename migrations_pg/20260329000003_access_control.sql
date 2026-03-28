-- Access grants for restricted (paywalled) articles
CREATE TABLE IF NOT EXISTS article_access_grants (
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    grantee_did VARCHAR(255) NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (article_uri, grantee_did)
);

CREATE INDEX IF NOT EXISTS idx_access_grants_grantee ON article_access_grants(grantee_did);

-- Restricted flag on articles
ALTER TABLE articles ADD COLUMN IF NOT EXISTS restricted BOOLEAN NOT NULL DEFAULT FALSE;
