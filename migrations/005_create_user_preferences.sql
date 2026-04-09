CREATE TABLE user_preferences (
    user_id         INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    default_mode    TEXT NOT NULL DEFAULT 'text',
    default_tags    TEXT NOT NULL DEFAULT '[]',
    auto_publish    INTEGER NOT NULL DEFAULT 0,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
