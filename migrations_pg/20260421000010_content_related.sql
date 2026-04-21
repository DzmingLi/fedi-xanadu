-- Fourth tag relation: "related". The content touches this concept
-- without teaching it or depending on it (e.g. a popular-science book
-- that discusses calculus but is not a calculus textbook).
CREATE TABLE content_related (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_related_tag ON content_related(tag_id);
