CREATE TABLE IF NOT EXISTS drafts (
    id TEXT PRIMARY KEY,
    did TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    content_format TEXT NOT NULL DEFAULT 'typst',
    lang TEXT NOT NULL DEFAULT 'zh',
    license TEXT NOT NULL DEFAULT 'CC-BY-NC-SA-4.0',
    tags TEXT NOT NULL DEFAULT '[]',
    prereqs TEXT NOT NULL DEFAULT '[]',
    at_uri TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_drafts_did ON drafts(did);
