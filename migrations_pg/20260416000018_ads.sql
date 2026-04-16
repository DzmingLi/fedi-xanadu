-- Programmatic ad delivery system
CREATE TABLE ads (
    id          VARCHAR(64) PRIMARY KEY,
    title       VARCHAR(200) NOT NULL,
    body        VARCHAR(500),
    image_url   TEXT,
    link_url    TEXT NOT NULL,
    -- targeting
    placement   VARCHAR(30) NOT NULL DEFAULT 'sidebar',  -- sidebar, feed, banner …
    -- delivery control
    weight      INT NOT NULL DEFAULT 1 CHECK (weight >= 0),
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    starts_at   TIMESTAMPTZ,
    ends_at     TIMESTAMPTZ,
    -- budget
    daily_impression_cap INT,           -- NULL = unlimited
    total_impression_cap INT,           -- NULL = unlimited
    -- counters (denormalised for fast reads)
    impressions BIGINT NOT NULL DEFAULT 0,
    clicks      BIGINT NOT NULL DEFAULT 0,
    -- meta
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ads_active_placement ON ads (placement, is_active) WHERE is_active = true;

-- Daily impression log for cap enforcement
CREATE TABLE ad_daily_impressions (
    ad_id   VARCHAR(64) NOT NULL REFERENCES ads(id) ON DELETE CASCADE,
    day     DATE NOT NULL DEFAULT CURRENT_DATE,
    count   BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (ad_id, day)
);
