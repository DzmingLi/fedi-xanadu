-- Course groups: the canonical concept that spans multiple iterations of
-- the same course. "CS229 (Autumn 2008)" and (hypothetically) "CS229
-- (Autumn 2018)" each live in the courses table, but share a group like
-- "Stanford CS229 — Machine Learning".
--
-- A group holds only the metadata that stays constant across iterations
-- (canonical title, course code, institution, description). Per-iteration
-- details (semester, source URL, instructors-this-year, session list)
-- stay on courses.

CREATE TABLE course_groups (
    id          VARCHAR(64) PRIMARY KEY,       -- cg-{tid}
    title       VARCHAR(500) NOT NULL,
    code        VARCHAR(50),                   -- e.g. "CS229", "ML4201"
    institution VARCHAR(500),
    description TEXT NOT NULL DEFAULT '',
    created_by  VARCHAR(255) NOT NULL,         -- DID
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE courses
    ADD COLUMN group_id VARCHAR(64) REFERENCES course_groups(id) ON DELETE SET NULL;

CREATE INDEX idx_courses_group_id ON courses(group_id);

-- Register course_groups in the polymorphic content table so comments can
-- hang off a group URI like "coursegroup:<id>". Group pages get their own
-- discussion thread — the spot where people argue about which iteration
-- is best, post cross-year errata, etc. Q&A intentionally NOT added: a
-- group spans iterations, and questions belong on specific lectures or
-- assignments where the content is concrete.
ALTER TABLE content DROP CONSTRAINT content_content_type_check;
ALTER TABLE content ADD CONSTRAINT content_content_type_check
    CHECK (content_type = ANY (ARRAY[
        'article','series','question','answer',
        'book','chapter','book_series','coursegroup'
    ]));
