-- Entity patches: patch-based edit history for structured entities (courses, books, etc.)
--
-- Each patch is a JSON Patch (RFC 6902) array of operations.
-- Entity tables store materialized current state; patches are the source of truth.
-- Creator edits are auto-applied; other users' edits go to "pending" for review.

CREATE TABLE entity_patches (
    id VARCHAR(64) PRIMARY KEY,               -- p-{tid}
    entity_type VARCHAR(30) NOT NULL,         -- 'course', 'book'
    entity_id VARCHAR(64) NOT NULL,
    author_did VARCHAR(255) NOT NULL,
    operations JSONB NOT NULL,                -- RFC 6902 JSON Patch array
    summary TEXT NOT NULL DEFAULT '',
    status VARCHAR(20) NOT NULL DEFAULT 'applied',  -- applied, pending, rejected
    reviewed_by VARCHAR(255),                 -- who approved/rejected
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entity_patches_entity ON entity_patches(entity_type, entity_id, created_at);
CREATE INDEX idx_entity_patches_status ON entity_patches(status) WHERE status = 'pending';
CREATE INDEX idx_entity_patches_author ON entity_patches(author_did);
