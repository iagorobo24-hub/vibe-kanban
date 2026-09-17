use axum::{
    Json, Router,
    extract::{Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use executors::catalog::{
    ModelProviderCatalog, RoutingRecommendation, RoutingRequest, WorkMode, WorkModeRole,
};
use serde::Deserialize;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct WorkModesQuery {
    /// Case-insensitive filter over `WorkModeRole` (`PLANNING`, `EXECUTION`,
    /// `HYBRID`). Invalid values are rejected with a 400 instead of silently
    /// returning an empty list.
    pub role: Option<String>,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/catalog", get(get_catalog))
        .route("/work-modes", get(get_work_modes))
        .route("/recommend", post(recommend_route))
}

/// R1: the catalog is constructed once at startup and shared through the
/// deployment state. It is no longer rebuilt inside every handler. The clone
/// below only copies the payload that goes on the wire.
pub async fn get_catalog(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ModelProviderCatalog>>, ApiError> {
    let catalog = (*deployment.model_catalog()).clone();
    Ok(ResponseJson(ApiResponse::success(catalog)))
}

/// Pure filter behind `GET /api/routing/work-modes`.
///
/// Extracted from the handler so that the D4-critical logic is reachable from a
/// unit test without booting a deployment. The bug this guards against is
/// comparing the query value against Rust's `Debug` output (`Planning`) instead
/// of the serde wire format (`PLANNING`): that comparison silently never
/// matches and the endpoint returns an empty list with HTTP 200.
pub fn filter_work_modes(
    catalog: &ModelProviderCatalog,
    role: Option<&str>,
) -> Result<Vec<WorkMode>, ApiError> {
    match role {
        Some(raw) => {
            // D4: match on the serde representation (PLANNING), not on Rust's
            // `Debug` output (Planning). Going through `WorkModeRole::from_query`
            // keeps serde as the single source of truth.
            let wanted = WorkModeRole::from_query(raw).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "Rol de modo de trabajo inválido: '{}'. Valores válidos: PLANNING, EXECUTION, HYBRID.",
                    raw
                ))
            })?;

            Ok(catalog
                .work_modes
                .iter()
                .filter(|mode| mode.role == wanted)
                .cloned()
                .collect())
        }
        None => Ok(catalog.work_modes.clone()),
    }
}

