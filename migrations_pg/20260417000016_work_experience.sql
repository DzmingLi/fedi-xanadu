-- Work experience entries on user profiles, structured like education.
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS experience JSONB NOT NULL DEFAULT '[]';
