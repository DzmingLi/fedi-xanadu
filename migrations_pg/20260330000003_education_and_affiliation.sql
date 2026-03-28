-- Replace simple school field with structured education + affiliation

-- Education: array of {degree, school, year} entries
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS education JSONB NOT NULL DEFAULT '[]';
-- Current affiliation (workplace / research lab / org)
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS affiliation VARCHAR(255);
-- Whether the education/affiliation is admin-verified
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS credentials_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS credentials_verified_at TIMESTAMPTZ;

-- Mirror on platform_users for admin record-keeping
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS education JSONB NOT NULL DEFAULT '[]';
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS affiliation VARCHAR(255);
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS credentials_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE platform_users ADD COLUMN IF NOT EXISTS credentials_verified_at TIMESTAMPTZ;

-- Migrate existing school data into education array
UPDATE profiles
SET education = jsonb_build_array(jsonb_build_object('school', school, 'degree', '', 'year', ''))
WHERE school IS NOT NULL AND school != '' AND education = '[]';

UPDATE platform_users
SET education = jsonb_build_array(jsonb_build_object('school', school, 'degree', '', 'year', ''))
WHERE school IS NOT NULL AND school != '' AND education = '[]';

-- Copy verification status
UPDATE profiles SET credentials_verified = school_verified WHERE school_verified = true;
UPDATE platform_users SET credentials_verified = school_verified WHERE school_verified = true;

-- Admin question/answer publishing endpoint uses existing article create flow,
-- no schema changes needed for Q&A.
