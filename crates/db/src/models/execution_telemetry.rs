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
    pub execution_process_id: Option<Uuid>,
    pub session_id: Uuid,
    pub workspace_id: Uuid,
    pub executor: String,
    pub model_id: Option<String>,
    pub task_type: String,
    pub real_outcome: String,
    pub outcome_note: Option<String>,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
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
    pub duration_ms: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TelemetrySummaryRow {
    pub executor: String,
    pub model_id: Option<String>,
    pub task_type: String,
    pub real_outcome: String,
    pub count: i64,
    pub avg_duration_ms: Option<f64>,
    pub total_cost_usd: Option<f64>,
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
                 model_id, task_type, real_outcome, outcome_note, cost_usd,
                 duration_ms, input_tokens, output_tokens, total_tokens, recorded_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, execution_process_id, session_id, workspace_id, executor,
                         model_id, task_type, real_outcome, outcome_note, cost_usd,
                         duration_ms, input_tokens, output_tokens, total_tokens,
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
        .bind(data.duration_ms)
        .bind(data.input_tokens)
        .bind(data.output_tokens)
        .bind(data.total_tokens)
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
                      duration_ms, input_tokens, output_tokens, total_tokens,
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
            r#"SELECT executor, model_id, task_type, real_outcome, COUNT(*) as "count",
                      AVG(duration_ms) as "avg_duration_ms",
                      SUM(cost_usd) as "total_cost_usd"
               FROM execution_telemetry
               GROUP BY executor, model_id, task_type, real_outcome
               ORDER BY executor, task_type, real_outcome"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Find the latest telemetry record for an execution process, if one exists.
    pub async fn find_by_execution_process_id(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT id, execution_process_id, session_id, workspace_id, executor,
                      model_id, task_type, real_outcome, outcome_note, cost_usd,
                      duration_ms, input_tokens, output_tokens, total_tokens,
                      recorded_by, created_at
               FROM execution_telemetry
               WHERE execution_process_id = ?
               ORDER BY created_at DESC
               LIMIT 1"#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    async fn create_test_db() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("telemetry-test.db");
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.to_string_lossy()))
            .expect("options")
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .expect("pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        (pool, dir)
    }

    #[tokio::test]
    async fn records_telemetry_with_metrics_and_survives_process_deletion() {
        let (pool, _dir) = create_test_db().await;

        let session_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let process_id = Uuid::new_v4();

        // 1. Create a dummy workspace and session first to satisfy foreign keys
        sqlx::query(
            "INSERT INTO workspaces (id, branch, created_at, updated_at) VALUES (?, 'main', datetime('now'), datetime('now'))"
        )
        .bind(workspace_id)
        .execute(&pool)
        .await
        .expect("insert workspace");

        sqlx::query(
            "INSERT INTO sessions (id, workspace_id, created_at, updated_at) VALUES (?, ?, datetime('now'), datetime('now'))"
        )
        .bind(session_id)
        .bind(workspace_id)
        .execute(&pool)
        .await
        .expect("insert session");

        sqlx::query(
            "INSERT INTO execution_processes (id, session_id, run_reason, status, started_at, created_at, updated_at) \
             VALUES (?, ?, 'codingagent', 'completed', datetime('now'), datetime('now'), datetime('now'))"
        )
        .bind(process_id)
        .bind(session_id)
        .execute(&pool)
        .await
        .expect("insert execution process");

        // 2. Record execution telemetry with duration and tokens
        let data = RecordExecutionTelemetry {
            execution_process_id: process_id,
            executor: "CLAUDE_CODE".to_string(),
            model_id: Some("claude-sonnet-4-6".to_string()),
            task_type: "refactor".to_string(),
            real_outcome: "success".to_string(),
            outcome_note: Some("Clean refactor verified".to_string()),
            cost_usd: Some(0.045),
            duration_ms: Some(12500),
            input_tokens: Some(3500),
            output_tokens: Some(850),
            total_tokens: Some(4350),
            recorded_by: Some("test".to_string()),
        };

        let recorded = ExecutionTelemetry::record(&pool, session_id, workspace_id, &data)
            .await
            .expect("record telemetry");

        assert_eq!(recorded.execution_process_id, Some(process_id));
        assert_eq!(recorded.duration_ms, Some(12500));
        assert_eq!(recorded.input_tokens, Some(3500));
        assert_eq!(recorded.output_tokens, Some(850));
        assert_eq!(recorded.total_tokens, Some(4350));
        assert_eq!(recorded.cost_usd, Some(0.045));

        // 3. Test summary query and find_by_execution_process_id
        let found = ExecutionTelemetry::find_by_execution_process_id(&pool, process_id)
            .await
            .expect("find by exec id");
        assert!(found.is_some());
        assert_eq!(found.unwrap().cost_usd, Some(0.045));

        let summaries = ExecutionTelemetry::summary(&pool).await.expect("summary");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].count, 1);
        assert_eq!(summaries[0].avg_duration_ms, Some(12500.0));
        assert_eq!(summaries[0].total_cost_usd, Some(0.045));

        // 4. CRITICAL TEST: Delete the execution process (simulating workspace/session cleanup)
        // With ON DELETE SET NULL, the telemetry row must NOT be deleted, and execution_process_id becomes NULL!
        sqlx::query("DELETE FROM execution_processes WHERE id = ?")
            .bind(process_id)
            .execute(&pool)
            .await
            .expect("delete execution process");

        let rows = ExecutionTelemetry::list(&pool, None, None, None)
            .await
            .expect("list telemetry");
        assert_eq!(rows.len(), 1, "Telemetry record must survive execution_process deletion!");
        assert_eq!(rows[0].execution_process_id, None, "execution_process_id should be SET NULL");
        assert_eq!(rows[0].session_id, session_id, "session_id must be preserved");
        assert_eq!(rows[0].workspace_id, workspace_id, "workspace_id must be preserved");
        assert_eq!(rows[0].duration_ms, Some(12500), "duration_ms must be preserved");
        assert_eq!(rows[0].total_tokens, Some(4350), "total_tokens must be preserved");
    }
}
