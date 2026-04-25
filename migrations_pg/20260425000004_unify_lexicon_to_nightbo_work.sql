-- ---------------------------------------------------------------------------
-- Unify series records under at.nightbo.work
--
-- Rewrite of 20260425 dropped at.nightbo.series in favour of one
-- at.nightbo.work record per series, but the data was never re-keyed —
-- existing rows still carry repo_uris like
--   at://did:.../at.nightbo.series/s-...
-- which makes server-side resolvers built around the new lexicon (e.g.
-- series_repo_uri) miss every existing series.
--
-- This migration rewrites repo_uris in every table that references them.
-- We bypass FK enforcement for the duration of the transaction
-- (`session_replication_role = replica`) so we can update parent + child
-- tables atomically without dropping/re-adding constraints.
-- ---------------------------------------------------------------------------

BEGIN;

SET LOCAL session_replication_role = replica;

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

-- Synthetic content/vote/notification URIs derived from repo_uri.
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

COMMIT;
