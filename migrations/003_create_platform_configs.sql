CREATE TABLE platform_configs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform    TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    image_count INTEGER NOT NULL DEFAULT 3,
    cookies     TEXT,
    extra       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, platform)
);
