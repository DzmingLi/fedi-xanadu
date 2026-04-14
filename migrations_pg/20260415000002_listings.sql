-- Academic listings (recruitment, student positions, internships)

CREATE TABLE listings (
    id VARCHAR(64) PRIMARY KEY,
    did VARCHAR(255) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    -- Position type
    kind VARCHAR(30) NOT NULL CHECK (kind IN ('phd', 'masters', 'ra', 'postdoc', 'intern', 'faculty', 'other')),
    -- Institution info
    institution VARCHAR(500) NOT NULL,
    department VARCHAR(500),
    location VARCHAR(500),
    -- Contact
    contact_email VARCHAR(255),
    contact_url VARCHAR(1024),
    -- Compensation / funding
    compensation TEXT,
    -- Deadline (nullable = rolling)
    deadline DATE,
    -- Status
    is_open BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_listings_did ON listings(did);
CREATE INDEX idx_listings_kind ON listings(kind);
CREATE INDEX idx_listings_open ON listings(is_open, created_at DESC);

-- Required skills for a listing (must have)
CREATE TABLE listing_required_tags (
    listing_id VARCHAR(64) NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (listing_id, tag_id)
);

CREATE INDEX idx_listing_required_tags_tag ON listing_required_tags(tag_id);

-- Preferred skills (nice to have)
CREATE TABLE listing_preferred_tags (
    listing_id VARCHAR(64) NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (listing_id, tag_id)
);

CREATE INDEX idx_listing_preferred_tags_tag ON listing_preferred_tags(tag_id);
