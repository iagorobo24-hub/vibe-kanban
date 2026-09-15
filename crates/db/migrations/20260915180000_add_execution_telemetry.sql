-- Fase 4 (métricas antes de routing, docs/agentos/04-ROADMAP.md).
-- Histórico manual de resultado real por ejecución: distinto de exit_code,
-- porque un proceso puede terminar completed/0 sin haber tenido éxito real
-- (ver evidencia G2 2026-09-15: falso positivo de Antigravity).
PRAGMA foreign_keys = ON;

CREATE TABLE execution_telemetry (
    id                    BLOB PRIMARY KEY,
    execution_process_id  BLOB NOT NULL,
    session_id            BLOB NOT NULL,
    workspace_id          BLOB NOT NULL,
    executor              TEXT NOT NULL,
    model_id              TEXT,
    task_type             TEXT NOT NULL,
    real_outcome          TEXT NOT NULL
                            CHECK (real_outcome IN ('success','failure','partial','unknown')),
    outcome_note          TEXT,
    cost_usd              REAL,
    recorded_by           TEXT NOT NULL DEFAULT 'manual',
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (execution_process_id) REFERENCES execution_processes(id) ON DELETE CASCADE
);

CREATE INDEX idx_execution_telemetry_execution_process_id ON execution_telemetry(execution_process_id);
CREATE INDEX idx_execution_telemetry_executor ON execution_telemetry(executor);
CREATE INDEX idx_execution_telemetry_task_type ON execution_telemetry(task_type);
CREATE INDEX idx_execution_telemetry_workspace_id ON execution_telemetry(workspace_id);
