-- Course-level learning status mirrors book_reading_status.
CREATE TABLE course_learning_status (
    course_id  VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    user_did   VARCHAR(255) NOT NULL,
    status     VARCHAR(20) NOT NULL
               CHECK (status IN ('want_to_learn', 'learning', 'finished', 'dropped')),
    progress   SMALLINT NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (course_id, user_did)
);
CREATE INDEX idx_course_learning_status_user ON course_learning_status (user_did, course_id);

-- Per-session (per-lecture) progress mirrors book_chapter_progress.
CREATE TABLE course_session_progress (
    course_id    VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    session_id   VARCHAR(64) NOT NULL REFERENCES course_sessions(id) ON DELETE CASCADE,
    user_did     VARCHAR(255) NOT NULL,
    completed    BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (session_id, user_did)
);
CREATE INDEX idx_course_session_progress_user ON course_session_progress (user_did, course_id);

-- Widen book_reading_status to accept 'dropped' — the UI button already
-- sends this value, but the original CHECK constraint was missing it.
ALTER TABLE book_reading_status DROP CONSTRAINT book_reading_status_status_check;
ALTER TABLE book_reading_status ADD CONSTRAINT book_reading_status_status_check
    CHECK (status IN ('want_to_read', 'reading', 'finished', 'dropped'));
