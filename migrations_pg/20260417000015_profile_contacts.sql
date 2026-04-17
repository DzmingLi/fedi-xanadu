-- Replace free-form `links` array with a fixed set of contact kinds.
-- Legacy `links` content is intentionally dropped (product decision).
ALTER TABLE profiles DROP COLUMN IF EXISTS links;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS contacts JSONB NOT NULL DEFAULT '{}';
