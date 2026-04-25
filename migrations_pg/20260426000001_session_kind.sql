-- Section dividers in the course schedule.
--
-- Course schedules tend to group lectures into thematic blocks
-- ("Operational semantics", "Types", "Advanced features", …). The
-- old shape — every row is a numbered lecture — couldn't represent
-- those headers, so course pages ended up as one long undifferentiated
-- list. Add a `kind` discriminator: `lecture` (default, the existing
-- behaviour), `section` (a header that spans the whole calendar
-- width), or `exam` (reserved for future styling; currently
-- frontend treats sessions with empty attachments as exams).
ALTER TABLE course_sessions
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'lecture';
