-- OAuth sessions for AT Protocol users (replaces createSession password login)
CREATE TABLE IF NOT EXISTS oauth_sessions (
    token VARCHAR(128) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    handle VARCHAR(255),
    pds_url VARCHAR(512),
    access_token TEXT,
    refresh_token TEXT,
    dpop_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_oauth_sessions_did ON oauth_sessions(did);
