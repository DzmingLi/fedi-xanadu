CREATE TABLE IF NOT EXISTS platform_users (
    did VARCHAR(255) PRIMARY KEY,
    handle VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(255),
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
