ALTER TABLE tags ADD COLUMN names JSONB NOT NULL DEFAULT '{}';
UPDATE tags SET names = jsonb_build_object('zh', name) WHERE name != id;
