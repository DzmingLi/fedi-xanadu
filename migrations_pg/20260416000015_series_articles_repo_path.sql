-- Add repo_path to series_articles so batch-publish can upsert by path
ALTER TABLE series_articles ADD COLUMN repo_path TEXT;

-- Allow looking up an article by its path within a series
CREATE UNIQUE INDEX idx_series_articles_repo_path
    ON series_articles(series_id, repo_path) WHERE repo_path IS NOT NULL;
