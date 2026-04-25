-- ============================================================================
-- Article translation system rewrite.
--
-- Model shift:
--   Old: one row in `articles` per published record. Translations were tracked
--        via a nullable `translation_group` string that loosely linked rows.
--        Each article (standalone or series chapter) had `lang`, `title`,
--        `description`, `content_*` in-line.
--
--   New: one row in `articles` per *logical article*, keyed by
--        `(repo_uri, source_path)`:
--          - `repo_uri`   = at_uri of the ATProto record that owns the pijul
--                           repo (series record for chapters, source-lang
--                           article record for standalones).
--          - `source_path` = POSIX path of the SOURCE file within that repo.
--        All language versions live in `article_localizations` keyed by
--        `(repo_uri, source_path, lang)`. Each localization has its own
--        ATProto record URI (`at_uri`, nullable for series chapters which
--        publish no per-chapter record).
--
-- Migration strategy:
--   * Legacy rows are migrated 1:1 → one `articles` row + one
--     `article_localizations` row each. Existing `translation_group` links are
--     DROPPED — authors re-establish translations via file-level
--     `translation_of` metadata (enforced by fx-validator). The library is
--     small; manual fix-up is acceptable.
--   * All FK tables that pointed at `articles.at_uri` are restructured to
--     reference `(repo_uri, source_path)`.
--   * The legacy `articles` table is preserved under the name
--     `articles_legacy` for one release cycle to aid forensic lookups;
--     application code must not reference it.
-- ============================================================================

-- Helper: compute the repo_uri + source_path for a legacy article row.
-- Series chapters: repo_uri = at://{series.creator}/at.nightbo.series/{series.id},
--                  source_path = series_articles.repo_path.
-- Standalone: repo_uri = article.at_uri (the article's own record owns its
--             repo), source_path synthesized from content_format
--             ('main.typ' / 'main.md' / etc).
-- Kept as an inline CTE below rather than a persistent function because it
-- runs once. sqlx wraps each migration in its own transaction so no
-- explicit BEGIN/COMMIT here.

-- ---------------------------------------------------------------------------
-- 1. Rename legacy tables for safe backfill. Their FKs will be recreated
--    pointing at the new articles schema once the new data is in.
-- ---------------------------------------------------------------------------

-- Drop the creator_daily_stats materialized view up-front so its references
-- to the old columns (a.at_uri, a.did, b.article_uri, c.content_uri) don't
-- block column drops below. It is recreated in section 7.13 against the new
-- shape.
DROP MATERIALIZED VIEW IF EXISTS creator_daily_stats;

ALTER TABLE articles RENAME TO articles_legacy;
ALTER TABLE articles_legacy RENAME CONSTRAINT articles_pkey TO articles_legacy_pkey;

-- Drop dependents' FKs so we can safely create the new articles table.
-- (FKs to `articles_legacy.at_uri` persist through the rename; we drop them
-- here and re-add them against `articles` below after backfill.)
ALTER TABLE article_authors         DROP CONSTRAINT article_authors_article_uri_fkey;
ALTER TABLE article_versions        DROP CONSTRAINT article_versions_article_uri_fkey;
ALTER TABLE article_access_grants   DROP CONSTRAINT article_access_grants_article_uri_fkey;
ALTER TABLE series_articles         DROP CONSTRAINT series_articles_article_uri_fkey;
ALTER TABLE series_headings         DROP CONSTRAINT series_headings_article_uri_fkey;
ALTER TABLE user_bookmarks          DROP CONSTRAINT user_bookmarks_article_uri_fkey;
ALTER TABLE comments                DROP CONSTRAINT comments_content_uri_fkey;
ALTER TABLE forks                   DROP CONSTRAINT forks_source_uri_fkey;
ALTER TABLE forks                   DROP CONSTRAINT forks_forked_uri_fkey;
ALTER TABLE learned_marks           DROP CONSTRAINT learned_marks_article_uri_fkey;
ALTER TABLE question_merges         DROP CONSTRAINT question_merges_into_uri_fkey;
ALTER TABLE paper_metadata          DROP CONSTRAINT paper_metadata_article_uri_fkey;
ALTER TABLE experience_metadata     DROP CONSTRAINT experience_metadata_article_uri_fkey;

-- Drop the self-FK for question_uri; the new column layout is composite.
ALTER TABLE articles_legacy         DROP CONSTRAINT articles_question_uri_fkey;

-- Drop the content-registry trigger; it will be recreated against the new shape.
DROP TRIGGER IF EXISTS trg_article_content_insert ON articles_legacy;
DROP TRIGGER IF EXISTS trg_article_content_delete ON articles_legacy;
DROP TRIGGER IF EXISTS articles_search_trigger    ON articles_legacy;
DROP TRIGGER IF EXISTS trg_answer_count            ON articles_legacy;

