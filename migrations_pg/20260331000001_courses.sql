-- Course/Syllabus system
CREATE TABLE IF NOT EXISTS courses (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    instructor_did TEXT NOT NULL,
    cover_url TEXT,
    schedule_type VARCHAR(20) NOT NULL DEFAULT 'weekly',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS course_units (
    id TEXT PRIMARY KEY,
    course_id TEXT NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    sort_order INT NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    available_from DATE
);

CREATE TABLE IF NOT EXISTS course_items (
    id TEXT PRIMARY KEY,
    unit_id TEXT NOT NULL REFERENCES course_units(id) ON DELETE CASCADE,
    sort_order INT NOT NULL DEFAULT 0,
    role VARCHAR(20) NOT NULL DEFAULT 'reading',
    target_uri TEXT,
    external_url TEXT,
    title TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    due_date DATE
);

CREATE INDEX idx_course_units_course ON course_units(course_id);
CREATE INDEX idx_course_items_unit ON course_items(unit_id);

-- Allow 'course' in content table
ALTER TABLE content DROP CONSTRAINT IF EXISTS content_content_type_check;
ALTER TABLE content ADD CONSTRAINT content_content_type_check
  CHECK (content_type = ANY (ARRAY['article','series','question','answer','book','course']));
