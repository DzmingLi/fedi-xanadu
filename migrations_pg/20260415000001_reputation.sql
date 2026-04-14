-- User reputation system
-- Reputation is materialized on the profiles table and updated on vote events.

ALTER TABLE profiles ADD COLUMN reputation INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_profiles_reputation ON profiles(reputation DESC);
