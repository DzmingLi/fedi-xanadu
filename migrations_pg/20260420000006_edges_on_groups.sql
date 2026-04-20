-- Phase B: edge tables carry `group_id` alongside `tag_id`. Writes fill
-- both so old code keeps working; reads can dedupe/aggregate by group_id
-- without the expand-to-group CTE.
--
-- Hot tables first (user-visible); remaining edge tables follow in a
-- later migration so each step ships safely.

DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOR tbl IN
        SELECT unnest(ARRAY[
            'content_teaches',
            'content_prereqs',
            'content_topics',
            'user_skills',
            'user_interests'
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

-- content_teaches and content_prereqs / content_topics / user_skills /
-- user_interests may have duplicates after group collapse (e.g. both
-- 'Math' and '数学' attached to the same book chapter). Dedup — keep
-- the row whose tag_id matches the group's en rep when possible.
DO $$
DECLARE
    tbl TEXT;
    pk_expr TEXT;
BEGIN
    FOR tbl, pk_expr IN
        SELECT 'content_teaches',   'content_uri, group_id' UNION ALL
        SELECT 'content_teaches',   'content_uri, group_id' UNION ALL
        SELECT 'content_topics',    'content_uri, group_id' UNION ALL
        SELECT 'user_interests',    'did, group_id'        UNION ALL
        SELECT 'user_skills',       'did, group_id'
    LOOP
        EXECUTE format($q$
            DELETE FROM %I a
            USING (
                SELECT ctid FROM (
                    SELECT ctid,
                           ROW_NUMBER() OVER (
                               PARTITION BY %s
                               ORDER BY (tag_id IN (SELECT tag_id FROM tag_group_representatives r
                                                    WHERE r.group_id = %I.group_id AND r.lang = 'en')) DESC,
                                        tag_id
                           ) AS rn
                    FROM %I
                ) s WHERE rn > 1
            ) d
            WHERE a.ctid = d.ctid
        $q$, tbl, pk_expr, tbl, tbl);
    END LOOP;
END $$;

-- content_prereqs has (content_uri, group_id, prereq_type) as the key.
DELETE FROM content_prereqs a
USING (
    SELECT ctid FROM (
        SELECT ctid,
               ROW_NUMBER() OVER (
                   PARTITION BY content_uri, group_id, prereq_type
                   ORDER BY tag_id
               ) AS rn
        FROM content_prereqs
    ) s WHERE rn > 1
) d
WHERE a.ctid = d.ctid;

-- Now safe to make group_id NOT NULL and add unique indexes. Leaving
-- tag_id columns untouched as legacy/audit; new reads key on group_id.
ALTER TABLE content_teaches ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE content_prereqs ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE content_topics  ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE user_skills     ALTER COLUMN group_id SET NOT NULL;
ALTER TABLE user_interests  ALTER COLUMN group_id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_content_teaches_group  ON content_teaches  (content_uri, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_content_topics_group   ON content_topics   (content_uri, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_content_prereqs_group  ON content_prereqs  (content_uri, group_id, prereq_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_skills_group      ON user_skills      (did, group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_interests_group   ON user_interests   (did, group_id);

-- Trigger: keep group_id in sync with tag_id on write. Callers can pass
-- either column; the other fills in automatically. Eventually we drop
-- tag_id and callers pass group_id only — this trigger is the bridge.
CREATE OR REPLACE FUNCTION sync_edge_group_id() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.group_id IS NULL AND NEW.tag_id IS NOT NULL THEN
        NEW.group_id := (SELECT group_id FROM tags WHERE id = NEW.tag_id);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
DECLARE tbl TEXT;
BEGIN
    FOR tbl IN SELECT unnest(ARRAY[
        'content_teaches','content_prereqs','content_topics',
        'user_skills','user_interests'
    ])
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I_sync_group ON %I', tbl, tbl);
        EXECUTE format(
            'CREATE TRIGGER %I_sync_group BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION sync_edge_group_id()',
            tbl, tbl
        );
    END LOOP;
END $$;
