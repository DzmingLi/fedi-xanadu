-- Article collaboration: track collaborators and their pijul channels
CREATE TABLE article_collaborators (
    article_uri VARCHAR(512) NOT NULL,
    user_did VARCHAR(255) NOT NULL,
    channel_name VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'editor',
    invited_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (article_uri, user_did)
);

CREATE INDEX idx_article_collab_uri ON article_collaborators(article_uri);
CREATE INDEX idx_article_collab_user ON article_collaborators(user_did);
