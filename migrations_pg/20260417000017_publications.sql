-- Publications: Medium-style ongoing content channels with multi-editor support.
-- Content (articles or series) can be cross-posted into a publication by its
-- author. Membership requires a bilateral handshake: owner lists the member,
-- and the member confirms (membership_at_uri becomes non-null).

CREATE TABLE IF NOT EXISTS publications (
    id VARCHAR(64) PRIMARY KEY,                  -- slug, URL-safe
    title_i18n JSONB NOT NULL DEFAULT '{}',      -- {"en": "AI Weekly", "zh": "AI 周刊"}
    description_i18n JSONB NOT NULL DEFAULT '{}',
    cover_url TEXT,
    created_by VARCHAR(255) NOT NULL,            -- owner DID
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    at_uri VARCHAR(512) UNIQUE                   -- at://owner/li.dzming.fedi-xanadu.publication/{slug}
);

CREATE INDEX IF NOT EXISTS idx_publications_created_by ON publications(created_by);

CREATE TABLE IF NOT EXISTS publication_members (
    publication_id VARCHAR(64) NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL CHECK (role IN ('owner', 'editor', 'writer')),
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by VARCHAR(255) NOT NULL,              -- DID of whoever (owner/editor) added them
    membership_at_uri VARCHAR(512),              -- set once the member creates their membership record
    PRIMARY KEY (publication_id, did)
);

CREATE INDEX IF NOT EXISTS idx_publication_members_did ON publication_members(did);

CREATE TABLE IF NOT EXISTS publication_content (
    publication_id VARCHAR(64) NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    content_uri VARCHAR(512) NOT NULL,           -- article or series at_uri
    content_kind VARCHAR(20) NOT NULL CHECK (content_kind IN ('article', 'series')),
    added_by VARCHAR(255) NOT NULL,              -- DID of the content author (who cross-posted)
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    entry_at_uri VARCHAR(512),                   -- at://author/li.dzming.fedi-xanadu.publication.entry/{rkey}
    PRIMARY KEY (publication_id, content_uri)
);

CREATE INDEX IF NOT EXISTS idx_publication_content_pub_added ON publication_content(publication_id, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_publication_content_uri ON publication_content(content_uri);

CREATE TABLE IF NOT EXISTS publication_followers (
    publication_id VARCHAR(64) NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    did VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    follow_at_uri VARCHAR(512),                  -- at://follower/li.dzming.fedi-xanadu.publication.follow/{rkey}
    PRIMARY KEY (publication_id, did)
);

CREATE INDEX IF NOT EXISTS idx_publication_followers_did ON publication_followers(did);
