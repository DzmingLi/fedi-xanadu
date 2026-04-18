-- Global tag hierarchy — "belongs to" relationships between tags.
-- Distinct from user_tag_prereqs (which is subjective learning order per
-- user). Hierarchy is objective: real-numbers IS part of calculus.
-- Anyone logged in may edit; tag_parent_edits preserves history.
CREATE TABLE tag_parents (
    parent_tag VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    child_tag  VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (parent_tag, child_tag),
    CHECK (parent_tag <> child_tag)
);
CREATE INDEX idx_tag_parents_child  ON tag_parents (child_tag);
CREATE INDEX idx_tag_parents_parent ON tag_parents (parent_tag);

CREATE TABLE tag_parent_edits (
    id          BIGSERIAL PRIMARY KEY,
    parent_tag  VARCHAR(255) NOT NULL,
    child_tag   VARCHAR(255) NOT NULL,
    action      VARCHAR(10)  NOT NULL CHECK (action IN ('add', 'remove')),
    editor_did  VARCHAR(255) NOT NULL,
    edited_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tag_parent_edits_tag ON tag_parent_edits (parent_tag, child_tag);
CREATE INDEX idx_tag_parent_edits_by  ON tag_parent_edits (editor_did);

-- Seed from the anonymous user_tag_tree (the de-facto default hierarchy
-- so far — 46 rows). Self-loops rejected by the CHECK constraint.
INSERT INTO tag_parents (parent_tag, child_tag)
SELECT DISTINCT parent_tag, child_tag
FROM user_tag_tree
WHERE did = 'did:plc:anonymous'
  AND parent_tag <> child_tag
ON CONFLICT DO NOTHING;
