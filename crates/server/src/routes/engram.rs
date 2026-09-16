use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use engram::{ContextGrant, MemoryEntry, MemoryStore, default_db_path};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateGrantPayload {
    pub recipient_id: String,
    pub session_id: String,
    pub project_id: String,
    pub scope: String,
    pub purpose: String,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: i64,
    #[serde(default = "default_granted_by")]
    pub granted_by: String,
}

fn default_ttl() -> i64 {
    86400 // 24 horas por defecto
}

fn default_granted_by() -> String {
    "agentos-control-plane".to_string()
}

#[derive(Debug, Deserialize, TS)]
pub struct ListGrantsParams {
    pub project_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ListEntriesParams {
    pub namespace: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateEntryPayload {
    pub namespace: String,
    pub origin: String,
    pub content: String,
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct RevokeGrantPayload {
    #[serde(default = "default_granted_by")]
    pub revoked_by: String,
}

async fn get_store() -> Result<MemoryStore, ApiError> {
    let path = default_db_path();
    let store = MemoryStore::open(&path).await?;
    Ok(store)
}

pub async fn create_grant(
    State(_deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateGrantPayload>,
) -> Result<ResponseJson<ApiResponse<ContextGrant>>, ApiError> {
    let store = get_store().await?;
    let grant = store
        .grant_context(
            &payload.recipient_id,
            &payload.session_id,
            &payload.project_id,
            &payload.scope,
            &payload.purpose,
            payload.ttl_seconds,
            &payload.granted_by,
        )
        .await?;

    Ok(ResponseJson(ApiResponse::success(grant)))
}

pub async fn list_grants(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<ListGrantsParams>,
) -> Result<ResponseJson<ApiResponse<Vec<ContextGrant>>>, ApiError> {
    let store = get_store().await?;
    let grants = store
        .list_active_grants(params.project_id.as_deref(), params.session_id.as_deref())
        .await?;

    Ok(ResponseJson(ApiResponse::success(grants)))
}

pub async fn revoke_grant(
    State(_deployment): State<DeploymentImpl>,
    Path(grant_id): Path<Uuid>,
    Json(payload): Json<Option<RevokeGrantPayload>>,
) -> Result<ResponseJson<ApiResponse<bool>>, ApiError> {
    let store = get_store().await?;
    let revoked_by = payload
        .map(|p| p.revoked_by)
        .unwrap_or_else(default_granted_by);

    let revoked = store.revoke_grant(grant_id, &revoked_by).await?;

    Ok(ResponseJson(ApiResponse::success(revoked)))
}

pub async fn list_entries(
    State(_deployment): State<DeploymentImpl>,
    Query(params): Query<ListEntriesParams>,
) -> Result<ResponseJson<ApiResponse<Vec<MemoryEntry>>>, ApiError> {
    let store = get_store().await?;
    let limit = params.limit.unwrap_or(20);
    let entries = store.read(&params.namespace, limit).await?;

    Ok(ResponseJson(ApiResponse::success(entries)))
}

pub async fn create_entry(
    State(_deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateEntryPayload>,
) -> Result<ResponseJson<ApiResponse<MemoryEntry>>, ApiError> {
    let store = get_store().await?;
    let entry = store
        .write(
            &payload.namespace,
            &payload.origin,
            &payload.content,
            payload.ttl_seconds,
        )
        .await?;

    Ok(ResponseJson(ApiResponse::success(entry)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/grants", post(create_grant).get(list_grants))
        .route("/grants/{id}/revoke", post(revoke_grant))
        .route("/entries", get(list_entries).post(create_entry))
}
