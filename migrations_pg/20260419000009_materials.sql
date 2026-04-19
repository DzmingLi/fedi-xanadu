-- Merge `readings` (text) + `reading_refs` (jsonb) + resources of type='notes'
-- into a single `materials` jsonb column with shape [{kind?, label, url?}].

ALTER TABLE course_sessions ADD COLUMN materials JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 1) Plain text readings → single material chip without kind or url.
UPDATE course_sessions
SET materials = materials || jsonb_build_array(jsonb_build_object('label', readings))
WHERE readings IS NOT NULL AND trim(readings) <> '';

-- 2) Existing reading_refs → kind='reading'.
UPDATE course_sessions
SET materials = materials || COALESCE(
    (SELECT jsonb_agg(
        CASE
          WHEN x ? 'url' AND x->>'url' IS NOT NULL AND x->>'url' <> '' THEN
            jsonb_build_object('kind', 'reading', 'label', x->>'label', 'url', x->>'url')
          ELSE
            jsonb_build_object('kind', 'reading', 'label', x->>'label')
        END
     )
     FROM jsonb_array_elements(reading_refs) x),
    '[]'::jsonb)
WHERE jsonb_typeof(reading_refs) = 'array' AND jsonb_array_length(reading_refs) > 0;

-- 3) resources with type='notes' → materials. Infer kind from the label text.
UPDATE course_sessions
SET materials = materials || COALESCE(
    (SELECT jsonb_agg(
        jsonb_build_object(
          'kind',
          CASE lower(x->>'label')
            WHEN 'handout' THEN 'handout'
            WHEN 'summary' THEN 'summary'
            WHEN 'slides' THEN 'slides'
            ELSE 'notes'
          END,
          'label', x->>'label',
          'url', x->>'url'
        )
     )
     FROM jsonb_array_elements(resources) x
     WHERE x->>'type' = 'notes'),
    '[]'::jsonb);

-- 4) Strip type='notes' entries from resources (keep video, hw, discussion, etc.).
UPDATE course_sessions
SET resources = COALESCE(
    (SELECT jsonb_agg(x) FROM jsonb_array_elements(resources) x WHERE x->>'type' <> 'notes'),
    '[]'::jsonb);

-- 5) Drop legacy columns.
ALTER TABLE course_sessions DROP COLUMN readings;
ALTER TABLE course_sessions DROP COLUMN reading_refs;
