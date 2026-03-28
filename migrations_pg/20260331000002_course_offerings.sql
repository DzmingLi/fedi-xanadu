-- Course semester/year versioning
ALTER TABLE courses ADD COLUMN term VARCHAR(20);       -- 'Spring', 'Fall', 'Summer', 'Winter', etc.
ALTER TABLE courses ADD COLUMN year SMALLINT;
ALTER TABLE courses ADD COLUMN canonical_id TEXT REFERENCES courses(id) ON DELETE SET NULL;

CREATE INDEX idx_courses_canonical ON courses(canonical_id) WHERE canonical_id IS NOT NULL;
