-- Paper-specific metadata for articles with category='paper'.
CREATE TABLE paper_metadata (
    article_uri VARCHAR(512) PRIMARY KEY REFERENCES articles(at_uri) ON DELETE CASCADE,
    venue       VARCHAR(255),           -- e.g. "CVPR", "Nature", "NeurIPS"
    venue_type  VARCHAR(20),            -- 'conference', 'journal', 'preprint', 'workshop', 'thesis'
    year        SMALLINT,               -- publication year
    doi         VARCHAR(255),           -- e.g. "10.1109/CVPR.2026.12345"
    arxiv_id    VARCHAR(50),            -- e.g. "2406.12345"
    accepted    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_paper_metadata_venue ON paper_metadata(venue);
CREATE INDEX idx_paper_metadata_year ON paper_metadata(year);
CREATE UNIQUE INDEX idx_paper_metadata_doi ON paper_metadata(doi) WHERE doi IS NOT NULL;
CREATE UNIQUE INDEX idx_paper_metadata_arxiv ON paper_metadata(arxiv_id) WHERE arxiv_id IS NOT NULL;
