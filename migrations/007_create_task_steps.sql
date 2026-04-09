CREATE TABLE task_steps (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    step_name   TEXT NOT NULL,
    step_order  INTEGER NOT NULL,
    status      TEXT NOT NULL DEFAULT 'Pending',
    result      TEXT,
    error       TEXT,
    started_at  TEXT,
    finished_at TEXT
);
