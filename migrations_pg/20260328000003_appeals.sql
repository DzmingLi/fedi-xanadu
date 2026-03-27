-- Soft-delete support for articles
ALTER TABLE articles
  ADD COLUMN deleted_at   TIMESTAMPTZ,
  ADD COLUMN delete_reason TEXT;

-- Appeal system
CREATE TABLE IF NOT EXISTS appeals (
    id VARCHAR(255) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,       -- 'ban' or 'article_deleted'
    target_uri VARCHAR(512),          -- article URI if article_deleted
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, approved, rejected
    admin_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_appeals_did ON appeals(did, created_at DESC);
CREATE INDEX idx_appeals_status ON appeals(status) WHERE status = 'pending';
