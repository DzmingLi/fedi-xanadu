-- Course-level supplementary resources (software pages, tooling links,
-- FAQs, etc.) — surfaced in the CourseDetail sidebar.
CREATE TABLE course_resources (
    id         VARCHAR(64) PRIMARY KEY
               DEFAULT ('cr-' || substr(md5(random()::text), 1, 12)),
    course_id  VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    kind       VARCHAR(50) NOT NULL,
    label      VARCHAR(255) NOT NULL,
    url        TEXT NOT NULL,
    position   SMALLINT NOT NULL DEFAULT 0,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_course_resources_course ON course_resources (course_id);