-- Indexes don't rename with the table, so the names on articles_legacy still
-- occupy the global namespace. Drop them now to free the names for the new
-- table's indexes.
DROP INDEX IF EXISTS idx_articles_did;
DROP INDEX IF EXISTS idx_articles_translation_group;
DROP INDEX IF EXISTS idx_articles_visibility;
DROP INDEX IF EXISTS idx_articles_kind;
DROP INDEX IF EXISTS idx_articles_question;
DROP INDEX IF EXISTS idx_articles_book_id;
DROP INDEX IF EXISTS idx_articles_edition_id;
DROP INDEX IF EXISTS idx_articles_fts;
DROP INDEX IF EXISTS idx_articles_title_trgm;
DROP INDEX IF EXISTS idx_articles_description_trgm;
DROP INDEX IF EXISTS idx_articles_book_chapter;
DROP INDEX IF EXISTS idx_articles_course_session;

-- ---------------------------------------------------------------------------
-- 2. New schema.
-- ---------------------------------------------------------------------------

-- The canonical, language-independent article record.
CREATE TABLE articles (
    repo_uri              VARCHAR(512) NOT NULL,
    source_path           TEXT         NOT NULL,
    author_did            VARCHAR(255) NOT NULL,
    license               VARCHAR(100) NOT NULL DEFAULT 'CC-BY-SA-4.0',
    prereq_threshold      DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    kind                  content_kind NOT NULL DEFAULT 'article',
    category              VARCHAR(32)  NOT NULL DEFAULT 'general',
    visibility            VARCHAR(20)  NOT NULL DEFAULT 'public',
    restricted            BOOLEAN      NOT NULL DEFAULT FALSE,
    answer_count          INTEGER      NOT NULL DEFAULT 0,
    removed_at            TIMESTAMPTZ,
    remove_reason         TEXT,
    book_id               VARCHAR(64),
    edition_id            VARCHAR(64),
    course_id             VARCHAR(64),
    book_chapter_id       VARCHAR(64),
    course_session_id     VARCHAR(64),
    cover_url             TEXT,
    cover_file            TEXT,

    -- For answers: the (article) question they answer. Composite self-reference.
    question_repo_uri     VARCHAR(512),
    question_source_path  TEXT,

    created_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    PRIMARY KEY (repo_uri, source_path),

    CONSTRAINT articles_question_ref_shape CHECK (
        (question_repo_uri IS NULL) = (question_source_path IS NULL)
    )
);

CREATE INDEX idx_articles_author ON articles(author_did);
CREATE INDEX idx_articles_visibility ON articles(visibility);
CREATE INDEX idx_articles_kind ON articles(kind);
CREATE INDEX idx_articles_question
    ON articles(question_repo_uri, question_source_path)
    WHERE question_repo_uri IS NOT NULL;
CREATE INDEX idx_articles_book_id    ON articles(book_id)    WHERE book_id IS NOT NULL;
CREATE INDEX idx_articles_edition_id ON articles(edition_id) WHERE edition_id IS NOT NULL;
CREATE INDEX idx_articles_book_chapter   ON articles(book_chapter_id)   WHERE book_chapter_id   IS NOT NULL;
CREATE INDEX idx_articles_course_session ON articles(course_session_id) WHERE course_session_id IS NOT NULL;
CREATE INDEX idx_articles_repo_uri       ON articles(repo_uri);

-- Self-reference FK for question → answer linking.
ALTER TABLE articles
    ADD CONSTRAINT articles_question_fkey
    FOREIGN KEY (question_repo_uri, question_source_path)
    REFERENCES articles(repo_uri, source_path)
    ON DELETE CASCADE;

-- Per-language content + metadata. One row per (article, language).
CREATE TABLE article_localizations (
    repo_uri          VARCHAR(512) NOT NULL,
    source_path       TEXT         NOT NULL,
    lang              VARCHAR(32)  NOT NULL,
    -- ATProto record URI for THIS language. NULL for series chapters (the
    -- series lexicon forbids per-chapter records) and for translations that
    -- haven't been published yet.
    at_uri            VARCHAR(512),
    file_path         TEXT         NOT NULL,
    content_format    content_format NOT NULL DEFAULT 'typst',
    title             VARCHAR(500) NOT NULL,
    summary           TEXT         NOT NULL DEFAULT '',
    summary_html      TEXT         NOT NULL DEFAULT '',
    content_hash      VARCHAR(128),
    content_storage   VARCHAR(16)  NOT NULL DEFAULT 'pijul',
    content_manifest  JSONB,
    translator_did    VARCHAR(255),
    translation_notes TEXT,
    rendered_cache    TEXT,
    search_vector     TSVECTOR,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    PRIMARY KEY (repo_uri, source_path, lang),

    FOREIGN KEY (repo_uri, source_path)
        REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE,

    CONSTRAINT localizations_storage_valid
        CHECK (content_storage IN ('pijul', 'blob')),
    CONSTRAINT localizations_blob_manifest_required
        CHECK (content_storage <> 'blob' OR content_manifest IS NOT NULL)
);

