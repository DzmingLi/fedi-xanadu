-- Add hierarchical series support: parent_id for nesting, order_index for ordering

-- Allow series to be nested under a parent series
ALTER TABLE series ADD COLUMN parent_id VARCHAR(255) REFERENCES series(id) ON DELETE CASCADE;
ALTER TABLE series ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_series_parent ON series(parent_id);

-- Add ordering to series_articles so articles within a section have stable order
ALTER TABLE series_articles ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;
