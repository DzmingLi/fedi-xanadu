-- Series collaboration: track collaborators and their pijul channels
CREATE TABLE series_collaborators (
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    channel_name VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'editor',
    invited_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (series_id, user_did)
);

CREATE INDEX idx_series_collab_series ON series_collaborators(series_id);
CREATE INDEX idx_series_collab_user ON series_collaborators(user_did);
