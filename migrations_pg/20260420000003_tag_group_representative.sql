-- Each alias/translation group has a designated representative tag —
-- the single label used when the UI needs to pick one for display (e.g.
-- "this article's prereq is X" where X is the group, not a specific
-- language label). Admin can change the representative on a per-group
-- basis. Edge-table writes still land on whichever specific tag_id the
-- author selected; only reads resolve to the representative for display.

ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS representative_tag_id VARCHAR(64)
    REFERENCES tags(id) ON DELETE SET NULL;

-- Backfill heuristic: English first (slug convention), then longer label
-- over shorter within the same language so "Computer Vision" beats "CV"
-- and "Linear Algebra" beats "LA". Admin can override per group.
UPDATE tag_groups g
SET representative_tag_id = (
    SELECT id FROM tags
    WHERE group_id = g.id
    ORDER BY (lang = 'en') DESC, length(name) DESC, id
    LIMIT 1
);

ALTER TABLE tag_groups
    ALTER COLUMN representative_tag_id SET NOT NULL;

-- Helper: return the representative tag id for a given tag's group,
-- falling back to the tag itself if nothing is set. Used in display-side
-- dedup so the rendered label reflects admin choice.
CREATE OR REPLACE FUNCTION tag_representative(t TEXT) RETURNS TEXT AS $$
    SELECT COALESCE(
        (SELECT g.representative_tag_id
         FROM tag_groups g JOIN tags tg ON tg.group_id = g.id
         WHERE tg.id = t),
        t
    );
$$ LANGUAGE SQL STABLE;
