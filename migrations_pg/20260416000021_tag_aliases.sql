-- Tag aliases: multiple names can resolve to the same tag.
-- E.g. "TCS", "theoretical-cs" both resolve to tag "tcs".
CREATE TABLE IF NOT EXISTS tag_aliases (
    alias   VARCHAR(255) PRIMARY KEY,
    tag_id  VARCHAR(255) NOT NULL REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tag_aliases_tag ON tag_aliases(tag_id);
