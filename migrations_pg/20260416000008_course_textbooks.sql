-- Course textbooks: link to existing books table
CREATE TABLE course_textbooks (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    book_id VARCHAR(64) NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'required',  -- required, recommended, supplementary
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (course_id, book_id)
);

CREATE INDEX idx_course_textbooks_course ON course_textbooks(course_id);
