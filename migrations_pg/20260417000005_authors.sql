-- Author entities — may or may not be NightBoat users.
CREATE TABLE authors (
    id          VARCHAR(64) PRIMARY KEY DEFAULT 'au-' || substr(md5(random()::text), 1, 12),
    name        VARCHAR(255) NOT NULL,
    -- Optional link to a NightBoat user (AT Protocol DID or platform DID)
    did         VARCHAR(255) UNIQUE,
    orcid       VARCHAR(20),
    affiliation VARCHAR(255),
    homepage    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_authors_name ON authors(name);
CREATE UNIQUE INDEX idx_authors_orcid ON authors(orcid) WHERE orcid IS NOT NULL;

-- Many-to-many: books ↔ authors (replaces books.authors text array)
CREATE TABLE book_authors (
    book_id     VARCHAR(64) NOT NULL,
    author_id   VARCHAR(64) NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    position    SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (book_id, author_id)
);

CREATE INDEX idx_book_authors_author ON book_authors(author_id);

-- Backfill: create author entries from existing books.authors arrays
-- and link them via book_authors
INSERT INTO authors (id, name)
SELECT DISTINCT
    'au-' || substr(md5(unnest), 1, 12),
    unnest
FROM (
    SELECT DISTINCT unnest(authors) FROM books
) sub
ON CONFLICT (id) DO NOTHING;

INSERT INTO book_authors (book_id, author_id, position)
SELECT b.id, a.id, row_number() OVER (PARTITION BY b.id ORDER BY ord) - 1
FROM books b,
     LATERAL unnest(b.authors) WITH ORDINALITY AS u(name, ord)
JOIN authors a ON a.name = u.name
ON CONFLICT DO NOTHING;