pub async fn get_work_modes(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<WorkModesQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<WorkMode>>>, ApiError> {
    let modes = filter_work_modes(&deployment.model_catalog(), query.role.as_deref())?;
    Ok(ResponseJson(ApiResponse::success(modes)))
}

pub async fn recommend_route(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<RoutingRequest>,
) -> Result<ResponseJson<ApiResponse<RoutingRecommendation>>, ApiError> {
    let recommendation = deployment
        .model_catalog()
        .recommend(&payload)
        .map_err(ApiError::BadRequest)?;

    Ok(ResponseJson(ApiResponse::success(recommendation)))
}

#[cfg(test)]
mod tests {
    use axum::http::Uri;

    use super::*;

    fn catalog() -> ModelProviderCatalog {
        ModelProviderCatalog::default()
    }

    /// Regression guard for review D4 at the handler level.
    ///
    /// The defect compared the query value against Rust's `Debug` output
    /// (`Planning`) instead of the serde wire format (`PLANNING`). The
    /// comparison never matched, so `?role=PLANNING` returned HTTP 200 with an
    /// empty list — a silent wrong answer, which is worse than an error.
    #[test]
    fn role_filter_matches_the_wire_format_not_the_debug_format() {
        let catalog = catalog();

        let planning = filter_work_modes(&catalog, Some("PLANNING"))
            .expect("PLANNING es un rol válido y no debe ser un error");

        assert!(
            !planning.is_empty(),
            "el filtro 'PLANNING' no debe devolver una lista vacía"
        );
        assert!(
            planning.iter().all(|mode| mode.role == WorkModeRole::Planning),
            "el filtro devolvió modos de otro rol"
        );
        assert!(
            planning.iter().any(|mode| mode.id == "planning"),
            "el modo 'planning' debe estar en el resultado"
        );
    }

    /// The same guard, generalised: every role in the catalog must be findable
    /// through its own canonical wire form. This catches the `Debug` regression
    /// for all roles, not just `PLANNING`.
    #[test]
    fn every_mode_role_round_trips_through_the_filter() {
        let catalog = catalog();

        for mode in &catalog.work_modes {
            let wire = mode.role.as_wire();
            let matched = filter_work_modes(&catalog, Some(&wire))
                .unwrap_or_else(|err| panic!("el rol '{wire}' debería ser válido: {err}"));

            assert!(
                !matched.is_empty(),
                "el rol '{wire}' devolvió una lista vacía; ¿se está comparando contra el Debug de Rust?"
            );
            assert!(
                matched.iter().any(|m| m.id == mode.id),
                "el modo '{}' (rol {wire}) no aparece al filtrar por su propio rol",
                mode.id
            );
            assert!(
                matched.iter().all(|m| m.role == mode.role),
                "el filtro '{wire}' devolvió modos de otro rol"
            );
        }
    }

    #[test]
    fn role_filter_is_case_insensitive() {
        let catalog = catalog();

        for raw in ["PLANNING", "planning", "Planning", "  planning  "] {
            let matched = filter_work_modes(&catalog, Some(raw))
                .unwrap_or_else(|err| panic!("'{raw}' debería interpretarse: {err}"));
            assert!(
                matched.iter().any(|mode| mode.id == "planning"),
                "'{raw}' no encontró el modo 'planning'"
            );
        }
    }

    #[test]
    fn no_role_returns_every_mode_unfiltered() {
        let catalog = catalog();
        let all = filter_work_modes(&catalog, None).expect("sin rol no debe haber error");

        assert_eq!(all.len(), catalog.work_modes.len());
        assert_eq!(all.len(), 4, "el catálogo por defecto tiene 4 modos");
    }

    #[test]
    fn an_invalid_role_is_a_bad_request_naming_the_value_and_the_valid_ones() {
        let catalog = catalog();

        let err = filter_work_modes(&catalog, Some("BOGUS"))
            .expect_err("un rol inválido debe ser un error, no una lista vacía");

        match err {
            ApiError::BadRequest(msg) => {
                assert!(msg.contains("BOGUS"), "el mensaje debe nombrar el valor recibido: {msg}");
                assert!(msg.contains("PLANNING"), "el mensaje debe listar los valores válidos: {msg}");
                assert!(msg.contains("EXECUTION") && msg.contains("HYBRID"), "faltan valores válidos en el mensaje: {msg}");
            }
            other => panic!("se esperaba BadRequest, se obtuvo: {other:?}"),
        }
    }

    /// Exercises the real Axum extraction path, so that a change to how the
    /// query is parsed is caught here rather than only by the live HTTP test.
    #[test]
    fn query_extraction_yields_the_raw_role_string() {
        let uri: Uri = "/work-modes?role=PLANNING".parse().unwrap();
        let Query(query) = Query::<WorkModesQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(query.role.as_deref(), Some("PLANNING"));

        let uri: Uri = "/work-modes?role=planning".parse().unwrap();
        let Query(query) = Query::<WorkModesQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(query.role.as_deref(), Some("planning"));

        let uri: Uri = "/work-modes".parse().unwrap();
        let Query(query) = Query::<WorkModesQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(query.role, None);

        // An explicitly empty role is present but empty, not absent: it must be
        // rejected as invalid rather than silently treated as "no filter".
        let uri: Uri = "/work-modes?role=".parse().unwrap();
        let Query(query) = Query::<WorkModesQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(query.role.as_deref(), Some(""));
        assert!(
            filter_work_modes(&catalog(), query.role.as_deref()).is_err(),
            "un rol vacío debe ser un 400, no una lista completa"
        );
    }

    #[test]
    fn recommended_route_is_the_one_the_catalog_chooses() {
        let catalog = catalog();

        let rec = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some("balanced".to_string()),
                task_type: None,
                max_budget_usd: None,
                required_capabilities: None,
            })
            .expect("el modo balanced debe resolver");

        assert_eq!(rec.selected_model_id, rec.selected_route.real_model_id);
        assert_eq!(rec.selected_executor, rec.selected_route.executor);
    }
}
