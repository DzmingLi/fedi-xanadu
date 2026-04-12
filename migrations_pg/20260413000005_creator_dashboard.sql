-- Series draft/published status
ALTER TABLE series ADD COLUMN is_published BOOLEAN NOT NULL DEFAULT FALSE;

-- Mark all existing series as published (they were created before this migration)
UPDATE series SET is_published = TRUE;

-- Article view tracking
CREATE TABLE article_views (
    id BIGSERIAL PRIMARY KEY,
    article_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri) ON DELETE CASCADE,
    viewer_did VARCHAR(255),
    viewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_article_views_uri ON article_views(article_uri);
CREATE INDEX idx_article_views_date ON article_views(viewed_at);

-- Materialized daily stats for fast dashboard queries
CREATE MATERIALIZED VIEW creator_daily_stats AS
SELECT creator_did, day,
       SUM(views)::bigint AS views,
       SUM(comments)::bigint AS comments,
       SUM(bookmarks)::bigint AS bookmarks
FROM (
    SELECT a.did AS creator_did, DATE(av.viewed_at) AS day,
           COUNT(av.id) AS views, 0::bigint AS comments, 0::bigint AS bookmarks
    FROM articles a JOIN article_views av ON av.article_uri = a.at_uri
    WHERE a.removed_at IS NULL
    GROUP BY a.did, DATE(av.viewed_at)

    UNION ALL

    SELECT a.did, DATE(c.created_at),
           0, COUNT(c.id), 0
    FROM articles a JOIN comments c ON c.content_uri = a.at_uri
    WHERE a.removed_at IS NULL
    GROUP BY a.did, DATE(c.created_at)

    UNION ALL

    SELECT a.did, DATE(b.created_at),
           0, 0, COUNT(b.article_uri)
    FROM articles a JOIN user_bookmarks b ON b.article_uri = a.at_uri
    WHERE a.removed_at IS NULL
    GROUP BY a.did, DATE(b.created_at)
) sub
GROUP BY creator_did, day;

CREATE UNIQUE INDEX idx_creator_daily_stats ON creator_daily_stats(creator_did, day);
