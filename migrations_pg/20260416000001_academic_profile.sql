-- Academic profile: publications, projects, teaching as JSONB arrays
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS publications JSONB NOT NULL DEFAULT '[]';
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS projects JSONB NOT NULL DEFAULT '[]';
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS teaching JSONB NOT NULL DEFAULT '[]';

-- Profile section layout preferences
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS profile_sections JSONB NOT NULL DEFAULT '["bio","publications","projects","teaching","education","articles","listings"]';