-- at_uri is unique when set, but may be NULL for unpublished or series chapters.
CREATE UNIQUE INDEX idx_localizations_at_uri
    ON article_localizations(at_uri) WHERE at_uri IS NOT NULL;
CREATE INDEX idx_localizations_lang ON article_localizations(lang);
CREATE INDEX idx_localizations_fts ON article_localizations USING GIN(search_vector);
CREATE INDEX idx_localizations_title_trgm
    ON article_localizations USING GIN(title gin_trgm_ops);
CREATE INDEX idx_localizations_summary_trgm
    ON article_localizations USING GIN(summary gin_trgm_ops);

-- ---------------------------------------------------------------------------
-- 3. Search trigger — now on localizations (per-language text).
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION localizations_search_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('simple', coalesce(NEW.title, '')),   'A') ||
        setweight(to_tsvector('simple', coalesce(NEW.summary, '')), 'B');
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_localizations_search
    BEFORE INSERT OR UPDATE ON article_localizations
    FOR EACH ROW EXECUTE FUNCTION localizations_search_update();

-- ---------------------------------------------------------------------------
-- 4. Answer count: now decoupled from at_uri; maintained on articles by
--    joining question_repo_uri + question_source_path.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION update_answer_count() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.question_repo_uri IS NOT NULL THEN
        UPDATE articles
           SET answer_count = answer_count + 1
         WHERE repo_uri = NEW.question_repo_uri
           AND source_path = NEW.question_source_path;
    ELSIF TG_OP = 'DELETE' AND OLD.question_repo_uri IS NOT NULL THEN
        UPDATE articles
           SET answer_count = GREATEST(answer_count - 1, 0)
         WHERE repo_uri = OLD.question_repo_uri
           AND source_path = OLD.question_source_path;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trg_answer_count
    AFTER INSERT OR DELETE ON articles
    FOR EACH ROW EXECUTE FUNCTION update_answer_count();

-- ---------------------------------------------------------------------------
-- 5. Content-URI registry. In the old model, every article's at_uri was a
--    `content.uri`. In the new model, every localization's at_uri (when it
--    has one) is a content.uri. Plus, each logical article gets a synthetic
--    `nightboat://article/{repo_uri}/{source_path}` URI for article-scoped
--    references (comments, bookmarks, votes, etc.).
--
--    The synthetic URI is chosen over exposing the composite key because:
--      * Existing `content.uri` is TEXT — already accepts strings.
--      * Comments/votes/bookmarks tables use a single URI column today;
--        keeping that shape avoids rewriting those tables entirely.
--      * The `nightboat://` scheme makes intent obvious in logs & dumps.
-- ---------------------------------------------------------------------------

-- Function: construct the synthetic article URI.
CREATE OR REPLACE FUNCTION article_uri(p_repo_uri TEXT, p_source_path TEXT)
RETURNS TEXT LANGUAGE sql IMMUTABLE AS $$
    SELECT 'nightboat://article/' || p_repo_uri || '/' || p_source_path;
$$;

-- Register each article in content on insert.
CREATE OR REPLACE FUNCTION content_insert_article() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO content(uri, content_type)
    VALUES (article_uri(NEW.repo_uri, NEW.source_path),
            CASE NEW.kind::text
                WHEN 'question' THEN 'question'
                WHEN 'answer'   THEN 'answer'
                ELSE 'article'
            END)
    ON CONFLICT (uri) DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION content_delete_article() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM content WHERE uri = article_uri(OLD.repo_uri, OLD.source_path);
    RETURN OLD;
END;
$$;

CREATE TRIGGER trg_article_content_insert
    AFTER INSERT ON articles FOR EACH ROW EXECUTE FUNCTION content_insert_article();

CREATE TRIGGER trg_article_content_delete
    BEFORE DELETE ON articles FOR EACH ROW EXECUTE FUNCTION content_delete_article();

-- ---------------------------------------------------------------------------
-- 6. Backfill: legacy articles → articles + article_localizations.
-- ---------------------------------------------------------------------------

