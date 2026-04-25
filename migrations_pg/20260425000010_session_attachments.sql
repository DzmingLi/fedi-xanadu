-- Collapse session.materials + session.resources into a single
-- session.attachments JSONB array. Rationale: the old split was driven
-- by display preference (materials show in left columns, resources show
-- in right ones) rather than data semantics — same shape, three storage
-- locations (materials[], resources[], readings free-text col) for the
-- same concept of "stuff attached to a lecture". Single Attachment with
-- a kind enum + required flag puts naming/grouping logic in the view
-- layer where it belongs.
--
-- Each Attachment: {kind, label, url, required: bool}. URL is required —
-- citations without URLs would belong in a separate citations table, not
-- here. Materials lacking URLs (rare; previously used for free-text
-- "readings" placeholders) are dropped.

ALTER TABLE course_sessions
    ADD COLUMN attachments JSONB NOT NULL DEFAULT '[]'::jsonb;

-- Pull materials in: optional → required = NOT optional. Drop entries
-- without a URL (the schema redesign forbids them).
UPDATE course_sessions SET attachments = COALESCE(
    (SELECT jsonb_agg(jsonb_build_object(
        'kind', CASE
            WHEN m->>'kind' IN ('reading','slides','handout','summary','notes','code')
                THEN m->>'kind'
            ELSE 'other'
        END,
        'label', m->>'label',
        'url', m->>'url',
        'required', NOT COALESCE((m->>'optional')::boolean, false)
     ))
     FROM jsonb_array_elements(materials) m
     WHERE COALESCE(m->>'url', '') <> ''),
    '[]'::jsonb
);

-- Append resources. resources[] has no `optional` field, so default to
-- required=true. Map type='hw' → kind='homework'; other type values pass
-- through if they're known kinds, else fall to 'other'.
UPDATE course_sessions SET attachments = attachments || COALESCE(
    (SELECT jsonb_agg(jsonb_build_object(
        'kind', CASE
            WHEN r->>'type' = 'hw' THEN 'homework'
            WHEN r->>'type' IN
                ('video','notes','discussion','reading','slides','handout',
                 'code','summary','outline','homework')
                THEN r->>'type'
            ELSE 'other'
        END,
        'label', r->>'label',
        'url', r->>'url',
        'required', true
     ))
     FROM jsonb_array_elements(resources) r
     WHERE COALESCE(r->>'url', '') <> ''),
    '[]'::jsonb
);

ALTER TABLE course_sessions DROP COLUMN materials;
ALTER TABLE course_sessions DROP COLUMN resources;
