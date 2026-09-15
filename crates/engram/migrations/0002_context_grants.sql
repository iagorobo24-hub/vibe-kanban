CREATE TABLE context_grants (
    id TEXT PRIMARY KEY NOT NULL,
    recipient_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    purpose TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    granted_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE INDEX idx_context_grants_recipient_project
    ON context_grants (recipient_id, project_id, expires_at);
