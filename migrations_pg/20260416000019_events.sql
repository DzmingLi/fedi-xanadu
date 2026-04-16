-- Events / Meetups
CREATE TABLE IF NOT EXISTS events (
    id              VARCHAR(64) PRIMARY KEY,
    did             VARCHAR(255) NOT NULL,
    title           VARCHAR(500) NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    kind            VARCHAR(30) NOT NULL CHECK (kind IN ('conference','workshop','seminar','meetup','hackathon')),
    location        VARCHAR(500),
    online_url      VARCHAR(1024),
    start_time      TIMESTAMPTZ NOT NULL,
    end_time        TIMESTAMPTZ,
    organizer       VARCHAR(500) NOT NULL,
    contact_email   VARCHAR(255),
    contact_url     VARCHAR(1024),
    max_attendees   INT,
    is_cancelled    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_did ON events(did);
CREATE INDEX IF NOT EXISTS idx_events_start_time ON events(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);

CREATE TABLE IF NOT EXISTS event_teaches (
    event_id VARCHAR(64) NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id   VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (event_id, tag_id)
);

CREATE TABLE IF NOT EXISTS event_prereqs (
    event_id VARCHAR(64) NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id   VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (event_id, tag_id)
);

CREATE TABLE IF NOT EXISTS event_rsvps (
    event_id   VARCHAR(64) NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    did        VARCHAR(255) NOT NULL,
    status     VARCHAR(20) NOT NULL DEFAULT 'going' CHECK (status IN ('going','interested')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, did)
);

CREATE INDEX IF NOT EXISTS idx_event_rsvps_did ON event_rsvps(did);
