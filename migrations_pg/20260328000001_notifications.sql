-- Notification system
CREATE TABLE IF NOT EXISTS notifications (
    id VARCHAR(255) PRIMARY KEY,
    recipient_did VARCHAR(255) NOT NULL,
    actor_did VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    target_uri VARCHAR(512),
    context_id VARCHAR(512),
    read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_recipient ON notifications(recipient_did, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications(recipient_did) WHERE read = FALSE;
