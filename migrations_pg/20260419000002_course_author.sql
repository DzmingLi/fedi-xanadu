-- Courses can now be attributed to Author entities (parallel to books) so
-- external instructors without platform accounts work the same way as
-- external book authors. When an author later claims their identity via
-- admin-approved binding, the author row's did gets populated and the UI
-- switches to linking their platform profile.
--
-- A course may have multiple instructors (co-teachers, guest lecturers);
-- course_authors mirrors book_authors / article_authors.
--
-- Also drops is_published — every course is public once created, same as
-- books and articles.

ALTER TABLE courses DROP COLUMN is_published;

CREATE TABLE course_authors (
    course_id  VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    author_id  VARCHAR(64) NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    position   SMALLINT    NOT NULL DEFAULT 0,
    role       VARCHAR(50)          DEFAULT 'instructor',
    PRIMARY KEY (course_id, author_id)
);
CREATE INDEX idx_course_authors_author ON course_authors (author_id);
