CREATE TABLE memory_entries (
    id TEXT PRIMARY KEY NOT NULL,
    namespace TEXT NOT NULL,
    origin TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT
);

CREATE INDEX idx_memory_entries_namespace_created
    ON memory_entries (namespace, created_at DESC);