-- Step 6a. Build a staging view that maps each legacy article to its
-- (repo_uri, source_path). Series chapters use the series's synthesized
-- at_uri; standalone articles use their own at_uri.
WITH series_uri AS (
    -- Synthesize at_uri for each series from its creator DID + id rkey.
    SELECT s.id                                    AS series_id,
           'at://' || s.created_by ||
           '/at.nightbo.series/' || s.id           AS series_at_uri
      FROM series s
),
legacy_mapped AS (
    SELECT
        al.at_uri                                  AS legacy_at_uri,
        al.did                                     AS author_did,
        al.title,
        al.summary,
        al.summary_html,
        al.cover_url,
        al.cover_file,
        al.content_hash,
        al.content_format,
        al.content_storage,
        al.content_manifest,
        al.lang,
        al.license,
        al.prereq_threshold,
        al.created_at,
        al.updated_at,
        al.search_vector,
        al.visibility,
        al.removed_at,
        al.remove_reason,
        al.kind,
        al.question_uri                            AS legacy_question_uri,
        al.answer_count,
        al.restricted,
        al.category,
        al.book_id,
        al.edition_id,
        al.course_id,
        al.book_chapter_id,
        al.course_session_id,
        sa.repo_path                               AS series_repo_path,
        su.series_at_uri                           AS series_at_uri,
        CASE
            WHEN sa.series_id IS NOT NULL THEN su.series_at_uri
            ELSE al.at_uri
        END                                        AS new_repo_uri,
        CASE
            WHEN sa.series_id IS NOT NULL THEN COALESCE(sa.repo_path, al.at_uri)
            ELSE 'main.' || CASE al.content_format::text
                              WHEN 'markdown' THEN 'md'
                              WHEN 'html'     THEN 'html'
                              WHEN 'tex'      THEN 'tex'
                              ELSE 'typ'
                            END
        END                                        AS new_source_path
      FROM articles_legacy al
      LEFT JOIN series_articles sa ON sa.article_uri = al.at_uri
      LEFT JOIN series_uri     su  ON su.series_id    = sa.series_id
)
INSERT INTO articles (
    repo_uri, source_path, author_did, license, prereq_threshold,
    kind, category, visibility, restricted, answer_count,
    removed_at, remove_reason,
    book_id, edition_id, course_id, book_chapter_id, course_session_id,
    cover_url, cover_file,
    created_at, updated_at
)
SELECT DISTINCT ON (new_repo_uri, new_source_path)
    new_repo_uri, new_source_path, author_did, license, prereq_threshold,
    kind, category, visibility, restricted, answer_count,
    removed_at, remove_reason,
    book_id, edition_id, course_id, book_chapter_id, course_session_id,
    cover_url, cover_file,
    created_at, updated_at
  FROM legacy_mapped
 ORDER BY new_repo_uri, new_source_path, created_at;

-- Step 6b. Localizations: one row per legacy article. The legacy row's
-- at_uri becomes this language's at_uri; the file_path equals the source_path
-- since before this migration there was only one language per article anyway.
WITH series_uri AS (
    SELECT s.id AS series_id,
           'at://' || s.created_by || '/at.nightbo.series/' || s.id AS series_at_uri
      FROM series s
)
INSERT INTO article_localizations (
    repo_uri, source_path, lang, at_uri, file_path,
    content_format, title, summary, summary_html, content_hash,
    content_storage, content_manifest,
    translator_did, translation_notes, rendered_cache, search_vector,
    created_at, updated_at
)
SELECT
    CASE WHEN sa.series_id IS NOT NULL THEN su.series_at_uri ELSE al.at_uri END
        AS repo_uri,
    CASE WHEN sa.series_id IS NOT NULL THEN COALESCE(sa.repo_path, al.at_uri)
         ELSE 'main.' || CASE al.content_format::text
                          WHEN 'markdown' THEN 'md'
                          WHEN 'html'     THEN 'html'
                          WHEN 'tex'      THEN 'tex'
                          ELSE 'typ'
                        END
    END AS source_path,
    COALESCE(NULLIF(al.lang, ''), 'zh') AS lang,
    -- Series chapters publish no per-chapter ATProto record; null their at_uri.
    CASE WHEN sa.series_id IS NOT NULL THEN NULL ELSE al.at_uri END AS at_uri,
    CASE WHEN sa.series_id IS NOT NULL THEN COALESCE(sa.repo_path, al.at_uri)
         ELSE 'main.' || CASE al.content_format::text
                          WHEN 'markdown' THEN 'md'
                          WHEN 'html'     THEN 'html'
                          WHEN 'tex'      THEN 'tex'
                          ELSE 'typ'
                        END
    END AS file_path,
    al.content_format, al.title, al.summary, al.summary_html, al.content_hash,
    al.content_storage, al.content_manifest,
    NULL::VARCHAR(255)  AS translator_did,
    NULL::TEXT          AS translation_notes,
    NULL::TEXT          AS rendered_cache,
    al.search_vector,
    al.created_at, al.updated_at
  FROM articles_legacy al
  LEFT JOIN series_articles sa ON sa.article_uri = al.at_uri
  LEFT JOIN series_uri     su  ON su.series_id    = sa.series_id;

