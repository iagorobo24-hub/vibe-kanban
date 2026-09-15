-- Migration: preserve telemetry when execution_processes/workspaces are deleted (ON DELETE SET NULL)
-- and add duration_ms and token counts (input_tokens, output_tokens, total_tokens).

PRAGMA foreign_keys = OFF;

CREATE TABLE new_execution_telemetry (
    id                    BLOB PRIMARY KEY,
    execution_process_id  BLOB,
    session_id            BLOB NOT NULL,
    workspace_id          BLOB NOT NULL,
    executor              TEXT NOT NULL,
    model_id              TEXT,
    task_type             TEXT NOT NULL,
    real_outcome          TEXT NOT NULL
                            CHECK (real_outcome IN ('success','failure','partial','unknown')),
    outcome_note          TEXT,
    cost_usd              REAL,
    duration_ms           INTEGER,
    input_tokens          INTEGER,
    output_tokens         INTEGER,
    total_tokens          INTEGER,
    recorded_by           TEXT NOT NULL DEFAULT 'manual',
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (execution_process_id) REFERENCES execution_processes(id) ON DELETE SET NULL
);

INSERT INTO new_execution_telemetry (
    id, execution_process_id, session_id, workspace_id, executor, model_id,
    task_type, real_outcome, outcome_note, cost_usd, duration_ms,
    input_tokens, output_tokens, total_tokens, recorded_by, created_at
)
SELECT
    id, execution_process_id, session_id, workspace_id, executor, model_id,
    task_type, real_outcome, outcome_note, cost_usd, NULL,
    NULL, NULL, NULL, recorded_by, created_at
FROM execution_telemetry;

DROP TABLE execution_telemetry;

ALTER TABLE new_execution_telemetry RENAME TO execution_telemetry;

CREATE INDEX idx_execution_telemetry_execution_process_id ON execution_telemetry(execution_process_id);
CREATE INDEX idx_execution_telemetry_executor ON execution_telemetry(executor);
CREATE INDEX idx_execution_telemetry_task_type ON execution_telemetry(task_type);
CREATE INDEX idx_execution_telemetry_workspace_id ON execution_telemetry(workspace_id);
CREATE INDEX idx_execution_telemetry_real_outcome ON execution_telemetry(real_outcome);

PRAGMA foreign_keys = ON;
