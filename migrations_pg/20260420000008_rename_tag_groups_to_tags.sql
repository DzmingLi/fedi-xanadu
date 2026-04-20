-- Align table names with the "a tag has multiple names" mental model:
--
--   OLD                                     NEW
--   ─────────────────────────────────────   ─────────────────────────
--   tag_groups            (concepts)        tags
--   tags                  (per-lang labels) tag_labels
--   tag_group_representatives               tag_representatives
--
-- Column names on edge tables (tag_id / group_id) are left unchanged for
-- now — renaming them in one shot proved too invasive across the Rust
-- codebase. A follow-up migration can rename columns once the code paths
-- are unified.
--
-- tag_deletion_requests.tag_id is repointed from tag_labels.id (specific
-- label) to tags.id (concept), because the semantics of a deletion
-- request is "delete the concept and all its labels".

BEGIN;

-- ---- tag_deletion_requests: repoint tag_id from label → concept.
ALTER TABLE tag_deletion_requests ADD COLUMN IF NOT EXISTS tag_id_new VARCHAR(64);
UPDATE tag_deletion_requests r
SET tag_id_new = l.group_id
FROM tags l
WHERE l.id = r.tag_id AND r.tag_id_new IS NULL;
ALTER TABLE tag_deletion_requests DROP CONSTRAINT IF EXISTS tag_deletion_requests_tag_id_fkey;
ALTER TABLE tag_deletion_requests DROP COLUMN tag_id;
ALTER TABLE tag_deletion_requests RENAME COLUMN tag_id_new TO tag_id;

-- ---- Rename the tables. `tags` must be freed first.
ALTER TABLE tags RENAME TO tag_labels;
ALTER TABLE tag_groups RENAME TO tags;
ALTER TABLE tag_group_representatives RENAME TO tag_representatives;

-- tag_deletion_requests.tag_id now references tags (concept).
ALTER TABLE tag_deletion_requests
    ADD CONSTRAINT tag_deletion_requests_tag_id_fkey
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE;

-- ---- Update BEFORE-INSERT trigger functions + helper SQL functions to
-- refer to the renamed per-label table (tag_labels instead of tags).
CREATE OR REPLACE FUNCTION sync_edge_group_id() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.group_id IS NULL AND NEW.tag_id IS NOT NULL THEN
        NEW.group_id := (SELECT group_id FROM tag_labels WHERE id = NEW.tag_id);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION sync_edge_parent_child_group() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.parent_group IS NULL AND NEW.parent_tag IS NOT NULL THEN
        NEW.parent_group := (SELECT group_id FROM tag_labels WHERE id = NEW.parent_tag);
    END IF;
    IF NEW.child_group IS NULL AND NEW.child_tag IS NOT NULL THEN
        NEW.child_group := (SELECT group_id FROM tag_labels WHERE id = NEW.child_tag);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION sync_edge_from_to_group() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.from_group IS NULL AND NEW.from_tag IS NOT NULL THEN
        NEW.from_group := (SELECT group_id FROM tag_labels WHERE id = NEW.from_tag);
    END IF;
    IF NEW.to_group IS NULL AND NEW.to_tag IS NOT NULL THEN
        NEW.to_group := (SELECT group_id FROM tag_labels WHERE id = NEW.to_tag);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION tag_group_members(t TEXT)
RETURNS SETOF TEXT AS $$
    SELECT id FROM tag_labels
    WHERE group_id = (SELECT group_id FROM tag_labels WHERE id = t);
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION tag_group_representative(gid TEXT)
RETURNS TEXT AS $$
    SELECT id FROM tag_labels WHERE group_id = gid
    ORDER BY (lang = 'en') DESC, id
    LIMIT 1;
$$ LANGUAGE SQL STABLE;

COMMIT;
