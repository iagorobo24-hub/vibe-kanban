//! Design Mode REST routes, nested under a workspace:
//! `/workspaces/{id}/design-system`.

use std::path::PathBuf;

use axum::{
    Extension, Json,
    extract::State,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use db::models::workspace::Workspace;
use deployment::Deployment;
use serde::Deserialize;
use services::services::{
    container::ContainerService,
    design_system::{
        AuditDesignSystemResult, DesignSystemError, DesignSystemService, DesignSystemTokens,
        GenerateDesignSystemRequest,
    },
};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Clone, Deserialize, TS)]
pub struct SaveDesignSystemBody {
    pub content: String,
}

fn map_design_error(err: DesignSystemError) -> ApiError {
    match err {
        DesignSystemError::NotFound(msg) => ApiError::NotFound(msg),
        other => ApiError::BadRequest(other.to_string()),
    }
}

async fn worktree_path(
    deployment: &DeploymentImpl,
    workspace: &Workspace,
) -> Result<PathBuf, ApiError> {
    let container_ref = deployment
        .container()
        .ensure_container_exists(&workspace)
        .await?;
    Ok(PathBuf::from(container_ref))
}

pub async fn get_design_system(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Option<DesignSystemTokens>>>, ApiError> {
    let path = worktree_path(&deployment, &workspace).await?;
    let tokens = tokio::task::spawn_blocking(move || DesignSystemService::load_design_system(&path))
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .map_err(map_design_error)?;
    Ok(ResponseJson(ApiResponse::success(tokens)))
}

pub async fn save_design_system(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<SaveDesignSystemBody>,
) -> Result<ResponseJson<ApiResponse<DesignSystemTokens>>, ApiError> {
    let path = worktree_path(&deployment, &workspace).await?;
    let tokens =
        tokio::task::spawn_blocking(move || DesignSystemService::save_design_system(&path, &body.content))
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
            .map_err(map_design_error)?;
    Ok(ResponseJson(ApiResponse::success(tokens)))
}

pub async fn generate_design_system(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<GenerateDesignSystemRequest>,
) -> Result<ResponseJson<ApiResponse<DesignSystemTokens>>, ApiError> {
    let path = worktree_path(&deployment, &workspace).await?;
    let tokens =
        tokio::task::spawn_blocking(move || DesignSystemService::generate_design_system(&path, &req))
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
            .map_err(map_design_error)?;
    Ok(ResponseJson(ApiResponse::success(tokens)))
}

pub async fn audit_design_system(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<AuditDesignSystemResult>>, ApiError> {
    let path = worktree_path(&deployment, &workspace).await?;
    let result =
        tokio::task::spawn_blocking(move || DesignSystemService::audit_worktree(&path))
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
            .map_err(map_design_error)?;
    Ok(ResponseJson(ApiResponse::success(result)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(get_design_system).put(save_design_system))
        .route("/generate", post(generate_design_system))
        .route("/audit", post(audit_design_system))
}
