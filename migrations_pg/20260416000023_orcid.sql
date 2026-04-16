-- ORCID binding for user profiles.
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS orcid VARCHAR(20);
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS orcid_verified_at TIMESTAMPTZ;
CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_orcid ON profiles(orcid) WHERE orcid IS NOT NULL;
