-- Phase B2: migrate the remaining 13 edge tables to carry group ids.
--
-- Single-tag columns: course_tags, course_session_tags,
-- course_session_prereqs, listing_preferred_tags, listing_required_tags,
-- skill_trees.
--
-- Parent/child columns: tag_parents, tag_parent_edits, user_tag_tree,
-- skill_tree_edges.
--
-- From/to columns: skill_tree_prereqs, user_tag_prereqs.
--
-- tag_aliases is left alone — aliases are now siblings inside groups
-- and the table is empty anyway.

-- ---------------------------------------------------------------------
-- Single-tag ref columns
-- ---------------------------------------------------------------------
DO $$
DECLARE tbl TEXT;
BEGIN
    FOR tbl IN
        SELECT unnest(ARRAY[
            'course_tags',
            'course_session_tags',
            'course_session_prereqs',
            'listing_preferred_tags',
            'listing_required_tags',
            'skill_trees'
        ])
    LOOP
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS group_id VARCHAR(64) REFERENCES tag_groups(id)',
            tbl
        );
        EXECUTE format(
            'UPDATE %I e SET group_id = t.group_id FROM tags t WHERE e.tag_id = t.id AND e.group_id IS NULL',
            tbl
        );
    END LOOP;
END $$;

-- Dedup rows where two different tag_ids in the same group were attached
-- to the same entity. Keep the row with the smallest tag_id; the trigger
-- + unique indexes below prevent future dupes.
DO $$
DECLARE tbl_part TEXT;
BEGIN
    FOR tbl_part IN
        SELECT unnest(ARRAY[
            'course_tags||course_id, group_id',
            'course_session_tags||session_id, group_id',
            'listing_preferred_tags||listing_id, group_id',
            'listing_required_tags||listing_id, group_id'
        ])
    LOOP
        DECLARE
            tbl TEXT := split_part(tbl_part, '||', 1);
            part TEXT := split_part(tbl_part, '||', 2);
        BEGIN
            EXECUTE format($q$
                DELETE FROM %I a USING (
                    SELECT ctid FROM (
                        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY %s ORDER BY tag_id) AS rn FROM %I
                    ) s WHERE rn > 1
                ) d WHERE a.ctid = d.ctid
            $q$, tbl, part, tbl);
        END;
    END LOOP;
END $$;

-- course_session_prereqs has composite (session, group, prereq_type)
DELETE FROM course_session_prereqs a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY session_id, group_id, prereq_type ORDER BY tag_id) AS rn
        FROM course_session_prereqs
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

-- skill_trees uses (did, group_id) or (tree_uri, group_id)? Each row is
-- one skill-tree per owner+concept — so (owner, group_id).
-- Actually skill_trees is the tree record itself; only a handful of
-- rows. Leave dedup to manual if it happens.

ALTER TABLE course_tags            ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE course_session_tags    ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE course_session_prereqs ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE listing_preferred_tags ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE listing_required_tags  ALTER COLUMN group_id SET NOT NULL;
-- skill_trees.tag_id may still be NULL for some old rows; don't force
-- NOT NULL yet.

