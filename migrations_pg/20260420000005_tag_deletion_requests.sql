-- Tag-deletion requests: any signed-in user can ask for a tag to be
-- removed; an admin approves. Approval soft-deletes by setting
-- tags.removed_at; all read paths filter to removed_at IS NULL.

ALTER TABLE tags ADD COLUMN IF NOT EXISTS removed_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS tag_deletion_requests (
    id            VARCHAR(64) PRIMARY KEY DEFAULT 'tdr-' || substr(md5(random()::text), 1, 12),
    tag_id        VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    requester_did VARCHAR(255) NOT NULL,
    reason        TEXT        NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','approved','rejected','cancelled')),
    reviewer_did  VARCHAR(255),
    review_note   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_tag_deletion_requests_tag    ON tag_deletion_requests(tag_id);
CREATE INDEX IF NOT EXISTS idx_tag_deletion_requests_status ON tag_deletion_requests(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_tag_deletion_requests_one_pending
    ON tag_deletion_requests(tag_id, requester_did) WHERE status = 'pending';
