-- Collapse the "group vs tag" vocabulary now that the data model calls the
-- concept a `tag` and the per-language display row a `tag_label`. Every
-- relationship that is semantically between *concepts* (teaches, prereqs,
-- taxonomy, user interests) drops its label-level FK entirely: edges that
-- used to carry both `tag_id` (→ label) and `group_id` (→ tag) collapse
-- to a single `tag_id` (→ tag). Only tables whose semantics really are
-- about a specific spelling keep a `label_id` column:
--
--   * tag_labels           — the label row itself
--   * tag_representatives  — chosen display label per (tag, lang)
--   * tag_aliases          — per-label alternate spellings
--   * tag_parent_edits     — audit log preserves which label the editor clicked
--
-- Also drop `tag_labels.names`; the translation map is aggregated on
-- demand via `tag_label_map(tag_id)` at read time.

-- ══════════════════════════════════════════════════════════════════════
-- Drop the triggers and trigger functions that referenced the old pair
-- of label + group columns.
-- ══════════════════════════════════════════════════════════════════════
DROP TRIGGER IF EXISTS content_teaches_sync_group        ON content_teaches;
DROP TRIGGER IF EXISTS content_prereqs_sync_group        ON content_prereqs;
DROP TRIGGER IF EXISTS content_topics_sync_group         ON content_topics;
DROP TRIGGER IF EXISTS user_skills_sync_group            ON user_skills;
DROP TRIGGER IF EXISTS user_interests_sync_group         ON user_interests;
DROP TRIGGER IF EXISTS course_tags_sync_group            ON course_tags;
DROP TRIGGER IF EXISTS course_session_tags_sync_group    ON course_session_tags;
DROP TRIGGER IF EXISTS course_session_prereqs_sync_group ON course_session_prereqs;
DROP TRIGGER IF EXISTS listing_preferred_tags_sync_group ON listing_preferred_tags;
DROP TRIGGER IF EXISTS listing_required_tags_sync_group  ON listing_required_tags;
DROP TRIGGER IF EXISTS skill_trees_sync_group            ON skill_trees;
DROP TRIGGER IF EXISTS tag_parents_sync_group            ON tag_parents;
DROP TRIGGER IF EXISTS tag_parent_edits_sync_group       ON tag_parent_edits;
DROP TRIGGER IF EXISTS user_tag_tree_sync_group          ON user_tag_tree;
DROP TRIGGER IF EXISTS skill_tree_edges_sync_group       ON skill_tree_edges;
DROP TRIGGER IF EXISTS skill_tree_prereqs_sync_group     ON skill_tree_prereqs;
DROP TRIGGER IF EXISTS user_tag_prereqs_sync_group       ON user_tag_prereqs;

DROP FUNCTION IF EXISTS sync_edge_group_id();
DROP FUNCTION IF EXISTS sync_edge_parent_child_group();
DROP FUNCTION IF EXISTS sync_edge_from_to_group();
DROP FUNCTION IF EXISTS tag_group_members(text);
DROP FUNCTION IF EXISTS tag_group_representative(text);

-- ══════════════════════════════════════════════════════════════════════
-- Content & relation tables that collapse to tag-only.
--
-- For each: drop the old PK (it indexed the label column we're about to
-- remove), drop the label column, drop the group-based dedup index (we
-- promote it to the new PK), rename group_id → tag_id, install the new
-- PK.
-- ══════════════════════════════════════════════════════════════════════

-- content_teaches (content_uri, tag_id)
ALTER TABLE content_teaches DROP CONSTRAINT content_teaches_pkey;
ALTER TABLE content_teaches DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_content_teaches_group;
ALTER TABLE content_teaches RENAME COLUMN group_id TO tag_id;
ALTER TABLE content_teaches ADD PRIMARY KEY (content_uri, tag_id);

-- content_prereqs (content_uri, tag_id, prereq_type)
ALTER TABLE content_prereqs DROP CONSTRAINT content_prereqs_pkey;
ALTER TABLE content_prereqs DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_content_prereqs_group;
ALTER TABLE content_prereqs RENAME COLUMN group_id TO tag_id;
ALTER TABLE content_prereqs ADD PRIMARY KEY (content_uri, tag_id, prereq_type);

-- content_topics (content_uri, tag_id)
ALTER TABLE content_topics DROP CONSTRAINT content_topics_pkey;
ALTER TABLE content_topics DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_content_topics_group;
ALTER TABLE content_topics RENAME COLUMN group_id TO tag_id;
ALTER TABLE content_topics ADD PRIMARY KEY (content_uri, tag_id);

-- course_tags (course_id, tag_id)
ALTER TABLE course_tags DROP CONSTRAINT course_tags_pkey;
ALTER TABLE course_tags DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_course_tags_group;
ALTER TABLE course_tags RENAME COLUMN group_id TO tag_id;
ALTER TABLE course_tags ADD PRIMARY KEY (course_id, tag_id);