CREATE UNIQUE INDEX IF NOT EXISTS idx_course_tags_group
    ON course_tags (course_id, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_course_session_tags_group
    ON course_session_tags (session_id, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_course_session_prereqs_group
    ON course_session_prereqs (session_id, group_id, prereq_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_listing_preferred_tags_group
    ON listing_preferred_tags (listing_id, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_listing_required_tags_group
    ON listing_required_tags (listing_id, group_id);

DO $$
DECLARE tbl TEXT;
BEGIN
    FOR tbl IN SELECT unnest(ARRAY[
        'course_tags','course_session_tags','course_session_prereqs',
        'listing_preferred_tags','listing_required_tags','skill_trees'
    ])
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I_sync_group ON %I', tbl, tbl);
        EXECUTE format(
            'CREATE TRIGGER %I_sync_group BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION sync_edge_group_id()',
            tbl, tbl
        );
    END LOOP;
END $$;

-- ---------------------------------------------------------------------
-- Parent/child pair columns: tag_parents, tag_parent_edits,
-- user_tag_tree, skill_tree_edges.
-- ---------------------------------------------------------------------
DO $$
DECLARE tbl TEXT;
BEGIN
    FOR tbl IN
        SELECT unnest(ARRAY[
            'tag_parents',
            'tag_parent_edits',
            'user_tag_tree',
            'skill_tree_edges'
        ])
    LOOP
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS parent_group VARCHAR(64) REFERENCES tag_groups(id)',
            tbl
        );
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS child_group VARCHAR(64) REFERENCES tag_groups(id)',
            tbl
        );
        EXECUTE format(
            'UPDATE %I e SET parent_group = (SELECT group_id FROM tags WHERE id = e.parent_tag), child_group  = (SELECT group_id FROM tags WHERE id = e.child_tag) WHERE e.parent_group IS NULL OR e.child_group IS NULL',
            tbl
        );
    END LOOP;
END $$;

-- Drop self-loops that result from collapsing (parent, child) where
-- both land in the same group.
DELETE FROM tag_parents       WHERE parent_group = child_group;
DELETE FROM tag_parent_edits  WHERE parent_group = child_group;
DELETE FROM user_tag_tree     WHERE parent_group = child_group;
DELETE FROM skill_tree_edges  WHERE parent_group = child_group;

-- Dedup (parent_group, child_group) pairs — tag_parents: global;
-- user_tag_tree: per-did; skill_tree_edges: per tree_uri.
DELETE FROM tag_parents a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY parent_group, child_group ORDER BY parent_tag, child_tag) AS rn FROM tag_parents
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

DELETE FROM user_tag_tree a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY did, parent_group, child_group ORDER BY parent_tag, child_tag) AS rn FROM user_tag_tree
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

DELETE FROM skill_tree_edges a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY tree_uri, parent_group, child_group ORDER BY parent_tag, child_tag) AS rn FROM skill_tree_edges
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

ALTER TABLE tag_parents      ALTER COLUMN parent_group SET NOT NULL;
ALTER TABLE tag_parents      ALTER COLUMN child_group  SET NOT NULL;
ALTER TABLE tag_parent_edits ALTER COLUMN parent_group SET NOT NULL;
ALTER TABLE tag_parent_edits ALTER COLUMN child_group  SET NOT NULL;
ALTER TABLE user_tag_tree    ALTER COLUMN parent_group SET NOT NULL;
ALTER TABLE user_tag_tree    ALTER COLUMN child_group  SET NOT NULL;
ALTER TABLE skill_tree_edges ALTER COLUMN parent_group SET NOT NULL;
ALTER TABLE skill_tree_edges ALTER COLUMN child_group  SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_tag_parents_group
    ON tag_parents (parent_group, child_group);
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_tag_tree_group
    ON user_tag_tree (did, parent_group, child_group);
CREATE UNIQUE INDEX IF NOT EXISTS idx_skill_tree_edges_group
    ON skill_tree_edges (tree_uri, parent_group, child_group);

-- Trigger for parent/child tables: sync both columns from parent_tag /
-- child_tag.
CREATE OR REPLACE FUNCTION sync_edge_parent_child_group() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.parent_group IS NULL AND NEW.parent_tag IS NOT NULL THEN
        NEW.parent_group := (SELECT group_id FROM tags WHERE id = NEW.parent_tag);
    END IF;
    IF NEW.child_group IS NULL AND NEW.child_tag IS NOT NULL THEN
        NEW.child_group := (SELECT group_id FROM tags WHERE id = NEW.child_tag);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
DECLARE tbl TEXT;
BEGIN
    FOR tbl IN SELECT unnest(ARRAY[
        'tag_parents','tag_parent_edits','user_tag_tree','skill_tree_edges'
    ])
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I_sync_group ON %I', tbl, tbl);
        EXECUTE format(
            'CREATE TRIGGER %I_sync_group BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION sync_edge_parent_child_group()',
            tbl, tbl
        );
    END LOOP;
