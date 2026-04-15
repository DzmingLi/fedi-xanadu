-- Add label fields for assignment and discussion links
ALTER TABLE course_sessions ADD COLUMN IF NOT EXISTS assignment_label TEXT;
ALTER TABLE course_sessions ADD COLUMN IF NOT EXISTS discussion_label TEXT;