-- course_session_tags (session_id, tag_id)
ALTER TABLE course_session_tags DROP CONSTRAINT course_session_tags_pkey;
ALTER TABLE course_session_tags DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_course_session_tags_group;
ALTER TABLE course_session_tags RENAME COLUMN group_id TO tag_id;
ALTER TABLE course_session_tags ADD PRIMARY KEY (session_id, tag_id);

-- course_session_prereqs (session_id, tag_id)
ALTER TABLE course_session_prereqs DROP CONSTRAINT course_session_prereqs_pkey;
ALTER TABLE course_session_prereqs DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_course_session_prereqs_group;
ALTER TABLE course_session_prereqs RENAME COLUMN group_id TO tag_id;
ALTER TABLE course_session_prereqs ADD PRIMARY KEY (session_id, tag_id);

-- listing_preferred_tags (listing_id, tag_id)
ALTER TABLE listing_preferred_tags DROP CONSTRAINT listing_preferred_tags_pkey;
ALTER TABLE listing_preferred_tags DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_listing_preferred_tags_group;
ALTER TABLE listing_preferred_tags RENAME COLUMN group_id TO tag_id;
ALTER TABLE listing_preferred_tags ADD PRIMARY KEY (listing_id, tag_id);

-- listing_required_tags (listing_id, tag_id)
ALTER TABLE listing_required_tags DROP CONSTRAINT listing_required_tags_pkey;
ALTER TABLE listing_required_tags DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_listing_required_tags_group;
ALTER TABLE listing_required_tags RENAME COLUMN group_id TO tag_id;
ALTER TABLE listing_required_tags ADD PRIMARY KEY (listing_id, tag_id);

-- user_skills (did, tag_id)
ALTER TABLE user_skills DROP CONSTRAINT user_skills_pkey;
ALTER TABLE user_skills DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_user_skills_group;
ALTER TABLE user_skills RENAME COLUMN group_id TO tag_id;
ALTER TABLE user_skills ADD PRIMARY KEY (did, tag_id);

-- user_interests (did, tag_id)
ALTER TABLE user_interests DROP CONSTRAINT user_interests_pkey;
ALTER TABLE user_interests DROP COLUMN tag_id;
DROP INDEX IF EXISTS idx_user_interests_group;
ALTER TABLE user_interests RENAME COLUMN group_id TO tag_id;
ALTER TABLE user_interests ADD PRIMARY KEY (did, tag_id);

-- skill_trees — PK is (at_uri); just swap the tag columns.
ALTER TABLE skill_trees DROP COLUMN tag_id;
ALTER TABLE skill_trees RENAME COLUMN group_id TO tag_id;

-- ══════════════════════════════════════════════════════════════════════
-- Parent/child edge tables — collapse to (parent_tag, child_tag).
-- ══════════════════════════════════════════════════════════════════════

-- tag_parents (parent_tag, child_tag)
ALTER TABLE tag_parents DROP CONSTRAINT tag_parents_pkey;
ALTER TABLE tag_parents DROP COLUMN parent_tag;
ALTER TABLE tag_parents DROP COLUMN child_tag;
DROP INDEX IF EXISTS idx_tag_parents_group;
ALTER TABLE tag_parents RENAME COLUMN parent_group TO parent_tag;
ALTER TABLE tag_parents RENAME COLUMN child_group TO child_tag;
ALTER TABLE tag_parents ADD PRIMARY KEY (parent_tag, child_tag);

-- user_tag_tree (did, parent_tag, child_tag)
ALTER TABLE user_tag_tree DROP CONSTRAINT user_tag_tree_pkey;
ALTER TABLE user_tag_tree DROP COLUMN parent_tag;
ALTER TABLE user_tag_tree DROP COLUMN child_tag;
DROP INDEX IF EXISTS idx_user_tag_tree_group;
ALTER TABLE user_tag_tree RENAME COLUMN parent_group TO parent_tag;
ALTER TABLE user_tag_tree RENAME COLUMN child_group TO child_tag;
ALTER TABLE user_tag_tree ADD PRIMARY KEY (did, parent_tag, child_tag);

-- skill_tree_edges (tree_uri, parent_tag, child_tag)
ALTER TABLE skill_tree_edges DROP CONSTRAINT skill_tree_edges_pkey;
ALTER TABLE skill_tree_edges DROP COLUMN parent_tag;
ALTER TABLE skill_tree_edges DROP COLUMN child_tag;
DROP INDEX IF EXISTS idx_skill_tree_edges_group;
ALTER TABLE skill_tree_edges RENAME COLUMN parent_group TO parent_tag;
ALTER TABLE skill_tree_edges RENAME COLUMN child_group TO child_tag;
ALTER TABLE skill_tree_edges ADD PRIMARY KEY (tree_uri, parent_tag, child_tag);

