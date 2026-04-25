-- ---------------------------------------------------------------------------
-- Unify series records under at.nightbo.work
--
-- Rewrite of 20260425 dropped at.nightbo.series in favour of one
-- at.nightbo.work record per series, but the data was never re-keyed —
-- existing rows still carry repo_uris like
--   at://did:.../at.nightbo.series/s-...
-- which makes server-side resolvers built around the new lexicon (e.g.
-- series_repo_uri) miss every existing series and chapter URI lookups
-- 404. This migration rewrites the lexicon segment in every column that
-- references it.
--
-- The articles PK is (repo_uri, source_path), and 14 FK constraints reference
-- it. To update parent + children atomically we mark those FKs DEFERRABLE
-- INITIALLY IMMEDIATE (a metadata-only change), then `SET CONSTRAINTS ALL
-- DEFERRED` so PG validates referential integrity at COMMIT time once every
-- table has been rewritten consistently.
-- ---------------------------------------------------------------------------

-- Step 1: make the FKs that reference articles deferrable.
-- (sqlx wraps each migration in its own implicit transaction; this is safe
-- to run inside that transaction.)

ALTER TABLE article_access_grants
    ALTER CONSTRAINT article_access_grants_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE article_authors
    ALTER CONSTRAINT article_authors_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE article_collaborators
    ALTER CONSTRAINT article_collaborators_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE article_localizations
    ALTER CONSTRAINT article_localizations_repo_uri_source_path_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE article_versions
    ALTER CONSTRAINT article_versions_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE article_views
    ALTER CONSTRAINT article_views_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE articles
    ALTER CONSTRAINT articles_question_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE experience_metadata
    ALTER CONSTRAINT experience_metadata_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE learned_marks
    ALTER CONSTRAINT learned_marks_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE paper_metadata
    ALTER CONSTRAINT paper_metadata_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE question_merges
    ALTER CONSTRAINT question_merges_into_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE series_articles
    ALTER CONSTRAINT series_articles_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE series_headings
    ALTER CONSTRAINT series_headings_article_fkey DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE user_bookmarks
    ALTER CONSTRAINT user_bookmarks_article_fkey DEFERRABLE INITIALLY IMMEDIATE;

-- Step 2: defer FK validation until commit so we can update parent and child
-- rows in any order during this transaction.
SET CONSTRAINTS ALL DEFERRED;

-- Step 3: rewrite the lexicon segment everywhere it appears.

UPDATE articles
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE articles
   SET question_repo_uri = REPLACE(question_repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE question_repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_localizations
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_authors
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_collaborators
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_versions
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_views
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE article_access_grants
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE experience_metadata
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE learned_marks
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE paper_metadata
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE series_articles
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE series_article_prereqs
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE series_article_prereqs
   SET prereq_repo_uri = REPLACE(prereq_repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE prereq_repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE series_headings
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE user_bookmarks
   SET repo_uri = REPLACE(repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE repo_uri LIKE 'at://%/at.nightbo.series/%';

UPDATE question_merges
   SET into_repo_uri = REPLACE(into_repo_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE into_repo_uri LIKE 'at://%/at.nightbo.series/%';

-- Synthetic content/vote/notification URIs derived from repo_uri. content has
-- its own PK (uri) referenced by content_teaches/content_prereqs/etc; those
-- child tables have zero old-lexicon rows so updating content.uri is safe.
UPDATE content
   SET uri = REPLACE(uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE uri LIKE 'nightboat://article/at://%/at.nightbo.series/%';

UPDATE votes
   SET target_uri = REPLACE(target_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE target_uri LIKE 'nightboat://article/at://%/at.nightbo.series/%';

UPDATE notifications
   SET target_uri = REPLACE(target_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE target_uri LIKE 'nightboat://article/at://%/at.nightbo.series/%';

UPDATE reports
   SET target_uri = REPLACE(target_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE target_uri LIKE 'nightboat://article/at://%/at.nightbo.series/%';

UPDATE appeals
   SET target_uri = REPLACE(target_uri, '/at.nightbo.series/', '/at.nightbo.work/')
 WHERE target_uri LIKE 'nightboat://article/at://%/at.nightbo.series/%';

-- Constraints stay marked DEFERRABLE INITIALLY IMMEDIATE — same enforcement as
-- before for ordinary writes, and lets future migrations defer if they need to.
