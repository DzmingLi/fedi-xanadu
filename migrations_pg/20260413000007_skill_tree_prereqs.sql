-- Skill tree prerequisite edges (DAG, separate from hierarchy edges)
CREATE TABLE skill_tree_prereqs (
    tree_uri VARCHAR(512) NOT NULL REFERENCES skill_trees(at_uri) ON DELETE CASCADE,
    from_tag VARCHAR(255) NOT NULL,
    to_tag   VARCHAR(255) NOT NULL,
    prereq_type VARCHAR(50) NOT NULL DEFAULT 'required',
    PRIMARY KEY (tree_uri, from_tag, to_tag)
);

CREATE INDEX idx_skill_tree_prereqs_tree ON skill_tree_prereqs(tree_uri);

-- User's personal prereq edges (synced on adopt)
CREATE TABLE user_tag_prereqs (
    did      VARCHAR(255) NOT NULL,
    from_tag VARCHAR(255) NOT NULL,
    to_tag   VARCHAR(255) NOT NULL,
    prereq_type VARCHAR(50) NOT NULL DEFAULT 'required',
    PRIMARY KEY (did, from_tag, to_tag)
);

CREATE INDEX idx_user_tag_prereqs_did ON user_tag_prereqs(did);
