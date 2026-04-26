-- ═══════════════════════════════════════════════════════════════════════════
-- Conceptual rename: the OLD `courses` table held a single iteration of a
-- course in a specific term ("CS229 Autumn 2008"); the OLD `course_groups`
-- table was the umbrella over multiple iterations ("CS229 — Stanford
-- Machine Learning"). The NEW model promotes the umbrella to first-class
-- "course" status, so:
--
--     OLD courses           →  NEW terms
--     OLD course_groups     →  NEW courses
--     OLD course_<thing>    →  NEW term_<thing>          (per-iteration)
--     OLD course_id         →  NEW term_id               (FK to per-iteration)
--     OLD courses.group_id  →  NEW terms.course_id       (umbrella FK)
--     OLD prereq_course_id  →  NEW prereq_term_id
--     OLD content_type='coursegroup'  →  NEW 'course'
--     OLD URI 'coursegroup:cg-X'      →  NEW 'course:crs-X'
--     OLD URI 'course:crs-X'          →  NEW 'term:trm-X'
--     OLD entity_patches.entity_type='course'      →  'term'
--     OLD entity_patches.entity_type='coursegroup' →  'course'
--
-- ID prefixes also flip:
--     crs-XXX   (was per-iteration)  →  trm-XXX
--     cg-XXX    (was umbrella)       →  crs-XXX
--
-- Because the namespaces overlap mid-rename (we want to call the new
-- umbrella "crs-X" but those ids are in use by per-iteration rows), we
-- stage the prefix swap through a temporary "TMPCG-" prefix.
--
-- Phase order is critical:
--   1. Auto-create umbrella course_group rows for every ungrouped course
--      so that after the rename, every term has a parent course.
--   2. Stash existing cg-X ids under TMPCG-X (and propagate to FKs).
--   3. Rename crs-X → trm-X everywhere (all FK columns, content URIs,
--      patches, ATProto record references, articles, content table).
--   4. Rename TMPCG-X → crs-X (the new umbrella ids).
--   5. RENAME tables  (courses → terms, course_groups → courses, all
--      course_<thing> → term_<thing>).
--   6. RENAME columns (course_id → term_id everywhere; group_id →
--      course_id on the new terms table; prereq_course_id → prereq_term_id).
--   7. Update content CHECK constraint to allow 'course' (and drop the
--      now-obsolete 'coursegroup' value).
--   8. Re-create renamed indexes.
--
-- Everything runs inside one implicit transaction (sqlx wraps each
-- migration). If any step fails, the whole rename rolls back.
-- ═══════════════════════════════════════════════════════════════════════════

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 0. Drop FK / triggers that would otherwise interfere with re-keying
-- and re-typing. We re-add them after the prefix swap.
-- ───────────────────────────────────────────────────────────────────────────

-- ON UPDATE CASCADE would be ideal here but the current FKs use NO ACTION.
-- Drop the FKs that point at courses(id) / course_groups(id), let us flip
-- the keys, then re-add. Constraint names follow the default
-- `<table>_<col>_fkey` Postgres convention.

-- 0a. Tables FK'd to courses(id) (the per-iteration entity, soon to be terms).
ALTER TABLE course_series              DROP CONSTRAINT IF EXISTS course_series_course_id_fkey;
ALTER TABLE course_skill_trees         DROP CONSTRAINT IF EXISTS course_skill_trees_course_id_fkey;
ALTER TABLE course_listings            DROP CONSTRAINT IF EXISTS course_listings_course_id_fkey;
ALTER TABLE course_prerequisites       DROP CONSTRAINT IF EXISTS course_prerequisites_course_id_fkey;
ALTER TABLE course_prerequisites       DROP CONSTRAINT IF EXISTS course_prerequisites_prereq_course_id_fkey;
ALTER TABLE course_textbooks           DROP CONSTRAINT IF EXISTS course_textbooks_course_id_fkey;
ALTER TABLE course_tags                DROP CONSTRAINT IF EXISTS course_tags_course_id_fkey;
ALTER TABLE course_sessions            DROP CONSTRAINT IF EXISTS course_sessions_course_id_fkey;
ALTER TABLE course_ratings             DROP CONSTRAINT IF EXISTS course_ratings_course_id_fkey;
ALTER TABLE course_authors             DROP CONSTRAINT IF EXISTS course_authors_course_id_fkey;
ALTER TABLE course_learning_status     DROP CONSTRAINT IF EXISTS course_learning_status_course_id_fkey;
ALTER TABLE course_session_progress    DROP CONSTRAINT IF EXISTS course_session_progress_course_id_fkey;
ALTER TABLE course_resources           DROP CONSTRAINT IF EXISTS course_resources_course_id_fkey;
ALTER TABLE course_homeworks           DROP CONSTRAINT IF EXISTS course_homeworks_course_id_fkey;
ALTER TABLE articles                   DROP CONSTRAINT IF EXISTS articles_course_id_fkey;
-- courses.group_id → course_groups.id
ALTER TABLE courses                    DROP CONSTRAINT IF EXISTS courses_group_id_fkey;

-- 0b. FKs that point at content(uri). The content URI strings change
-- mid-migration (coursegroup:cg-X → course:crs-X, course:crs-X →
-- term:trm-X), and Postgres FKs have no ON UPDATE CASCADE here, so we
-- drop and re-add the FKs around the URI rewrites.
ALTER TABLE comments         DROP CONSTRAINT IF EXISTS comments_content_uri_fkey;
ALTER TABLE content_teaches  DROP CONSTRAINT IF EXISTS content_teaches_content_uri_fkey;
ALTER TABLE content_topics   DROP CONSTRAINT IF EXISTS content_topics_content_uri_fkey;
ALTER TABLE content_prereqs  DROP CONSTRAINT IF EXISTS content_prereqs_content_uri_fkey;
ALTER TABLE content_related  DROP CONSTRAINT IF EXISTS content_related_content_uri_fkey;

-- 0c. Loosen the content_type CHECK so the URI rewrite phase can flip
-- 'coursegroup' rows to 'course' without tripping the constraint.
-- We tighten back to the canonical set in Phase 8.
ALTER TABLE content DROP CONSTRAINT IF EXISTS content_content_type_check;
ALTER TABLE content ADD  CONSTRAINT content_content_type_check CHECK (
    content_type = ANY (ARRAY[
        'article','series','question','answer','book','chapter',
        'book_series','coursegroup','course','term','paper'
    ])
);

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 1. Auto-create umbrella course_group rows for every ungrouped
-- course. The new "courses" table needs a parent for every term, so any
-- per-iteration row that currently lacks a group_id gets a synthetic
-- umbrella that mirrors its title/code/institution/description.
--
-- Synthetic id format: cg-auto-<first 12 chars of original crs- id>.
-- After the prefix swap below, this becomes crs-auto-<…>, which is a
-- perfectly valid umbrella id and visually flags it as auto-generated
-- if anyone goes hunting later.
-- ───────────────────────────────────────────────────────────────────────────

INSERT INTO course_groups (id, title, code, institution, description, created_by, created_at)
SELECT
    'cg-auto-' || substr(c.id, 5, 12)        AS id,         -- strip "crs-" prefix
    c.title,
    c.code,
    c.institution,
    c.description,
    c.did                                    AS created_by,
    c.created_at
  FROM courses c
 WHERE c.group_id IS NULL
   -- Defensive: skip if the synthetic id would collide with an existing
   -- group (shouldn't happen in practice — cg-auto- has no historical
   -- usage — but cheap guard).
   AND NOT EXISTS (
       SELECT 1 FROM course_groups g
        WHERE g.id = 'cg-auto-' || substr(c.id, 5, 12)
   );

-- Link those courses to their freshly-minted umbrellas.
UPDATE courses c
   SET group_id = 'cg-auto-' || substr(c.id, 5, 12)
 WHERE c.group_id IS NULL;

-- Register the new umbrellas in the polymorphic content table so the
-- comment system sees them. Pre-rename content_type is still
-- 'coursegroup' here; the constraint flip happens in phase 7.
INSERT INTO content (uri, content_type)
SELECT 'coursegroup:' || g.id, 'coursegroup'
  FROM course_groups g
 WHERE g.id LIKE 'cg-auto-%'
   AND NOT EXISTS (SELECT 1 FROM content c WHERE c.uri = 'coursegroup:' || g.id);

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 2. Stash every cg-X id under TMPCG-X to free up the crs-* namespace
-- for Phase 3. Update the umbrella table itself plus every FK / URI that
-- references it.
-- ───────────────────────────────────────────────────────────────────────────

UPDATE course_groups
   SET id = 'TMPCG-' || substr(id, 4)        -- strip "cg-" (3 chars) → "TMPCG-"
 WHERE id LIKE 'cg-%';

UPDATE courses
   SET group_id = 'TMPCG-' || substr(group_id, 4)
 WHERE group_id LIKE 'cg-%';

-- Polymorphic content rows for course_groups are still 'coursegroup:cg-X' —
-- bump them to 'coursegroup:TMPCG-X' so comments / votes / bookmarks stay
-- pointed at the right group through the swap.
UPDATE content
   SET uri = 'coursegroup:TMPCG-' || substr(uri, length('coursegroup:cg-') + 1)
 WHERE uri LIKE 'coursegroup:cg-%';

UPDATE comments
   SET content_uri = 'coursegroup:TMPCG-' || substr(content_uri, length('coursegroup:cg-') + 1)
 WHERE content_uri LIKE 'coursegroup:cg-%';

UPDATE votes
   SET target_uri = 'coursegroup:TMPCG-' || substr(target_uri, length('coursegroup:cg-') + 1)
 WHERE target_uri LIKE 'coursegroup:cg-%';

-- (user_bookmarks only references articles; no coursegroup URIs to rewrite.)

-- Polymorphic learned / topic / teaches / prereqs / related tables also
-- key on content_uri; sweep them defensively. (Most entries here will be
-- for tags + articles, so the WHERE clause is cheap.)
UPDATE content_teaches
   SET content_uri = 'coursegroup:TMPCG-' || substr(content_uri, length('coursegroup:cg-') + 1)
 WHERE content_uri LIKE 'coursegroup:cg-%';
UPDATE content_prereqs
   SET content_uri = 'coursegroup:TMPCG-' || substr(content_uri, length('coursegroup:cg-') + 1)
 WHERE content_uri LIKE 'coursegroup:cg-%';
UPDATE content_topics
   SET content_uri = 'coursegroup:TMPCG-' || substr(content_uri, length('coursegroup:cg-') + 1)
 WHERE content_uri LIKE 'coursegroup:cg-%';
UPDATE content_related
   SET content_uri = 'coursegroup:TMPCG-' || substr(content_uri, length('coursegroup:cg-') + 1)
 WHERE content_uri LIKE 'coursegroup:cg-%';

-- entity_patches.entity_id stores the bare id (no prefix), keyed by
-- entity_type. Bump cg-X → TMPCG-X for entity_type='coursegroup' rows.
UPDATE entity_patches
   SET entity_id = 'TMPCG-' || substr(entity_id, 4)
 WHERE entity_type = 'coursegroup' AND entity_id LIKE 'cg-%';

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 3. Rename crs-X → trm-X across every table that holds either a
-- course id (the old per-iteration entity, now a "term") or an FK to one.
-- ───────────────────────────────────────────────────────────────────────────

-- 3a. The per-iteration rows themselves.
UPDATE courses
   SET id = 'trm-' || substr(id, 5)
 WHERE id LIKE 'crs-%';

-- 3b. FK columns on per-iteration child tables (course_id / prereq_course_id).
UPDATE course_series              SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_skill_trees         SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_listings            SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_prerequisites       SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_prerequisites       SET prereq_course_id = 'trm-' || substr(prereq_course_id, 5) WHERE prereq_course_id LIKE 'crs-%';
UPDATE course_textbooks           SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_tags                SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_sessions            SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_ratings             SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_authors             SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_learning_status     SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_session_progress    SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_resources           SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE course_homeworks           SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';
UPDATE articles                   SET course_id        = 'trm-' || substr(course_id, 5)        WHERE course_id        LIKE 'crs-%';

-- 3c. Polymorphic content table: 'course:crs-X' → 'term:trm-X'.
-- Also flip content_type from 'course' (which formerly meant per-iteration)
-- to 'term' to keep the type column semantically aligned with the URI.
UPDATE content
   SET uri = 'term:trm-' || substr(uri, length('course:crs-') + 1),
       content_type = 'term'
 WHERE uri LIKE 'course:crs-%';

UPDATE comments
   SET content_uri = 'term:trm-' || substr(content_uri, length('course:crs-') + 1)
 WHERE content_uri LIKE 'course:crs-%';

UPDATE votes
   SET target_uri = 'term:trm-' || substr(target_uri, length('course:crs-') + 1)
 WHERE target_uri LIKE 'course:crs-%';

UPDATE content_teaches
   SET content_uri = 'term:trm-' || substr(content_uri, length('course:crs-') + 1)
 WHERE content_uri LIKE 'course:crs-%';
UPDATE content_prereqs
   SET content_uri = 'term:trm-' || substr(content_uri, length('course:crs-') + 1)
 WHERE content_uri LIKE 'course:crs-%';
UPDATE content_topics
   SET content_uri = 'term:trm-' || substr(content_uri, length('course:crs-') + 1)
 WHERE content_uri LIKE 'course:crs-%';
UPDATE content_related
   SET content_uri = 'term:trm-' || substr(content_uri, length('course:crs-') + 1)
 WHERE content_uri LIKE 'course:crs-%';

-- 3d. entity_patches: rows with entity_type='course' now describe a term.
UPDATE entity_patches
   SET entity_type = 'term',
       entity_id   = 'trm-' || substr(entity_id, 5)
 WHERE entity_type = 'course' AND entity_id LIKE 'crs-%';

-- Defensive: any 'course' patch row whose id wasn't prefixed crs- still
-- gets reclassified — its semantic meaning is "per-iteration".
UPDATE entity_patches
   SET entity_type = 'term'
 WHERE entity_type = 'course';

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 4. Promote TMPCG-X → crs-X (the new umbrella ids).
-- ───────────────────────────────────────────────────────────────────────────

UPDATE course_groups
   SET id = 'crs-' || substr(id, length('TMPCG-') + 1)
 WHERE id LIKE 'TMPCG-%';

UPDATE courses
   SET group_id = 'crs-' || substr(group_id, length('TMPCG-') + 1)
 WHERE group_id LIKE 'TMPCG-%';

-- Flip the temporary 'coursegroup:TMPCG-X' URIs to the final
-- 'course:crs-X' shape now that crs- means "umbrella".
UPDATE content
   SET uri = 'course:crs-' || substr(uri, length('coursegroup:TMPCG-') + 1),
       content_type = 'course'
 WHERE uri LIKE 'coursegroup:TMPCG-%';

UPDATE comments
   SET content_uri = 'course:crs-' || substr(content_uri, length('coursegroup:TMPCG-') + 1)
 WHERE content_uri LIKE 'coursegroup:TMPCG-%';

UPDATE votes
   SET target_uri = 'course:crs-' || substr(target_uri, length('coursegroup:TMPCG-') + 1)
 WHERE target_uri LIKE 'coursegroup:TMPCG-%';

UPDATE content_teaches
   SET content_uri = 'course:crs-' || substr(content_uri, length('coursegroup:TMPCG-') + 1)
 WHERE content_uri LIKE 'coursegroup:TMPCG-%';
UPDATE content_prereqs
   SET content_uri = 'course:crs-' || substr(content_uri, length('coursegroup:TMPCG-') + 1)
 WHERE content_uri LIKE 'coursegroup:TMPCG-%';
UPDATE content_topics
   SET content_uri = 'course:crs-' || substr(content_uri, length('coursegroup:TMPCG-') + 1)
 WHERE content_uri LIKE 'coursegroup:TMPCG-%';
UPDATE content_related
   SET content_uri = 'course:crs-' || substr(content_uri, length('coursegroup:TMPCG-') + 1)
 WHERE content_uri LIKE 'coursegroup:TMPCG-%';

-- entity_patches: the umbrella becomes the "course" patch target.
UPDATE entity_patches
   SET entity_type = 'course',
       entity_id   = 'crs-' || substr(entity_id, length('TMPCG-') + 1)
 WHERE entity_type = 'coursegroup' AND entity_id LIKE 'TMPCG-%';

UPDATE entity_patches
   SET entity_type = 'course'
 WHERE entity_type = 'coursegroup';

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 5. Rename tables. The old `courses` becomes `terms`; the old
-- `course_groups` becomes the new `courses`. All the per-iteration child
-- tables get the `term_` prefix.
-- ───────────────────────────────────────────────────────────────────────────

-- IMPORTANT: rename old `courses` away first, otherwise renaming
-- `course_groups` into `courses` collides with the still-existing name.
ALTER TABLE courses             RENAME TO terms;
ALTER TABLE course_groups       RENAME TO courses;

-- Per-iteration child tables → term_<thing>.
ALTER TABLE course_series           RENAME TO term_series;
ALTER TABLE course_skill_trees      RENAME TO term_skill_trees;
ALTER TABLE course_listings         RENAME TO term_listings;
ALTER TABLE course_prerequisites    RENAME TO term_prerequisites;
ALTER TABLE course_textbooks        RENAME TO term_textbooks;
ALTER TABLE course_tags             RENAME TO term_tags;
ALTER TABLE course_sessions         RENAME TO term_sessions;
ALTER TABLE course_session_tags     RENAME TO term_session_tags;
ALTER TABLE course_session_prereqs  RENAME TO term_session_prereqs;
ALTER TABLE course_ratings          RENAME TO term_ratings;
ALTER TABLE course_authors          RENAME TO term_authors;
ALTER TABLE course_learning_status  RENAME TO term_learning_status;
ALTER TABLE course_session_progress RENAME TO term_session_progress;
ALTER TABLE course_resources        RENAME TO term_resources;
ALTER TABLE course_homeworks        RENAME TO term_homeworks;

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 6. Rename columns.
--   course_id → term_id wherever it points at the per-iteration table.
--   group_id  → course_id on terms (it now points at the umbrella).
--   prereq_course_id → prereq_term_id on the prereq table.
-- ───────────────────────────────────────────────────────────────────────────

ALTER TABLE term_series           RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_skill_trees      RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_listings         RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_prerequisites    RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_prerequisites    RENAME COLUMN prereq_course_id TO prereq_term_id;
ALTER TABLE term_textbooks        RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_tags             RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_sessions         RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_ratings          RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_authors          RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_learning_status  RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_session_progress RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_resources        RENAME COLUMN course_id        TO term_id;
ALTER TABLE term_homeworks        RENAME COLUMN course_id        TO term_id;

-- terms.group_id (FK to umbrella) → terms.course_id
ALTER TABLE terms                 RENAME COLUMN group_id         TO course_id;

-- articles.course_id has always meant "this article belongs to a specific
-- iteration (now a term)" — rename accordingly. course_session_id likewise
-- points at a session under that iteration (now a term_session).
ALTER TABLE articles              RENAME COLUMN course_id        TO term_id;
ALTER TABLE articles              RENAME COLUMN course_session_id TO term_session_id;

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 7. Re-add foreign keys with their new target / column names.
-- ───────────────────────────────────────────────────────────────────────────

ALTER TABLE term_series
    ADD CONSTRAINT term_series_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_skill_trees
    ADD CONSTRAINT term_skill_trees_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_listings
    ADD CONSTRAINT term_listings_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_prerequisites
    ADD CONSTRAINT term_prerequisites_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE,
    ADD CONSTRAINT term_prerequisites_prereq_term_id_fkey
        FOREIGN KEY (prereq_term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_textbooks
    ADD CONSTRAINT term_textbooks_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_tags
    ADD CONSTRAINT term_tags_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_sessions
    ADD CONSTRAINT term_sessions_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_ratings
    ADD CONSTRAINT term_ratings_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_authors
    ADD CONSTRAINT term_authors_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_learning_status
    ADD CONSTRAINT term_learning_status_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_session_progress
    ADD CONSTRAINT term_session_progress_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_resources
    ADD CONSTRAINT term_resources_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE term_homeworks
    ADD CONSTRAINT term_homeworks_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE CASCADE;

ALTER TABLE articles
    ADD CONSTRAINT articles_term_id_fkey
        FOREIGN KEY (term_id) REFERENCES terms(id) ON DELETE SET NULL;

ALTER TABLE terms
    ADD CONSTRAINT terms_course_id_fkey
        FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE SET NULL;

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 8. Tighten the polymorphic content_type CHECK constraint to the
-- final canonical set. The earlier (loosened) version permitted both
-- 'coursegroup' and 'course'; now we drop the legacy 'coursegroup' value.
-- Belt-and-braces: anything still typed 'coursegroup' (e.g. orphaned rows
-- whose URI swap missed) gets reclassified to 'course' first.
-- ───────────────────────────────────────────────────────────────────────────

UPDATE content SET content_type = 'course' WHERE content_type = 'coursegroup';

ALTER TABLE content DROP CONSTRAINT IF EXISTS content_content_type_check;
ALTER TABLE content ADD  CONSTRAINT content_content_type_check CHECK (
    content_type = ANY (ARRAY[
        'article','series','question','answer','book','chapter',
        'book_series','course','term','paper'
    ])
);

-- Re-add the polymorphic-content FKs that were dropped in Phase 0b. All
-- URI rewrites are complete and the content table now satisfies them.
ALTER TABLE comments
    ADD CONSTRAINT comments_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;
ALTER TABLE content_teaches
    ADD CONSTRAINT content_teaches_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;
ALTER TABLE content_topics
    ADD CONSTRAINT content_topics_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;
ALTER TABLE content_prereqs
    ADD CONSTRAINT content_prereqs_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;
ALTER TABLE content_related
    ADD CONSTRAINT content_related_content_uri_fkey
    FOREIGN KEY (content_uri) REFERENCES content(uri) ON DELETE CASCADE;

-- ───────────────────────────────────────────────────────────────────────────
-- Phase 9. Re-create the indexes whose names baked in "course" / "courses".
-- Postgres carries the original index names through RENAME TABLE, so they
-- now read as e.g. `idx_courses_did` on the `terms` table — confusing.
-- Drop and recreate under the new naming convention.
-- ───────────────────────────────────────────────────────────────────────────

-- terms (was courses)
DROP INDEX IF EXISTS idx_courses_did;
DROP INDEX IF EXISTS idx_courses_published;     -- may already be gone (is_published was dropped)
DROP INDEX IF EXISTS idx_courses_group_id;
CREATE INDEX IF NOT EXISTS idx_terms_did       ON terms(did);
CREATE INDEX IF NOT EXISTS idx_terms_course_id ON terms(course_id);

-- term_series (was course_series)
DROP INDEX IF EXISTS idx_course_series_course;
CREATE INDEX IF NOT EXISTS idx_term_series_term ON term_series(term_id);

-- term_textbooks
DROP INDEX IF EXISTS idx_course_textbooks_course;
CREATE INDEX IF NOT EXISTS idx_term_textbooks_term ON term_textbooks(term_id);

-- term_tags
DROP INDEX IF EXISTS idx_course_tags_tag;
CREATE INDEX IF NOT EXISTS idx_term_tags_tag ON term_tags(tag_id);

-- term_sessions
DROP INDEX IF EXISTS idx_course_sessions_course;
CREATE INDEX IF NOT EXISTS idx_term_sessions_term ON term_sessions(term_id);

-- term_authors
DROP INDEX IF EXISTS idx_course_authors_author;
CREATE INDEX IF NOT EXISTS idx_term_authors_author ON term_authors(author_id);

-- term_learning_status
DROP INDEX IF EXISTS idx_course_learning_status_user;
CREATE INDEX IF NOT EXISTS idx_term_learning_status_user
    ON term_learning_status(user_did, term_id);

-- term_session_progress
DROP INDEX IF EXISTS idx_course_session_progress_user;
CREATE INDEX IF NOT EXISTS idx_term_session_progress_user
    ON term_session_progress(user_did, term_id);

-- term_resources
DROP INDEX IF EXISTS idx_course_resources_course;
CREATE INDEX IF NOT EXISTS idx_term_resources_term ON term_resources(term_id);

-- term_homeworks
DROP INDEX IF EXISTS idx_course_homeworks_course;
DROP INDEX IF EXISTS idx_course_homeworks_session;
CREATE INDEX IF NOT EXISTS idx_term_homeworks_term    ON term_homeworks(term_id, position);
CREATE INDEX IF NOT EXISTS idx_term_homeworks_session ON term_homeworks(session_id) WHERE session_id IS NOT NULL;

-- Articles indexes that referenced the now-renamed columns.
DROP INDEX IF EXISTS idx_articles_course_session;
DROP INDEX IF EXISTS idx_articles_course_session_id;
CREATE INDEX IF NOT EXISTS idx_articles_term_session
    ON articles(term_session_id) WHERE term_session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_articles_term_id
    ON articles(term_id) WHERE term_id IS NOT NULL;

-- term_session_tags / term_session_prereqs unique indexes were renamed by
-- the table rename but their names still mention the old prefix. Refresh.
DROP INDEX IF EXISTS idx_course_tags_group;
DROP INDEX IF EXISTS idx_course_session_tags_group;
DROP INDEX IF EXISTS idx_course_session_prereqs_group;
-- The corresponding new-prefix uniqueness is provided by the PRIMARY KEY
-- on each table (after the 20260421000002 migration), so no new index is
-- needed beyond the per-tag lookup helpers below.
CREATE INDEX IF NOT EXISTS idx_term_tags_term_id            ON term_tags(term_id);
CREATE INDEX IF NOT EXISTS idx_term_session_tags_session    ON term_session_tags(session_id);
CREATE INDEX IF NOT EXISTS idx_term_session_prereqs_session ON term_session_prereqs(session_id);

-- ═══════════════════════════════════════════════════════════════════════════
-- Done.
--
-- Summary of changes:
--   * Tables: courses → terms; course_groups → courses; every other
--     course_<thing> → term_<thing>.
--   * Columns: course_id → term_id (per-iteration FK); group_id →
--     course_id on terms (umbrella FK); prereq_course_id → prereq_term_id.
--   * IDs: crs-X (was iteration) → trm-X; cg-X (was umbrella) → crs-X;
--     ungrouped iterations got auto-generated cg-auto-X (now crs-auto-X)
--     umbrellas mirroring their metadata.
--   * URIs: 'coursegroup:cg-X' → 'course:crs-X'; 'course:crs-X' →
--     'term:trm-X'. All swept across content / comments / votes /
--     bookmarks / content_{teaches,prereqs,topics,related}.
--   * entity_patches.entity_type: 'course' → 'term'; 'coursegroup' →
--     'course'. entity_id rewritten to match.
--   * content_type CHECK constraint adds 'course' + 'term', drops
--     'coursegroup'.
--   * Indexes renamed to follow the new table prefix convention.
-- ═══════════════════════════════════════════════════════════════════════════
