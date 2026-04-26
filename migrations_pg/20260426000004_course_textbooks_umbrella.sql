-- Umbrella-course textbooks. Many courses use the same textbook across
-- every iteration (15-312 always uses PFPL); duplicating that link on
-- every term_textbooks row is noise. Add a parallel table at the course
-- level. Term-level textbooks (`term_textbooks`) stay for genuinely
-- term-specific recommendations (a guest-instructor's extra reading,
-- an alternate edition for a particular semester).
--
-- The frontend renders course-level books in the course header (above
-- the term picker) and term-level books in the per-term materials
-- section (below the calendar). A book may appear at one level OR
-- the other; deduplication is the editor's responsibility.
--
-- Naming hazard: there is now BOTH `course_textbooks` (umbrella) and
-- `term_textbooks` (iteration). They look symmetric and are.

CREATE TABLE course_textbooks (
    course_id  VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    book_id    VARCHAR(64) NOT NULL REFERENCES books(id)   ON DELETE CASCADE,
    role       VARCHAR(50) NOT NULL DEFAULT 'required',
    sort_order INTEGER     NOT NULL DEFAULT 0,
    PRIMARY KEY (course_id, book_id)
);

CREATE INDEX idx_course_textbooks_course ON course_textbooks(course_id);
