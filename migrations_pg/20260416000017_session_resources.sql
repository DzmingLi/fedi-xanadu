-- Replace individual URL columns with a single JSONB resources array.
-- Each element: {"type": "video"|"notes"|"hw"|"discussion", "url": "...", "label": "..."}
ALTER TABLE course_sessions ADD COLUMN IF NOT EXISTS resources JSONB NOT NULL DEFAULT '[]';

-- Migrate existing data
UPDATE course_sessions SET resources = (
  SELECT jsonb_agg(r) FROM (
    SELECT jsonb_build_object('type', 'video', 'url', video_url, 'label', '视频') AS r WHERE video_url IS NOT NULL
    UNION ALL
    SELECT jsonb_build_object('type', 'notes', 'url', notes_url, 'label', '讲义') WHERE notes_url IS NOT NULL
    UNION ALL
    SELECT jsonb_build_object('type', 'hw', 'url', assignment_url, 'label', COALESCE(assignment_label, 'HW')) WHERE assignment_url IS NOT NULL
    UNION ALL
    SELECT jsonb_build_object('type', 'discussion', 'url', discussion_url, 'label', COALESCE(discussion_label, '讨论')) WHERE discussion_url IS NOT NULL
  ) sub
) WHERE video_url IS NOT NULL OR notes_url IS NOT NULL OR assignment_url IS NOT NULL OR discussion_url IS NOT NULL;

-- Drop old columns
ALTER TABLE course_sessions DROP COLUMN IF EXISTS video_url;
ALTER TABLE course_sessions DROP COLUMN IF EXISTS notes_url;
ALTER TABLE course_sessions DROP COLUMN IF EXISTS assignment_url;
ALTER TABLE course_sessions DROP COLUMN IF EXISTS assignment_label;
ALTER TABLE course_sessions DROP COLUMN IF EXISTS discussion_url;
ALTER TABLE course_sessions DROP COLUMN IF EXISTS discussion_label;
