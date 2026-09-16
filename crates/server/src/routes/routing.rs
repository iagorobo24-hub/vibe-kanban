use axum::{
    Json, Router,
    extract::Query,
    response::Json as ResponseJson,
    routing::{get, post},
};
use executors::catalog::{
    ModelProviderCatalog, RoutingRecommendation, RoutingRequest, WorkMode,
};
use serde::Deserialize;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct WorkModesQuery {
    pub role: Option<String>,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/catalog", get(get_catalog))
        .route("/work-modes", get(get_work_modes))
        .route("/recommend", post(recommend_route))
}

pub async fn get_catalog() -> Result<ResponseJson<ApiResponse<ModelProviderCatalog>>, ApiError> {
    let catalog = ModelProviderCatalog::default();
    Ok(ResponseJson(ApiResponse::success(catalog)))
}

pub async fn get_work_modes(
    Query(query): Query<WorkModesQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<WorkMode>>>, ApiError> {
    let catalog = ModelProviderCatalog::default();
    let modes = if let Some(role) = query.role {
        catalog
            .work_modes
            .into_iter()
            .filter(|m| {
                format!("{:?}", m.role)
                    .to_lowercase()
                    .contains(&role.to_lowercase())
            })
            .collect()
    } else {
        catalog.work_modes
    };
    Ok(ResponseJson(ApiResponse::success(modes)))
}

pub async fn recommend_route(
    Json(payload): Json<RoutingRequest>,
) -> Result<ResponseJson<ApiResponse<RoutingRecommendation>>, ApiError> {
    let catalog = ModelProviderCatalog::default();
    let recommendation = catalog
        .recommend(&payload)
        .map_err(|e| ApiError::BadRequest(e))?;

    Ok(ResponseJson(ApiResponse::success(recommendation)))
}
