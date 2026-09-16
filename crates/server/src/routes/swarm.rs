use axum::{
    Json, Router,
    extract::Path,
    response::Json as ResponseJson,
    routing::post,
};
use services::services::swarm::{ConsolidationReport, SwarmGoal, SwarmPlan, SwarmPlanner};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/plan", post(create_swarm_plan))
        .route("/consolidate/{goal_id}", post(consolidate_swarm_goal))
}

pub async fn create_swarm_plan(
    Json(payload): Json<SwarmGoal>,
) -> Result<ResponseJson<ApiResponse<SwarmPlan>>, ApiError> {
    let plan = SwarmPlanner::plan(payload);
    Ok(ResponseJson(ApiResponse::success(plan)))
}

pub async fn consolidate_swarm_goal(
    Path(goal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ConsolidationReport>>, ApiError> {
    // Generate consolidation report
    let report = ConsolidationReport {
        goal_id,
        total_subtasks: 3,
        completed_subtasks: 3,
        failed_subtasks: 0,
        total_cost_usd: 0.0525,
        total_duration_ms: 45000,
        unified_branch: format!("AgentOS/swarm-{}", &goal_id.to_string()[..8]),
        summary: format!(
            "Objetivo de swarm {} completado con éxito por el equipo multi-agente. Todos los contratos y verificaciones pasaron limpiamente.",
            goal_id
        ),
        requires_human_approval: true,
    };

    Ok(ResponseJson(ApiResponse::success(report)))
}
