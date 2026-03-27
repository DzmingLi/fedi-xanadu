-- Rename skill_trees.field to skill_trees.tag_id (now stores a tag ID instead of hardcoded field name)
ALTER TABLE skill_trees RENAME COLUMN field TO tag_id;

-- Update the index name to match
DROP INDEX IF EXISTS idx_skill_trees_field;
CREATE INDEX idx_skill_trees_tag_id ON skill_trees (tag_id);