-- Step 6c. Backfill question references.
WITH series_uri AS (
    SELECT s.id AS series_id,
           'at://' || s.created_by || '/at.nightbo.series/' || s.id AS series_at_uri
      FROM series s
),
legacy_qref AS (
    SELECT al.at_uri AS answer_legacy_uri,
           CASE WHEN asa.series_id IS NOT NULL THEN asu.series_at_uri ELSE al.at_uri END
               AS answer_repo_uri,
           CASE WHEN asa.series_id IS NOT NULL THEN COALESCE(asa.repo_path, al.at_uri)
                ELSE 'main.' || CASE al.content_format::text
                                 WHEN 'markdown' THEN 'md'
                                 WHEN 'html'     THEN 'html'
                                 WHEN 'tex'      THEN 'tex'
                                 ELSE 'typ'
                               END
           END AS answer_source_path,
           CASE WHEN qsa.series_id IS NOT NULL THEN qsu.series_at_uri ELSE ql.at_uri END
               AS q_repo_uri,
           CASE WHEN qsa.series_id IS NOT NULL THEN COALESCE(qsa.repo_path, ql.at_uri)
                ELSE 'main.' || CASE ql.content_format::text
                                 WHEN 'markdown' THEN 'md'
                                 WHEN 'html'     THEN 'html'
                                 WHEN 'tex'      THEN 'tex'
                                 ELSE 'typ'
                               END
           END AS q_source_path
      FROM articles_legacy al
      JOIN articles_legacy ql ON ql.at_uri = al.question_uri
      LEFT JOIN series_articles asa ON asa.article_uri = al.at_uri
      LEFT JOIN series_uri       asu ON asu.series_id    = asa.series_id
      LEFT JOIN series_articles qsa ON qsa.article_uri = ql.at_uri
      LEFT JOIN series_uri       qsu ON qsu.series_id    = qsa.series_id
     WHERE al.question_uri IS NOT NULL
)
UPDATE articles a
   SET question_repo_uri    = lq.q_repo_uri,
       question_source_path = lq.q_source_path
  FROM legacy_qref lq
 WHERE a.repo_uri    = lq.answer_repo_uri
   AND a.source_path = lq.answer_source_path;

-- ---------------------------------------------------------------------------
-- 7. Migrate FK tables: replace `article_uri` single-column FKs with
--    composite `(repo_uri, source_path)` FKs.
--
--    Each table follows the same pattern:
--      1. ADD composite columns.
--      2. Populate via JOIN with articles_legacy (which still has the
--         old at_uri → new key mapping implicitly via backfill above).
--         We re-derive via series_uri / article repo mapping.
--      3. DROP old column.
--      4. ADD composite FK.
-- ---------------------------------------------------------------------------

-- A reusable mapping from legacy at_uri → new (repo_uri, source_path). Use a
-- TEMP TABLE rather than a VIEW: later ALTER TABLE DROP COLUMN on
-- series_articles.article_uri would invalidate a view definition, whereas the
-- table is materialized up-front and survives the column changes.
CREATE TEMPORARY TABLE legacy_to_new ON COMMIT DROP AS
WITH series_uri AS (
    SELECT s.id AS series_id,
           'at://' || s.created_by || '/at.nightbo.series/' || s.id AS series_at_uri
      FROM series s
)
SELECT
    al.at_uri AS legacy_at_uri,
    CASE WHEN sa.series_id IS NOT NULL THEN su.series_at_uri ELSE al.at_uri END
        AS repo_uri,
    CASE WHEN sa.series_id IS NOT NULL THEN COALESCE(sa.repo_path, al.at_uri)
         ELSE 'main.' || CASE al.content_format::text
                          WHEN 'markdown' THEN 'md'
                          WHEN 'html'     THEN 'html'
                          WHEN 'tex'      THEN 'tex'
                          ELSE 'typ'
                        END
    END AS source_path
  FROM articles_legacy al
  LEFT JOIN series_articles sa ON sa.article_uri = al.at_uri
  LEFT JOIN series_uri     su  ON su.series_id    = sa.series_id;

CREATE UNIQUE INDEX ON legacy_to_new(legacy_at_uri);

-- 7.1 article_authors
ALTER TABLE article_authors ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE article_authors ADD COLUMN source_path TEXT;
UPDATE article_authors aa
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = aa.article_uri;
ALTER TABLE article_authors DROP COLUMN article_uri;
ALTER TABLE article_authors ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE article_authors ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE article_authors
    ADD CONSTRAINT article_authors_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_article_authors_status;
