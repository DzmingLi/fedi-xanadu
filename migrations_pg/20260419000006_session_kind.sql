-- course_sessions now covers not just lectures but also labs and graded
-- work. `assignment` is the umbrella for homework, projects, and exams —
-- anything with a deadline and a submission. The `topic` field still
-- carries the specific name ("Final Exam", "Project 2: Gitlet").
--
-- Existing rows default to 'lecture'. The readings / resources columns
-- are unchanged and apply to all kinds.
ALTER TABLE course_sessions
    ADD COLUMN kind VARCHAR(20) NOT NULL DEFAULT 'lecture'
        CHECK (kind IN ('lecture', 'lab', 'assignment'));

CREATE INDEX idx_course_sessions_kind ON course_sessions (course_id, kind, sort_order);