-- ══════════════════════════════════════════════════════════════════════
-- Prereq-edge tables — collapse to (from_tag, to_tag).
-- ══════════════════════════════════════════════════════════════════════

-- skill_tree_prereqs (tree_uri, from_tag, to_tag, prereq_type)
ALTER TABLE skill_tree_prereqs DROP CONSTRAINT skill_tree_prereqs_pkey;
ALTER TABLE skill_tree_prereqs DROP COLUMN from_tag;
ALTER TABLE skill_tree_prereqs DROP COLUMN to_tag;
DROP INDEX IF EXISTS idx_skill_tree_prereqs_group;
ALTER TABLE skill_tree_prereqs RENAME COLUMN from_group TO from_tag;
ALTER TABLE skill_tree_prereqs RENAME COLUMN to_group TO to_tag;
ALTER TABLE skill_tree_prereqs ADD PRIMARY KEY (tree_uri, from_tag, to_tag, prereq_type);

-- user_tag_prereqs (did, from_tag, to_tag, prereq_type)
ALTER TABLE user_tag_prereqs DROP CONSTRAINT user_tag_prereqs_pkey;
ALTER TABLE user_tag_prereqs DROP COLUMN from_tag;
ALTER TABLE user_tag_prereqs DROP COLUMN to_tag;
DROP INDEX IF EXISTS idx_user_tag_prereqs_group;
ALTER TABLE user_tag_prereqs RENAME COLUMN from_group TO from_tag;
ALTER TABLE user_tag_prereqs RENAME COLUMN to_group TO to_tag;
ALTER TABLE user_tag_prereqs ADD PRIMARY KEY (did, from_tag, to_tag, prereq_type);

-- ══════════════════════════════════════════════════════════════════════
-- Tables that genuinely need both label and tag — rename both columns.
-- ══════════════════════════════════════════════════════════════════════

-- tag_labels: the label's own concept FK.
ALTER TABLE tag_labels RENAME COLUMN group_id TO tag_id;

-- tag_aliases merges into tag_labels: each alias becomes another label
-- row in the same tag. Existing aliases inherit lang from their anchor
-- label (English default for whatever reason). Rows whose anchor is
-- already gone are skipped.
INSERT INTO tag_labels (id, name, lang, tag_id, created_by)
SELECT a.alias,
       a.alias,
       COALESCE(anchor.lang, 'en'),
       anchor.tag_id,
       anchor.created_by
FROM tag_aliases a
JOIN tag_labels anchor ON anchor.id = a.tag_id
WHERE NOT EXISTS (SELECT 1 FROM tag_labels e WHERE e.id = a.alias);

DROP TABLE tag_aliases;

-- tag_representatives (tag_id, lang, label_id)
ALTER TABLE tag_representatives DROP CONSTRAINT tag_group_representatives_pkey;
ALTER TABLE tag_representatives RENAME COLUMN tag_id TO label_id;
ALTER TABLE tag_representatives RENAME COLUMN group_id TO tag_id;
ALTER TABLE tag_representatives ADD PRIMARY KEY (tag_id, lang);

-- tag_parent_edits: audit log collapses to tag-level too. The label a
-- past editor clicked has no lasting meaning beyond the tag the click
-- resolved to.
ALTER TABLE tag_parent_edits DROP COLUMN parent_tag;
ALTER TABLE tag_parent_edits DROP COLUMN child_tag;
ALTER TABLE tag_parent_edits RENAME COLUMN parent_group TO parent_tag;
ALTER TABLE tag_parent_edits RENAME COLUMN child_group  TO child_tag;

-- tag_deletion_requests: promote from label-level to tag-level.
-- Preserve existing rows by resolving each old tag_id (label) into its
-- tag, then swap columns.
ALTER TABLE tag_deletion_requests ADD COLUMN new_tag_id VARCHAR(64);
UPDATE tag_deletion_requests r
SET new_tag_id = (SELECT tag_id FROM tag_labels WHERE id = r.tag_id);
-- Requests whose underlying label was already hard-deleted are orphan
-- audit rows — drop them so we can enforce NOT NULL.
DELETE FROM tag_deletion_requests WHERE new_tag_id IS NULL;
ALTER TABLE tag_deletion_requests DROP COLUMN tag_id;
ALTER TABLE tag_deletion_requests RENAME COLUMN new_tag_id TO tag_id;
ALTER TABLE tag_deletion_requests ALTER COLUMN tag_id SET NOT NULL;
ALTER TABLE tag_deletion_requests
    ADD CONSTRAINT tag_deletion_requests_tag_id_fkey
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE;

