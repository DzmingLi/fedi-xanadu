-- Prevent duplicate author entities with the same display name.
-- Backfill runs merged duplicates manually via UPDATE/DELETE on prod
-- before this ran; migration only enforces the constraint going forward.
ALTER TABLE authors ADD CONSTRAINT authors_name_unique UNIQUE (name);
