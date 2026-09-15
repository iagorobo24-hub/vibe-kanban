use axum::{
    Json, Router,
    extract::{Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    execution_process::ExecutionProcess,
    execution_telemetry::{ExecutionTelemetry, RecordExecutionTelemetry, TelemetrySummaryRow},
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize, TS)]
pub struct TelemetryListParams {
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub real_outcome: Option<String>,
}

/// Record the real (manually-judged) outcome of an execution process.
/// Fase 4 — docs/agentos/04-ROADMAP.md: "registrar resultado real, no solo
/// exit code". `session_id`/`workspace_id` are derived server-side from the
/// execution process so a caller cannot attach telemetry to processes it
/// does not own the context of.
pub async fn record_telemetry(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<RecordExecutionTelemetry>,
) -> Result<ResponseJson<ApiResponse<ExecutionTelemetry>>, ApiError> {
    let pool = &deployment.db().pool;

    let execution_process = ExecutionProcess::find_by_id(pool, payload.execution_process_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Execution process not found".to_string()))?;

    let (workspace, session) = execution_process
        .parent_workspace_and_session(pool)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest("Execution process has no parent session/workspace".to_string())
        })?;

    let recorded = ExecutionTelemetry::record(pool, session.id, workspace.id, &payload).await?;

    Ok(ResponseJson(ApiResponse::success(recorded)))
}

pub async fn list_telemetry(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<TelemetryListParams>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionTelemetry>>>, ApiError> {
    let rows = ExecutionTelemetry::list(
        &deployment.db().pool,
        params.executor.as_deref(),
        params.task_type.as_deref(),
        params.real_outcome.as_deref(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn telemetry_summary(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TelemetrySummaryRow>>>, ApiError> {
    let rows = ExecutionTelemetry::summary(&deployment.db().pool).await?;

    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let inner = Router::new()
        .route("/", get(list_telemetry).post(record_telemetry))
        .route("/summary", get(telemetry_summary));

    Router::new().nest("/telemetry", inner)
}
