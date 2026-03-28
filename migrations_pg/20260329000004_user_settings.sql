CREATE TABLE IF NOT EXISTS user_settings (
    did VARCHAR(255) PRIMARY KEY,
    native_lang VARCHAR(10) NOT NULL DEFAULT 'zh',
    known_langs JSONB NOT NULL DEFAULT '["zh"]',
    prefer_native BOOLEAN NOT NULL DEFAULT true,
    hide_unknown BOOLEAN NOT NULL DEFAULT false,
    default_format VARCHAR(20) NOT NULL DEFAULT 'typst',
    email VARCHAR(255),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
