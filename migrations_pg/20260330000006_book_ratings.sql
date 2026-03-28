-- Book ratings: half-star system, max 5 stars (stored as integer 1-10, representing 0.5-5.0)
CREATE TABLE IF NOT EXISTS book_ratings (
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    rating SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (book_id, user_did)
);

-- Reading status: want_to_read, reading, finished
CREATE TABLE IF NOT EXISTS book_reading_status (
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('want_to_read', 'reading', 'finished')),
    progress SMALLINT NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (book_id, user_did)
);
