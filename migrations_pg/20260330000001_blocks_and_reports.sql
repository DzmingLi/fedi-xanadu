-- User blocks: blocker sees no content from blocked_did
CREATE TABLE IF NOT EXISTS user_blocks (
    did VARCHAR(255) NOT NULL,
    blocked_did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (did, blocked_did)
);

CREATE INDEX IF NOT EXISTS idx_user_blocks_did ON user_blocks(did);

-- Reports: submitted by users, reviewed by admins
CREATE TABLE IF NOT EXISTS reports (
    id VARCHAR(64) PRIMARY KEY,
    reporter_did VARCHAR(255) NOT NULL,
    target_did VARCHAR(255) NOT NULL,
    target_uri VARCHAR(512),
    kind VARCHAR(32) NOT NULL,  -- 'user', 'article', 'comment'
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- 'pending', 'resolved', 'dismissed'
    admin_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status);
CREATE INDEX IF NOT EXISTS idx_reports_target ON reports(target_did);