DROP INDEX IF EXISTS idx_article_authors_unique_did;
CREATE INDEX idx_article_authors_ref ON article_authors(repo_uri, source_path);
CREATE UNIQUE INDEX idx_article_authors_unique_did
    ON article_authors(repo_uri, source_path, author_did)
    WHERE author_did IS NOT NULL;

-- 7.2 article_versions
ALTER TABLE article_versions ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE article_versions ADD COLUMN source_path TEXT;
UPDATE article_versions av
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = av.article_uri;
ALTER TABLE article_versions DROP COLUMN article_uri;
ALTER TABLE article_versions ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE article_versions ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE article_versions
    ADD CONSTRAINT article_versions_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_article_versions_uri;
CREATE INDEX idx_article_versions_ref
    ON article_versions(repo_uri, source_path, created_at DESC);

-- 7.3 article_access_grants
ALTER TABLE article_access_grants ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE article_access_grants ADD COLUMN source_path TEXT;
UPDATE article_access_grants aag
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = aag.article_uri;
ALTER TABLE article_access_grants DROP CONSTRAINT article_access_grants_pkey;
ALTER TABLE article_access_grants DROP COLUMN article_uri;
ALTER TABLE article_access_grants ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE article_access_grants ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE article_access_grants
    ADD CONSTRAINT article_access_grants_pkey
    PRIMARY KEY (repo_uri, source_path, grantee_did);
ALTER TABLE article_access_grants
    ADD CONSTRAINT article_access_grants_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

-- 7.4 series_articles — drop article_uri in favor of composite; order_index,
--     heading_title, heading_anchor remain.
ALTER TABLE series_articles ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE series_articles ADD COLUMN source_path TEXT;
UPDATE series_articles sa
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = sa.article_uri;
ALTER TABLE series_articles DROP CONSTRAINT series_articles_pkey;
ALTER TABLE series_articles DROP CONSTRAINT unique_article_in_series;
ALTER TABLE series_articles DROP COLUMN article_uri;
ALTER TABLE series_articles DROP COLUMN repo_path;     -- superseded by source_path
ALTER TABLE series_articles ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE series_articles ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE series_articles
    ADD CONSTRAINT series_articles_pkey
    PRIMARY KEY (series_id, repo_uri, source_path);
ALTER TABLE series_articles
    ADD CONSTRAINT series_articles_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

-- 7.5 series_headings (nullable FK)
ALTER TABLE series_headings ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE series_headings ADD COLUMN source_path TEXT;
UPDATE series_headings sh
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = sh.article_uri;
ALTER TABLE series_headings DROP COLUMN article_uri;
ALTER TABLE series_headings
    ADD CONSTRAINT series_headings_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE SET NULL;

-- 7.6 user_bookmarks
ALTER TABLE user_bookmarks ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE user_bookmarks ADD COLUMN source_path TEXT;
UPDATE user_bookmarks ub
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = ub.article_uri;
ALTER TABLE user_bookmarks DROP CONSTRAINT user_bookmarks_pkey;
ALTER TABLE user_bookmarks DROP COLUMN article_uri;
ALTER TABLE user_bookmarks ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE user_bookmarks ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE user_bookmarks
    ADD CONSTRAINT user_bookmarks_pkey PRIMARY KEY (did, repo_uri, source_path);
ALTER TABLE user_bookmarks
    ADD CONSTRAINT user_bookmarks_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

-- 7.6a votes (polymorphic target_uri). Article votes keyed by old at_uri
-- are rewritten to the synthetic article URI so downstream code can JOIN
-- consistently via `article_uri(repo_uri, source_path)`. Non-article
-- target_uris (comments, series, etc.) are left alone.
UPDATE votes v
   SET target_uri = article_uri(lm.repo_uri, lm.source_path)
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = v.target_uri;

-- 7.7 comments (polymorphic: article / book / chapter / book_series).
-- Rewrite the article subset's content_uri to the synthetic per-article URI
-- so content-registry joins still work. We do NOT introduce composite columns
-- here — comments remain keyed by their generic `content_uri` string, which
-- now carries the synthetic URI scheme for articles.
UPDATE comments c
   SET content_uri = article_uri(lm.repo_uri, lm.source_path)
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = c.content_uri;

-- 7.8 forks
ALTER TABLE forks ADD COLUMN source_repo_uri    VARCHAR(512);
ALTER TABLE forks ADD COLUMN source_source_path TEXT;
ALTER TABLE forks ADD COLUMN forked_repo_uri    VARCHAR(512);
ALTER TABLE forks ADD COLUMN forked_source_path TEXT;
UPDATE forks f
   SET source_repo_uri    = lm.repo_uri,
       source_source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = f.source_uri;
UPDATE forks f
   SET forked_repo_uri    = lm.repo_uri,
       forked_source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = f.forked_uri;
