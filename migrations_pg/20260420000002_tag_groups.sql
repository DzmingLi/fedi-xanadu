-- Tag system Phase 1: introduce alias/translation groups.
--
-- A tag_group is a concept; tags within a group are different labels for the
-- same concept. Labels can be different languages (translation — e.g.
-- "set-theory" ↔ "集合论") or same-language aliases (e.g. "machine-learning"
-- ↔ "ML"). All tags in a group are peers; there is no "canonical" or
-- "original" label at the data level.
--
-- This phase only adds the group structure and backfills. Edge tables
-- (content_prereqs / content_teaches / tag_parents / user_skills etc.)
-- continue to reference tag_id as before; service code will resolve to
-- group members at query time. Phase 2 will update callers.
--
-- Migration strategy:
--   1. Each existing tag becomes a solo-member group, lang='en' (our
--      legacy convention — all existing tag IDs are English slugs).
--   2. For each locale in the legacy `names` JSONB (e.g. names.zh), create
--      a NEW sibling tag in the same group with that locale. The sibling
--      tag's id is the locale value itself (Chinese characters, French
--      spelling, etc.). We verified no existing tag ids collide.
--   3. `tag_aliases` is currently empty so no alias migration is needed.

CREATE TABLE tag_groups (
    id          VARCHAR(64) PRIMARY KEY
                 DEFAULT 'tg-' || substr(md5(random()::text), 1, 16),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE tags ADD COLUMN IF NOT EXISTS group_id VARCHAR(64)
    REFERENCES tag_groups(id) ON DELETE RESTRICT;
ALTER TABLE tags ADD COLUMN IF NOT EXISTS lang VARCHAR(10) NOT NULL DEFAULT 'en';

CREATE INDEX IF NOT EXISTS idx_tags_group_id ON tags(group_id);

-- Backfill: create one group per existing tag, link it, lang='en'.
DO $$
DECLARE
    tag_rec RECORD;
    new_group VARCHAR(64);
BEGIN
    FOR tag_rec IN SELECT id FROM tags WHERE group_id IS NULL LOOP
        INSERT INTO tag_groups DEFAULT VALUES RETURNING id INTO new_group;
        UPDATE tags SET group_id = new_group, lang = 'en' WHERE id = tag_rec.id;
    END LOOP;
END $$;

ALTER TABLE tags ALTER COLUMN group_id SET NOT NULL;

-- Backfill: promote each non-en locale in `names` JSONB to a sibling tag
-- in the same group.
DO $$
DECLARE
    tag_rec RECORD;
    locale_key TEXT;
    locale_val TEXT;
BEGIN
    FOR tag_rec IN
        SELECT id, group_id, created_by, names
        FROM tags
        WHERE jsonb_typeof(names) = 'object' AND names != '{}'::jsonb
    LOOP
        FOR locale_key, locale_val IN SELECT * FROM jsonb_each_text(tag_rec.names) LOOP
            IF locale_key != 'en' AND locale_val IS NOT NULL AND btrim(locale_val) != '' THEN
                -- Skip if a tag with this id already exists (safety).
                IF NOT EXISTS (SELECT 1 FROM tags WHERE id = locale_val) THEN
                    INSERT INTO tags (id, name, created_by, lang, group_id, names)
                    VALUES (
                        locale_val,
                        locale_val,
                        tag_rec.created_by,
                        locale_key,
                        tag_rec.group_id,
                        jsonb_build_object(locale_key, locale_val)
                    );
                END IF;
            END IF;
        END LOOP;
    END LOOP;
END $$;

-- Helper: return all tag ids that are peers of the given tag (same group,
-- including the tag itself). Used by service code to resolve "edges on
-- this concept" into "edges on any member of this group".
CREATE OR REPLACE FUNCTION tag_group_members(t TEXT)
RETURNS SETOF TEXT AS $$
    SELECT id FROM tags
    WHERE group_id = (SELECT group_id FROM tags WHERE id = t);
$$ LANGUAGE SQL STABLE;

-- Helper: return one representative tag id for a given group. Picks the
-- English one if present (our legacy convention), else any member.
CREATE OR REPLACE FUNCTION tag_group_representative(gid TEXT)
RETURNS TEXT AS $$
    SELECT id FROM tags WHERE group_id = gid
    ORDER BY (lang = 'en') DESC, id
    LIMIT 1;
$$ LANGUAGE SQL STABLE;
