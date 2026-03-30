-- Series-level pijul repo: each series has its own repo containing all chapters + shared resources
ALTER TABLE series ADD COLUMN IF NOT EXISTS pijul_node_id VARCHAR(255);
