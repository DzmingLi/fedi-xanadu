-- Homeworks as a first-class entity. Until now they lived inside
-- course_sessions.resources as opaque JSONB entries with `type='hw'` —
-- good enough for display but impossible to link to from anywhere else.
-- Making them a row gives questions, comments, and future tag scopes
-- something stable to point at.

CREATE TABLE course_homeworks (
    id          VARCHAR(64) PRIMARY KEY,       -- chw-{tid}
    course_id   VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    -- Optional anchor to a specific lecture. Null for homeworks that span
    -- several lectures (e.g. a whole unit or the final project).
    session_id  VARCHAR(64) REFERENCES course_sessions(id) ON DELETE SET NULL,
    label       VARCHAR(500) NOT NULL,          -- "Homework 3 — GP Classification"
    url         VARCHAR(1024),                  -- PDF / webpage / github folder
    description TEXT NOT NULL DEFAULT '',
    position    INTEGER NOT NULL DEFAULT 0,
    due_date    DATE,
    created_by  VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_course_homeworks_course  ON course_homeworks(course_id, position);
CREATE INDEX idx_course_homeworks_session ON course_homeworks(session_id) WHERE session_id IS NOT NULL;

-- Questions (articles with kind='question') already carry book_id and
-- course_id. Add the narrower scopes so people can file a thread that
-- explicitly targets "CS229 Lecture 5" or "CS229 HW3" rather than the
-- whole course.
--
-- course_session_id may already exist via a parallel migration; guard
-- with IF NOT EXISTS so either order works. Homework_id is new here.
ALTER TABLE articles ADD COLUMN IF NOT EXISTS course_session_id VARCHAR(64);
ALTER TABLE articles ADD COLUMN IF NOT EXISTS homework_id        VARCHAR(64)
    REFERENCES course_homeworks(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_articles_course_session_id
    ON articles(course_session_id) WHERE course_session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_articles_homework_id
    ON articles(homework_id)       WHERE homework_id       IS NOT NULL;
