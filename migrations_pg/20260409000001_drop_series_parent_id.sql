-- Remove hierarchical series support: parent_id was never used after the series design was simplified
DROP INDEX IF EXISTS idx_series_parent;
ALTER TABLE series DROP COLUMN IF EXISTS parent_id;
