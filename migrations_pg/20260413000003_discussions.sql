-- Discussions: pull request mechanism for pijul changes
CREATE TABLE discussions (
    id VARCHAR(255) PRIMARY KEY,
    target_uri VARCHAR(512) NOT NULL,
    source_uri VARCHAR(512) NOT NULL,
    author_did VARCHAR(255) NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE discussion_changes (
    id SERIAL PRIMARY KEY,
    discussion_id VARCHAR(255) NOT NULL REFERENCES discussions(id) ON DELETE CASCADE,
    change_hash VARCHAR(128) NOT NULL,
    added_by VARCHAR(255) NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied BOOLEAN NOT NULL DEFAULT FALSE,
    applied_at TIMESTAMPTZ
);

CREATE INDEX idx_discussions_target ON discussions(target_uri);
CREATE INDEX idx_discussions_source ON discussions(source_uri);
CREATE INDEX idx_discussion_changes_disc ON discussion_changes(discussion_id);
