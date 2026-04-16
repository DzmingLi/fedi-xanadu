-- Supplementary resources for books (solutions, exercises, videos, slides, etc.)
CREATE TABLE book_resources (
    id         VARCHAR(64) PRIMARY KEY DEFAULT 'br-' || substr(md5(random()::text), 1, 12),
    book_id    VARCHAR(64) NOT NULL,
    edition_id VARCHAR(64),           -- NULL = applies to all editions
    kind       VARCHAR(50) NOT NULL,  -- 'solutions', 'exercises', 'video', 'slides', 'errata', 'code', 'other'
    label      VARCHAR(255) NOT NULL, -- display name
    url        TEXT NOT NULL,
    position   SMALLINT NOT NULL DEFAULT 0,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_book_resources_book ON book_resources(book_id);
