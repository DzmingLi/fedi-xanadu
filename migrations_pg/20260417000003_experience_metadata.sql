-- Experience post metadata (postgrad, interview, competition, etc.)
CREATE TABLE experience_metadata (
    article_uri VARCHAR(512) PRIMARY KEY REFERENCES articles(at_uri) ON DELETE CASCADE,
    kind        VARCHAR(30),   -- 'postgrad', 'interview', 'competition', 'application', 'other'
    target      VARCHAR(255),  -- target school/company/competition name
    year        SMALLINT,
    result      VARCHAR(50),   -- 'accepted', 'rejected', 'pending', 'passed', 'failed'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_experience_metadata_kind ON experience_metadata(kind);
CREATE INDEX idx_experience_metadata_target ON experience_metadata(target);
CREATE INDEX idx_experience_metadata_year ON experience_metadata(year);
