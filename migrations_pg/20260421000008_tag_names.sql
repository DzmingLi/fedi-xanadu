-- Phase 2 of the tag-system refactor.
--
-- Old model: `tags` (concept) + `tag_labels` (one row per label string,
-- language-stamped) + `tag_representatives` (primary label per concept ×
-- locale). `tag_labels.id` was a human string serving triple duty as PK,
-- display name, and URL slug — which caused pollution bugs and collisions.
--
-- New model: `tags` (concept) + `tag_names` (one row per name, surrogate
-- PK, no primary/alias distinction — all names equal) + `user_name_pref`
-- (per-user preferred name, optional). Default display when no pref is
-- set: first-added name in user's locale → en → any.
--
-- Edge tables already reference `tags.id` and stay unchanged.

-- 1. New table: tag_names. Identity + display only — who-added-what
-- lives in `tag_audit_log` so admin can review changes without
-- muddying the hot-path read schema.
CREATE TABLE tag_names (
    id          varchar(64)  PRIMARY KEY DEFAULT ('tn-' || substr(md5(random()::text), 1, 16)),
    tag_id      varchar(64)  NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    name        varchar(255) NOT NULL,
    lang        varchar(10)  NOT NULL,
    added_at    timestamptz  NOT NULL DEFAULT now(),
    UNIQUE (tag_id, name, lang)
);
CREATE INDEX idx_tag_names_tag_id   ON tag_names(tag_id);
CREATE INDEX idx_tag_names_lang     ON tag_names(lang);
CREATE INDEX idx_tag_names_name_ci  ON tag_names(lower(name));

-- 1b. Audit trail for tag-system mutations. Every create-tag, add-name,
-- remove-name, and merge operation lands a row here so admin can see
-- who's shaping the tag graph and catch abuse. Display / edge-write
-- paths never read from this table.
CREATE TABLE tag_audit_log (
    id          bigserial    PRIMARY KEY,
    action      varchar(30)  NOT NULL CHECK (action IN ('create_tag', 'add_name', 'remove_name', 'merge_tag')),
    actor_did   varchar(255) NOT NULL,
    tag_id      varchar(64),
    name        varchar(255),
    lang        varchar(10),
    merged_into varchar(64),
    at          timestamptz  NOT NULL DEFAULT now()
);
CREATE INDEX idx_tag_audit_actor ON tag_audit_log(actor_did);
CREATE INDEX idx_tag_audit_tag   ON tag_audit_log(tag_id);
CREATE INDEX idx_tag_audit_at    ON tag_audit_log(at DESC);

-- 2. New table: user_name_pref
CREATE TABLE user_name_pref (
    did       varchar(255) NOT NULL,
    tag_id    varchar(64)  NOT NULL REFERENCES tags(id)      ON DELETE CASCADE,
    name_id   varchar(64)  NOT NULL REFERENCES tag_names(id) ON DELETE CASCADE,
    chosen_at timestamptz  NOT NULL DEFAULT now(),
    PRIMARY KEY (did, tag_id)
);
CREATE INDEX idx_user_name_pref_did ON user_name_pref(did);

-- 3. Copy data: tag_labels (live rows only) → tag_names, and seed the
-- audit log with an `add_name` entry per migrated row so the history
-- of who originally introduced each name is preserved.
INSERT INTO tag_names (id, tag_id, name, lang, added_at)
SELECT 'tn-' || substr(md5(random()::text || id), 1, 16),
       tag_id, name, lang, created_at
  FROM tag_labels
 WHERE removed_at IS NULL;

INSERT INTO tag_audit_log (action, actor_did, tag_id, name, lang, at)
SELECT 'add_name', created_by, tag_id, name, lang, created_at
  FROM tag_labels
 WHERE removed_at IS NULL;

-- 4. Redefine the two SQL helpers against tag_names. Their return
-- shape stays compatible with existing callers (display string for
-- `tag_canonical_label`, lang→string JSONB for `tag_label_map`), but
-- the meaning of "canonical" shifts from "admin-picked rep label"
-- to "earliest-added name in the requested locale, then en, then any".
-- User-preference overlay happens at the application layer, not here.
DROP FUNCTION IF EXISTS tag_canonical_label(varchar, text);
DROP FUNCTION IF EXISTS tag_canonical_label(varchar);
DROP FUNCTION IF EXISTS tag_label_map(varchar);

CREATE FUNCTION tag_canonical_label(tid varchar, locale text DEFAULT 'en')
RETURNS varchar AS $$
    SELECT name FROM tag_names
    WHERE tag_id = tid
    ORDER BY (lang = locale) DESC, (lang = 'en') DESC, added_at ASC, id
    LIMIT 1
$$ LANGUAGE sql STABLE;

CREATE FUNCTION tag_label_map(tid varchar)
RETURNS jsonb AS $$
    SELECT COALESCE(jsonb_object_agg(lang, name), '{}'::jsonb)
    FROM (
        SELECT DISTINCT ON (lang) lang, name
        FROM tag_names
        WHERE tag_id = tid
        ORDER BY lang, added_at ASC, id
    ) earliest_per_lang
$$ LANGUAGE sql STABLE;

-- 5. Drop legacy tables. tag_representatives references tag_labels, so
-- it goes first. tag_labels is referenced by nothing else after the
-- representatives table is gone (edges reference `tags`, not labels).
DROP TABLE tag_representatives;
DROP TABLE tag_labels;

-- 6. Soft-delete queue no longer applies — all `tag_names` are live.
-- Existing `tag_deletion_requests` keeps working as concept-level
-- deletion requests (tag_id → tags.id). No change needed.

-- 7. Leave `tags.id` as the canonical identifier used by every edge
-- table. URL slugs can come in a later migration if needed.