END $$;

-- ---------------------------------------------------------------------
-- From/to pair columns: skill_tree_prereqs, user_tag_prereqs.
-- ---------------------------------------------------------------------
ALTER TABLE skill_tree_prereqs ADD COLUMN IF NOT EXISTS from_group VARCHAR(64) REFERENCES tag_groups(id);
ALTER TABLE skill_tree_prereqs ADD COLUMN IF NOT EXISTS to_group   VARCHAR(64) REFERENCES tag_groups(id);
ALTER TABLE user_tag_prereqs   ADD COLUMN IF NOT EXISTS from_group VARCHAR(64) REFERENCES tag_groups(id);
ALTER TABLE user_tag_prereqs   ADD COLUMN IF NOT EXISTS to_group   VARCHAR(64) REFERENCES tag_groups(id);

UPDATE skill_tree_prereqs e SET
    from_group = (SELECT group_id FROM tags WHERE id = e.from_tag),
    to_group   = (SELECT group_id FROM tags WHERE id = e.to_tag)
WHERE e.from_group IS NULL OR e.to_group IS NULL;

UPDATE user_tag_prereqs e SET
    from_group = (SELECT group_id FROM tags WHERE id = e.from_tag),
    to_group   = (SELECT group_id FROM tags WHERE id = e.to_tag)
WHERE e.from_group IS NULL OR e.to_group IS NULL;

DELETE FROM skill_tree_prereqs WHERE from_group = to_group;
DELETE FROM user_tag_prereqs   WHERE from_group = to_group;

DELETE FROM skill_tree_prereqs a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY tree_uri, from_group, to_group, prereq_type ORDER BY from_tag, to_tag) AS rn
        FROM skill_tree_prereqs
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

DELETE FROM user_tag_prereqs a USING (
    SELECT ctid FROM (
        SELECT ctid, ROW_NUMBER() OVER (PARTITION BY did, from_group, to_group, prereq_type ORDER BY from_tag, to_tag) AS rn
        FROM user_tag_prereqs
    ) s WHERE rn > 1
) d WHERE a.ctid = d.ctid;

ALTER TABLE skill_tree_prereqs ALTER COLUMN from_group SET NOT NULL;
ALTER TABLE skill_tree_prereqs ALTER COLUMN to_group   SET NOT NULL;
ALTER TABLE user_tag_prereqs   ALTER COLUMN from_group SET NOT NULL;
ALTER TABLE user_tag_prereqs   ALTER COLUMN to_group   SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_skill_tree_prereqs_group
    ON skill_tree_prereqs (tree_uri, from_group, to_group, prereq_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_tag_prereqs_group
    ON user_tag_prereqs (did, from_group, to_group, prereq_type);

CREATE OR REPLACE FUNCTION sync_edge_from_to_group() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.from_group IS NULL AND NEW.from_tag IS NOT NULL THEN
        NEW.from_group := (SELECT group_id FROM tags WHERE id = NEW.from_tag);
    END IF;
    IF NEW.to_group IS NULL AND NEW.to_tag IS NOT NULL THEN
        NEW.to_group := (SELECT group_id FROM tags WHERE id = NEW.to_tag);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS skill_tree_prereqs_sync_group ON skill_tree_prereqs;
CREATE TRIGGER skill_tree_prereqs_sync_group BEFORE INSERT OR UPDATE ON skill_tree_prereqs
    FOR EACH ROW EXECUTE FUNCTION sync_edge_from_to_group();

DROP TRIGGER IF EXISTS user_tag_prereqs_sync_group ON user_tag_prereqs;
CREATE TRIGGER user_tag_prereqs_sync_group BEFORE INSERT OR UPDATE ON user_tag_prereqs
    FOR EACH ROW EXECUTE FUNCTION sync_edge_from_to_group();