-- ══════════════════════════════════════════════════════════════════════
-- Replace the old `group_id → tags(id)` FKs with ON DELETE CASCADE so
-- that deleting a tag automatically sweeps every edge/content row that
-- referenced it. The original FKs were NO ACTION on group_id, which
-- leaves orphan rows when a tag disappears.
-- ══════════════════════════════════════════════════════════════════════
DO $$
DECLARE
    rec RECORD;
BEGIN
    FOR rec IN
        SELECT tbl, fk_name FROM (VALUES
            ('content_teaches',        'content_teaches_group_id_fkey'),
            ('content_prereqs',        'content_prereqs_group_id_fkey'),
            ('content_topics',         'content_topics_group_id_fkey'),
            ('course_tags',            'course_tags_group_id_fkey'),
            ('course_session_tags',    'course_session_tags_group_id_fkey'),
            ('course_session_prereqs', 'course_session_prereqs_group_id_fkey'),
            ('listing_preferred_tags', 'listing_preferred_tags_group_id_fkey'),
            ('listing_required_tags',  'listing_required_tags_group_id_fkey'),
            ('user_skills',            'user_skills_group_id_fkey'),
            ('user_interests',         'user_interests_group_id_fkey'),
            ('skill_trees',            'skill_trees_group_id_fkey'),
            ('tag_representatives',    'tag_group_representatives_group_id_fkey')
        ) AS t(tbl, fk_name)
    LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', rec.tbl, rec.fk_name);
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE',
            rec.tbl, rec.tbl || '_tag_id_fkey'
        );
    END LOOP;
END $$;

-- Parent/child edge tables: FKs on parent_tag + child_tag.
DO $$
DECLARE
    rec RECORD;
BEGIN
    FOR rec IN SELECT unnest(ARRAY[
        'tag_parents', 'tag_parent_edits', 'user_tag_tree', 'skill_tree_edges'
    ]) AS tbl
    LOOP
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            rec.tbl, rec.tbl || '_parent_group_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            rec.tbl, rec.tbl || '_child_group_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (parent_tag) REFERENCES tags(id) ON DELETE CASCADE',
            rec.tbl, rec.tbl || '_parent_tag_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (child_tag) REFERENCES tags(id) ON DELETE CASCADE',
            rec.tbl, rec.tbl || '_child_tag_fkey'
        );
    END LOOP;
END $$;

-- From/to prereq-edge tables: FKs on from_tag + to_tag.
DO $$
DECLARE
    rec RECORD;
BEGIN
    FOR rec IN SELECT unnest(ARRAY[
        'skill_tree_prereqs', 'user_tag_prereqs'
    ]) AS tbl
    LOOP
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            rec.tbl, rec.tbl || '_from_group_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I',
            rec.tbl, rec.tbl || '_to_group_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (from_tag) REFERENCES tags(id) ON DELETE CASCADE',
            rec.tbl, rec.tbl || '_from_tag_fkey'
        );
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (to_tag) REFERENCES tags(id) ON DELETE CASCADE',
            rec.tbl, rec.tbl || '_to_tag_fkey'
        );
    END LOOP;
END $$;

-- ══════════════════════════════════════════════════════════════════════
-- Drop the jsonb cache now that readers aggregate from siblings on demand.
-- ══════════════════════════════════════════════════════════════════════
ALTER TABLE tag_labels DROP COLUMN IF EXISTS names;

-- ══════════════════════════════════════════════════════════════════════
-- Helper function: per-tag {lang: name} map, assembled from siblings.
-- ══════════════════════════════════════════════════════════════════════
CREATE OR REPLACE FUNCTION tag_label_map(tid VARCHAR(64))
RETURNS JSONB
LANGUAGE SQL
STABLE
AS $$
    SELECT COALESCE(jsonb_object_agg(lang, name), '{}'::jsonb)
    FROM tag_labels
    WHERE tag_id = tid AND removed_at IS NULL
$$;

-- All labels sharing a tag with the given label.
CREATE OR REPLACE FUNCTION tag_sibling_labels(label_id VARCHAR(255))
RETURNS TABLE (id VARCHAR(255))
LANGUAGE SQL
STABLE
AS $$
    SELECT id FROM tag_labels
    WHERE tag_id = (SELECT tag_id FROM tag_labels WHERE id = label_id)
$$;

-- Canonical label for a tag in a given locale; English-first fallback.
CREATE OR REPLACE FUNCTION tag_canonical_label(tid VARCHAR(64), locale TEXT DEFAULT 'en')
RETURNS VARCHAR(255)
LANGUAGE SQL
STABLE
AS $$
    SELECT id FROM tag_labels
    WHERE tag_id = tid
    ORDER BY (lang = locale) DESC, (lang = 'en') DESC, id
    LIMIT 1
$$;
