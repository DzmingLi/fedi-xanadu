-- Add timestamp to votes for trending analysis
ALTER TABLE votes ADD COLUMN voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Indexes for recommendation query performance
CREATE INDEX idx_votes_target_time ON votes(target_uri, voted_at);
CREATE INDEX idx_article_views_uri_recent ON article_views(article_uri, viewed_at);
CREATE INDEX idx_content_teaches_uri ON content_teaches(content_uri);
CREATE INDEX idx_comments_uri_time ON comments(content_uri, created_at);
