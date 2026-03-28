ALTER TABLE series ADD COLUMN lang VARCHAR(10) NOT NULL DEFAULT 'zh';
ALTER TABLE series ADD COLUMN translation_group VARCHAR(255);
CREATE INDEX idx_series_translation_group ON series(translation_group);
