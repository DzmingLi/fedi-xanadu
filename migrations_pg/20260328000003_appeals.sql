-- Article visibility
--   public     — visible everywhere
--   cn_hidden  — visible on intl only, hidden on CN instance
--   removed    — admin-removed, 30-day appeal window
ALTER TABLE articles
  ADD COLUMN visibility    VARCHAR(20) NOT NULL DEFAULT 'public',
  ADD COLUMN removed_at    TIMESTAMPTZ,
  ADD COLUMN remove_reason TEXT;

CREATE INDEX idx_articles_visibility ON articles(visibility);

-- Phone verification (CN instance requires this for writes)
ALTER TABLE platform_users
  ADD COLUMN phone             VARCHAR(20) UNIQUE,
  ADD COLUMN phone_verified_at TIMESTAMPTZ;

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
