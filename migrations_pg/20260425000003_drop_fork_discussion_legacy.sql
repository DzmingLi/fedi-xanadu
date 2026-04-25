-- Drop the fork + discussion (PR) features.
--
-- Article forks (`forks` table) and skill-tree forks
-- (`skill_trees.forked_from`) are removed wholesale: no service emits or
-- reads them anymore. The PDS lexicon `at.nightbo.fork` was already
-- inactive and is no longer referenced.
--
-- Discussions were a fork-PR mechanism; without forks they have no entry
-- point so we drop them at the same time.

DROP INDEX IF EXISTS idx_forks_source;
DROP INDEX IF EXISTS idx_forks_forked;
DROP TABLE IF EXISTS forks;

ALTER TABLE skill_trees DROP COLUMN IF EXISTS forked_from;

DROP INDEX IF EXISTS idx_discussions_target;
DROP INDEX IF EXISTS idx_discussions_source;
DROP INDEX IF EXISTS idx_discussion_changes_disc;
DROP TABLE IF EXISTS discussion_changes;
DROP TABLE IF EXISTS discussions;

-- articles_legacy was kept for one release cycle (per the rewrite migration's
-- stated plan) for forensic lookups during the blob storage transition. The
-- rewrite is in production and the pijul drop has been deployed; legacy rows
-- are no longer needed.
DROP TABLE IF EXISTS articles_legacy CASCADE;
