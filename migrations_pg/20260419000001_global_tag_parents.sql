-- Global tag hierarchy (DAG — a tag can have multiple parents).
-- This is the canonical skill tree. `user_tag_tree` stays as the per-user
-- customization layer; queries should UNION both so users can extend but
-- the default structure lives here.
CREATE TABLE tag_parents (
    parent_tag VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    child_tag  VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (parent_tag, child_tag),
    CHECK (parent_tag <> child_tag)
);

CREATE INDEX idx_tag_parents_child  ON tag_parents (child_tag);
CREATE INDEX idx_tag_parents_parent ON tag_parents (parent_tag);

-- Seed from the existing anonymous user_tag_tree, which has been serving
-- as the de-facto global tree (46 rows at migration time). Wipe any
-- self-loops just in case.
INSERT INTO tag_parents (parent_tag, child_tag)
SELECT DISTINCT parent_tag, child_tag
FROM user_tag_tree
WHERE did = 'did:plc:anonymous'
  AND parent_tag <> child_tag
ON CONFLICT DO NOTHING;
