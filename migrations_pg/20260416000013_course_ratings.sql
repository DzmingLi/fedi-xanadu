-- Course ratings (same pattern as book_ratings)
CREATE TABLE IF NOT EXISTS course_ratings (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    user_did  VARCHAR(255) NOT NULL,
    rating    SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (course_id, user_did)
);

-- Allow articles to reference a course (for reviews)
ALTER TABLE articles ADD COLUMN IF NOT EXISTS course_id VARCHAR(64) REFERENCES courses(id) ON DELETE SET NULL;
