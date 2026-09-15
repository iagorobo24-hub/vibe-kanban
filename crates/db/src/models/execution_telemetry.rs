use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

/// Manually-recorded outcome for one execution process.
///
/// Deliberately separate from `ExecutionProcess::status`/`exit_code`: a
/// process can finish `completed`/`exit_code=0` without real success (see
/// the Antigravity false positive documented in
/// docs/agentos/evidence/2026-09-15-g2-antigravity-file-not-found.md), and
/// the roadmap (Fase 4, docs/agentos/04-ROADMAP.md) explicitly requires
/// "resultado real, no solo exit code".
pub const VALID_REAL_OUTCOMES: [&str; 4] = ["success", "failure", "partial", "unknown"];

#[derive(Debug, Error)]
pub enum ExecutionTelemetryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Invalid real_outcome '{0}', expected one of success|failure|partial|unknown")]
    InvalidOutcome(String),
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionTelemetry {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub session_id: Uuid,
    pub workspace_id: Uuid,
    pub executor: String,
    pub model_id: Option<String>,
    pub task_type: String,
    pub real_outcome: String,
    pub outcome_note: Option<String>,
    pub cost_usd: Option<f64>,
    pub recorded_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct RecordExecutionTelemetry {
    pub execution_process_id: Uuid,
    pub executor: String,
    pub model_id: Option<String>,
    pub task_type: String,
    pub real_outcome: String,
    pub outcome_note: Option<String>,
    pub cost_usd: Option<f64>,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TelemetrySummaryRow {
    pub executor: String,
    pub model_id: Option<String>,
    pub task_type: String,
    pub real_outcome: String,
    pub count: i64,
}

impl ExecutionTelemetry {
    /// Record a manual outcome for an execution process. `session_id` and
    /// `workspace_id` are resolved by the caller from the execution process
    /// context, not trusted from the request body.
    pub async fn record(
        pool: &SqlitePool,
        session_id: Uuid,
        workspace_id: Uuid,
        data: &RecordExecutionTelemetry,
    ) -> Result<Self, ExecutionTelemetryError> {
        if !VALID_REAL_OUTCOMES.contains(&data.real_outcome.as_str()) {
            return Err(ExecutionTelemetryError::InvalidOutcome(
                data.real_outcome.clone(),
            ));
        }

        let id = Uuid::new_v4();
        let recorded_by = data
            .recorded_by
            .clone()
            .unwrap_or_else(|| "manual".to_string());

        let row = sqlx::query_as::<_, Self>(
            r#"INSERT INTO execution_telemetry
                (id, execution_process_id, session_id, workspace_id, executor,
                 model_id, task_type, real_outcome, outcome_note, cost_usd, recorded_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, execution_process_id, session_id, workspace_id, executor,
                         model_id, task_type, real_outcome, outcome_note, cost_usd,
                         recorded_by, created_at"#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(session_id)
        .bind(workspace_id)
        .bind(&data.executor)
        .bind(&data.model_id)
        .bind(&data.task_type)
        .bind(&data.real_outcome)
        .bind(&data.outcome_note)
        .bind(data.cost_usd)
        .bind(&recorded_by)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// List telemetry rows, most recent first, optionally filtered.
    pub async fn list(
        pool: &SqlitePool,
        executor: Option<&str>,
        task_type: Option<&str>,
        real_outcome: Option<&str>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT id, execution_process_id, session_id, workspace_id, executor,
                      model_id, task_type, real_outcome, outcome_note, cost_usd,
                      recorded_by, created_at
               FROM execution_telemetry
               WHERE (?1 IS NULL OR executor = ?1)
                 AND (?2 IS NULL OR task_type = ?2)
                 AND (?3 IS NULL OR real_outcome = ?3)
               ORDER BY created_at DESC"#,
        )
        .bind(executor)
        .bind(task_type)
        .bind(real_outcome)
        .fetch_all(pool)
        .await
    }

    /// Aggregate counts by executor/model/task_type/outcome — the
    /// "histórico consultable que permite comparar resultados" required by
    /// the Fase 4 exit criterion.
    pub async fn summary(pool: &SqlitePool) -> Result<Vec<TelemetrySummaryRow>, sqlx::Error> {
        sqlx::query_as::<_, TelemetrySummaryRow>(
            r#"SELECT executor, model_id, task_type, real_outcome, COUNT(*) as "count"
               FROM execution_telemetry
               GROUP BY executor, model_id, task_type, real_outcome
               ORDER BY executor, task_type, real_outcome"#,
        )
        .fetch_all(pool)
        .await
    }
}
