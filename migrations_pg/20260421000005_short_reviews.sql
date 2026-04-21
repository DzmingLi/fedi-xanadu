-- Short reviews — "一句话书评" — for books and book series. PDS records live
-- under at.nightbo.book.shortReview / at.nightbo.bookseries.shortReview; the
-- tables below are the local mirror used for aggregation, visibility filtering,
-- and pagination.
--
-- Ratings are intentionally NOT embedded here. They stay in book_ratings /
-- book_series_ratings (mirrored from at.nightbo.book.rating /
-- at.nightbo.bookseries.rating). The UI overlays a user's rating onto their
-- short review at display time; a user can have one without the other.

CREATE TABLE book_short_reviews (
    id          VARCHAR(255) PRIMARY KEY,
    did         VARCHAR(255) NOT NULL,
    book_id     VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    edition_id  VARCHAR(64) REFERENCES book_editions(id) ON DELETE SET NULL,
    body        TEXT NOT NULL,
    lang        VARCHAR(10),
    visibility  VARCHAR(20) NOT NULL DEFAULT 'public'
                CHECK (visibility IN ('public','followers','private')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (did, book_id)
);
CREATE INDEX idx_book_short_reviews_book ON book_short_reviews(book_id, created_at DESC);
CREATE INDEX idx_book_short_reviews_did  ON book_short_reviews(did, created_at DESC);

CREATE TABLE book_series_short_reviews (
    id          VARCHAR(255) PRIMARY KEY,
    did         VARCHAR(255) NOT NULL,
    series_id   VARCHAR(64) NOT NULL REFERENCES book_series(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    lang        VARCHAR(10),
    visibility  VARCHAR(20) NOT NULL DEFAULT 'public'
                CHECK (visibility IN ('public','followers','private')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (did, series_id)
);
CREATE INDEX idx_bs_short_reviews_series ON book_series_short_reviews(series_id, created_at DESC);
CREATE INDEX idx_bs_short_reviews_did    ON book_series_short_reviews(did, created_at DESC);