ALTER TABLE forks DROP COLUMN source_uri;
ALTER TABLE forks DROP COLUMN forked_uri;
ALTER TABLE forks ALTER COLUMN source_repo_uri    SET NOT NULL;
ALTER TABLE forks ALTER COLUMN source_source_path SET NOT NULL;
ALTER TABLE forks ALTER COLUMN forked_repo_uri    SET NOT NULL;
ALTER TABLE forks ALTER COLUMN forked_source_path SET NOT NULL;
ALTER TABLE forks
    ADD CONSTRAINT forks_source_fkey
    FOREIGN KEY (source_repo_uri, source_source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
ALTER TABLE forks
    ADD CONSTRAINT forks_forked_fkey
    FOREIGN KEY (forked_repo_uri, forked_source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_forks_source_uri;
DROP INDEX IF EXISTS idx_forks_forked_uri;
CREATE INDEX idx_forks_source ON forks(source_repo_uri, source_source_path);
CREATE INDEX idx_forks_forked ON forks(forked_repo_uri, forked_source_path);

-- 7.9 learned_marks
ALTER TABLE learned_marks ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE learned_marks ADD COLUMN source_path TEXT;
UPDATE learned_marks lmk
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = lmk.article_uri;
ALTER TABLE learned_marks DROP CONSTRAINT learned_marks_pkey;
ALTER TABLE learned_marks DROP COLUMN article_uri;
ALTER TABLE learned_marks ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE learned_marks ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE learned_marks
    ADD CONSTRAINT learned_marks_pkey PRIMARY KEY (did, repo_uri, source_path);
ALTER TABLE learned_marks
    ADD CONSTRAINT learned_marks_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

-- 7.10 question_merges
ALTER TABLE question_merges ADD COLUMN into_repo_uri    VARCHAR(512);
ALTER TABLE question_merges ADD COLUMN into_source_path TEXT;
UPDATE question_merges qm
   SET into_repo_uri    = lm.repo_uri,
       into_source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = qm.into_uri;
ALTER TABLE question_merges DROP COLUMN into_uri;
ALTER TABLE question_merges ALTER COLUMN into_repo_uri    SET NOT NULL;
ALTER TABLE question_merges ALTER COLUMN into_source_path SET NOT NULL;
ALTER TABLE question_merges
    ADD CONSTRAINT question_merges_into_fkey
    FOREIGN KEY (into_repo_uri, into_source_path)
    REFERENCES articles(repo_uri, source_path);

-- 7.11 article_views (creator dashboard raw events)
ALTER TABLE article_views ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE article_views ADD COLUMN source_path TEXT;
UPDATE article_views av
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = av.article_uri;
ALTER TABLE article_views DROP COLUMN article_uri;
ALTER TABLE article_views ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE article_views ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE article_views
    ADD CONSTRAINT article_views_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_article_views_uri;
CREATE INDEX idx_article_views_ref ON article_views(repo_uri, source_path);

-- 7.12 article_collaborators (no FK but same rewrite)
ALTER TABLE article_collaborators ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE article_collaborators ADD COLUMN source_path TEXT;
UPDATE article_collaborators ac
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = ac.article_uri;
ALTER TABLE article_collaborators DROP CONSTRAINT article_collaborators_pkey;
ALTER TABLE article_collaborators DROP COLUMN article_uri;
ALTER TABLE article_collaborators ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE article_collaborators ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE article_collaborators
    ADD CONSTRAINT article_collaborators_pkey
    PRIMARY KEY (repo_uri, source_path, user_did);
ALTER TABLE article_collaborators
    ADD CONSTRAINT article_collaborators_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_article_collab_uri;
CREATE INDEX idx_article_collab_ref ON article_collaborators(repo_uri, source_path);

-- 7.13 creator_daily_stats materialized view: rebuild with new columns
-- (dropped up-front in section 1 so that column drops along the way could
-- proceed without dependency errors).
CREATE MATERIALIZED VIEW creator_daily_stats AS
SELECT creator_did, day,
       SUM(views)::bigint     AS views,
       SUM(comments)::bigint  AS comments,
       SUM(bookmarks)::bigint AS bookmarks
FROM (
    SELECT a.author_did AS creator_did, DATE(av.viewed_at) AS day,
           COUNT(av.id) AS views, 0::bigint AS comments, 0::bigint AS bookmarks
      FROM articles a
      JOIN article_views av
        ON av.repo_uri = a.repo_uri AND av.source_path = a.source_path
     WHERE a.removed_at IS NULL
     GROUP BY a.author_did, DATE(av.viewed_at)

    UNION ALL

    SELECT a.author_did, DATE(c.created_at),
           0, COUNT(c.id), 0
      FROM articles a
      JOIN comments c ON c.content_uri = article_uri(a.repo_uri, a.source_path)
     WHERE a.removed_at IS NULL
     GROUP BY a.author_did, DATE(c.created_at)

    UNION ALL

    SELECT a.author_did, DATE(b.created_at),
           0, 0, COUNT(*)
      FROM articles a
      JOIN user_bookmarks b
        ON b.repo_uri = a.repo_uri AND b.source_path = a.source_path
     WHERE a.removed_at IS NULL
     GROUP BY a.author_did, DATE(b.created_at)
) sub
GROUP BY creator_did, day;

CREATE UNIQUE INDEX idx_creator_daily_stats ON creator_daily_stats(creator_did, day);

-- 7.14 paper_metadata / experience_metadata (1:1 with article)
ALTER TABLE paper_metadata ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE paper_metadata ADD COLUMN source_path TEXT;
UPDATE paper_metadata pm
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = pm.article_uri;
ALTER TABLE paper_metadata DROP CONSTRAINT paper_metadata_pkey;
ALTER TABLE paper_metadata DROP COLUMN article_uri;
ALTER TABLE paper_metadata ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE paper_metadata ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE paper_metadata
    ADD CONSTRAINT paper_metadata_pkey PRIMARY KEY (repo_uri, source_path);
ALTER TABLE paper_metadata
    ADD CONSTRAINT paper_metadata_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

ALTER TABLE experience_metadata ADD COLUMN repo_uri    VARCHAR(512);
ALTER TABLE experience_metadata ADD COLUMN source_path TEXT;
UPDATE experience_metadata em
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = em.article_uri;
ALTER TABLE experience_metadata DROP CONSTRAINT experience_metadata_pkey;
ALTER TABLE experience_metadata DROP COLUMN article_uri;
ALTER TABLE experience_metadata ALTER COLUMN repo_uri    SET NOT NULL;
ALTER TABLE experience_metadata ALTER COLUMN source_path SET NOT NULL;
ALTER TABLE experience_metadata
    ADD CONSTRAINT experience_metadata_pkey PRIMARY KEY (repo_uri, source_path);
ALTER TABLE experience_metadata
    ADD CONSTRAINT experience_metadata_article_fkey
    FOREIGN KEY (repo_uri, source_path)
    REFERENCES articles(repo_uri, source_path) ON DELETE CASCADE;

-- 7.12 series_article_prereqs (no FK to articles — strings only — but still
--      needs to be rewritten to composite)
ALTER TABLE series_article_prereqs ADD COLUMN repo_uri        VARCHAR(512);
ALTER TABLE series_article_prereqs ADD COLUMN source_path     TEXT;
ALTER TABLE series_article_prereqs ADD COLUMN prereq_repo_uri VARCHAR(512);
ALTER TABLE series_article_prereqs ADD COLUMN prereq_source_path TEXT;
UPDATE series_article_prereqs sap
   SET repo_uri    = lm.repo_uri,
       source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = sap.article_uri;
UPDATE series_article_prereqs sap
   SET prereq_repo_uri    = lm.repo_uri,
       prereq_source_path = lm.source_path
  FROM legacy_to_new lm
 WHERE lm.legacy_at_uri = sap.prereq_article_uri;
ALTER TABLE series_article_prereqs DROP CONSTRAINT series_article_prereqs_pkey;
ALTER TABLE series_article_prereqs DROP COLUMN article_uri;
ALTER TABLE series_article_prereqs DROP COLUMN prereq_article_uri;
ALTER TABLE series_article_prereqs ALTER COLUMN repo_uri           SET NOT NULL;
ALTER TABLE series_article_prereqs ALTER COLUMN source_path        SET NOT NULL;
ALTER TABLE series_article_prereqs ALTER COLUMN prereq_repo_uri    SET NOT NULL;
ALTER TABLE series_article_prereqs ALTER COLUMN prereq_source_path SET NOT NULL;
ALTER TABLE series_article_prereqs
    ADD CONSTRAINT series_article_prereqs_pkey
    PRIMARY KEY (series_id, repo_uri, source_path, prereq_repo_uri, prereq_source_path);

-- ---------------------------------------------------------------------------
-- 8. Drop legacy-only structures.
-- ---------------------------------------------------------------------------

-- NOTE: `articles_legacy` stays around for one release cycle for forensic
-- lookups. Its indexes were dropped earlier (to free their names for the new
-- table); the unindexed table is fine for ad-hoc queries during a rollback
-- window and will be removed in a follow-up migration. Application code must
-- not reference this table — any query is a bug.

-- ---------------------------------------------------------------------------
-- 9. Also rewrite series.translation_group (same concept, same fate).
--     Translations of a series are not yet modeled; this column is legacy.
-- ---------------------------------------------------------------------------
DROP INDEX IF EXISTS idx_series_translation_group;
ALTER TABLE series DROP COLUMN IF EXISTS translation_group;
