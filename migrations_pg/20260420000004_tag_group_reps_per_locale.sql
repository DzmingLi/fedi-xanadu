-- Per-locale representatives. The earlier migration stored a single
-- `representative_tag_id` on tag_groups; that design made us encode
-- locale logic as a SQL CASE-scored comparison. Replace with an explicit
-- per-locale table: admin picks one representative per language, display
-- is a trivial lookup (uiLocale first, fall back to 'en').

CREATE TABLE IF NOT EXISTS tag_group_representatives (
    group_id VARCHAR(64) NOT NULL REFERENCES tag_groups(id) ON DELETE CASCADE,
    lang     VARCHAR(10) NOT NULL,
    tag_id   VARCHAR(64) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, lang)
);

-- Backfill: promote the current single representative to be the rep for
-- its own language (usually 'en' after the earlier heuristic backfill).
INSERT INTO tag_group_representatives (group_id, lang, tag_id)
SELECT g.id, t.lang, t.id
FROM tag_groups g JOIN tags t ON t.id = g.representative_tag_id
ON CONFLICT (group_id, lang) DO NOTHING;

-- Also seed a rep for every OTHER locale in each group — take any
-- member of that locale. Admin can re-pick later if they have multiple
-- same-lang siblings and the arbitrary pick is wrong.
INSERT INTO tag_group_representatives (group_id, lang, tag_id)
SELECT DISTINCT ON (t.group_id, t.lang) t.group_id, t.lang, t.id
FROM tags t
ORDER BY t.group_id, t.lang, length(t.name) DESC, t.id
ON CONFLICT (group_id, lang) DO NOTHING;

-- Drop the old single-rep column + function now that everything uses
-- the per-locale table.
ALTER TABLE tag_groups DROP COLUMN IF EXISTS representative_tag_id;
DROP FUNCTION IF EXISTS tag_representative(TEXT);
