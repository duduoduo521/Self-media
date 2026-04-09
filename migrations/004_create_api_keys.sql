CREATE TABLE api_keys (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider      TEXT NOT NULL,
    encrypted_key TEXT NOT NULL,
    region        TEXT NOT NULL DEFAULT 'cn',
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, provider)
);
