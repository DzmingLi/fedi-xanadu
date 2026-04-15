-- Courses: a first-class teaching entity that aggregates series, skill trees,
-- listings, and staff around a structured educational offering.

CREATE TABLE courses (
    id VARCHAR(64) PRIMARY KEY,               -- crs-{tid}
    did VARCHAR(255) NOT NULL,                 -- creator DID
    title VARCHAR(500) NOT NULL,
    code VARCHAR(50),                          -- e.g. "CS229", "6.006", "18.06"
    description TEXT NOT NULL DEFAULT '',
    syllabus TEXT NOT NULL DEFAULT '',         -- markdown
    institution VARCHAR(500),                  -- e.g. "MIT", "Stanford"
    department VARCHAR(500),
    semester VARCHAR(100),                     -- e.g. "Fall 2025", "Spring 2026"
    lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    license VARCHAR(100) NOT NULL DEFAULT 'CC-BY-SA-4.0',

    -- Attribution for imported/curated courses
    source_url VARCHAR(1024),                  -- original course URL (e.g. MIT OCW link)
    source_attribution TEXT,                   -- free-text attribution

    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_courses_did ON courses(did);
CREATE INDEX idx_courses_published ON courses(is_published, created_at DESC);

-- Course ↔ Series: a course can include multiple series (lectures, labs, problem sets)
CREATE TABLE course_series (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    series_id VARCHAR(255) NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'lectures',  -- lectures, labs, exercises, supplementary
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (course_id, series_id)
);

CREATE INDEX idx_course_series_course ON course_series(course_id);

-- Course ↔ Skill Tree: prerequisite knowledge graph
CREATE TABLE course_skill_trees (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'prerequisites',  -- prerequisites, outcomes
    PRIMARY KEY (course_id, tree_uri)
);

-- Course staff: instructors, TAs, etc.
CREATE TABLE course_staff (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    user_did VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'instructor',  -- instructor, ta, grader
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (course_id, user_did)
);

CREATE INDEX idx_course_staff_user ON course_staff(user_did);

-- Course ↔ Listings (linked, not owned — listings already exist independently)
CREATE TABLE course_listings (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    listing_id VARCHAR(64) NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    PRIMARY KEY (course_id, listing_id)
);

-- Course prerequisites: courses that should be taken before this one
CREATE TABLE course_prerequisites (
    course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    prereq_course_id VARCHAR(64) NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    PRIMARY KEY (course_id, prereq_course_id),
    CHECK (course_id <> prereq_course_id)
);
