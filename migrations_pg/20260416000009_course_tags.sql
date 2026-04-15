-- Course tags: topics/prerequisites associated with a course
CREATE TABLE course_tags (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (course_id, tag_id)
);

CREATE INDEX idx_course_tags_tag ON course_tags(tag_id);
