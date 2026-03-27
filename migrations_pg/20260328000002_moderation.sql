ALTER TABLE platform_users
  ADD COLUMN is_banned  BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN banned_at  TIMESTAMPTZ,
  ADD COLUMN ban_reason TEXT;
