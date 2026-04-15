-- Add schedule (calendar) to courses as JSONB array of sessions
-- Each element: {"session": 1, "topic": "...", "date": "2020-09-02", "notes": "Homework 1 due"}
ALTER TABLE courses ADD COLUMN IF NOT EXISTS schedule JSONB NOT NULL DEFAULT '[]';
