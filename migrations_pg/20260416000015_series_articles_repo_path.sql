-- Add repo_path column to series_articles for git-like repo structure
ALTER TABLE series_articles ADD COLUMN IF NOT EXISTS repo_path TEXT;

