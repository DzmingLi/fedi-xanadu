-- Bookmark visibility: users can choose which bookmark folders are public
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS bookmarks_public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS public_folders JSONB NOT NULL DEFAULT '[]';

-- School verification
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS school VARCHAR(255);
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS school_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS school_verified_at TIMESTAMPTZ;

-- Profiles need school fields for display
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS school VARCHAR(255);
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS school_verified BOOLEAN NOT NULL DEFAULT FALSE;
