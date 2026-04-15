-- Replace the flat JSONB schedule column with a proper course_sessions table.
-- Each session can have its own tags and prereq tags.

CREATE TABLE IF NOT EXISTS course_sessions (
    id          VARCHAR(64) PRIMARY KEY,
    course_id   VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    sort_order  INT NOT NULL DEFAULT 0,
    topic       TEXT,
    date        TEXT,
    readings    TEXT,
    video_url   TEXT,
    notes_url   TEXT,
    assignment_url TEXT,
    discussion_url TEXT,
    UNIQUE (course_id, sort_order)
);

CREATE INDEX idx_course_sessions_course ON course_sessions(course_id);

-- Tags associated with a session (what this session covers)
CREATE TABLE IF NOT EXISTS course_session_tags (
    session_id  VARCHAR(64) NOT NULL REFERENCES course_sessions(id) ON DELETE CASCADE,
    tag_id      VARCHAR(64) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, tag_id)
);

-- Prereq tags for a session (what you should know before this session)
CREATE TABLE IF NOT EXISTS course_session_prereqs (
    session_id  VARCHAR(64) NOT NULL REFERENCES course_sessions(id) ON DELETE CASCADE,
    tag_id      VARCHAR(64) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, tag_id)
);

-- Drop the old JSONB schedule column
ALTER TABLE courses DROP COLUMN IF EXISTS schedule;
