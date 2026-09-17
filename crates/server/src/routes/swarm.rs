use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::post,
};
use chrono::Utc;
use services::services::swarm::{
    ConsolidationReport, SwarmGoal, SwarmPlan, SwarmPlanner, SwarmSubTaskStatus,
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/plan", post(create_swarm_plan))
        .route("/consolidate/{goal_id}", post(consolidate_swarm_goal))
}

/// Create a swarm plan and persist it so that `/consolidate/{goal_id}` can act
/// on the real thing (review S2).
pub async fn create_swarm_plan(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SwarmGoal>,
) -> Result<ResponseJson<ApiResponse<SwarmPlan>>, ApiError> {
    let plan = SwarmPlanner::plan(payload, &deployment.model_catalog())
        .map_err(ApiError::BadRequest)?;
    deployment.swarm_store().insert(plan.clone()).await;
    Ok(ResponseJson(ApiResponse::success(plan)))
}

/// Consolidate a previously created plan.
///
/// Reads the stored plan and derives every number from the real subtask
/// statuses (review S3). Nothing here is hardcoded and the human approval
/// barrier is preserved.
pub async fn consolidate_swarm_goal(
    State(deployment): State<DeploymentImpl>,
    Path(goal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ConsolidationReport>>, ApiError> {
    let plan = deployment
        .swarm_store()
        .get(&goal_id)
        .await
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No existe ningún plan de swarm para el objetivo '{}'. Créalo primero con POST /api/swarm/plan.",
                goal_id
            ))
        })?;

    Ok(ResponseJson(ApiResponse::success(
        build_consolidation_report(&plan),
    )))
}

/// Derive a consolidation report from a stored plan.
///
/// Every figure comes from the plan itself. Where a figure cannot be measured
/// honestly yet, the summary says so instead of inventing a value.
fn build_consolidation_report(plan: &SwarmPlan) -> ConsolidationReport {
    let total_subtasks = plan.subtasks.len();

    let completed_subtasks = plan
        .subtasks
        .iter()
        .filter(|subtask| subtask.status == SwarmSubTaskStatus::Completed)
        .count();

    let failed_subtasks = plan
        .subtasks
        .iter()
        .filter(|subtask| subtask.status == SwarmSubTaskStatus::Failed)
        .count();

    // Only completed work counts towards realised cost. The estimate for a
    // subtask that never ran is not a cost.
    //
    // `Iterator::sum()` over an *empty* f64 iterator yields `-0.0` on this
    // toolchain (verified with rustc: `-0.0`, `is_sign_negative() == true`).
    // That serializes as `-0.0` and prints as `$-0.00000` in the summary, so
    // normalise it: `-0.0 == 0.0` is true, hence a plain zero is returned.
    let completed_cost: f64 = plan
        .subtasks
        .iter()
        .filter(|subtask| subtask.status == SwarmSubTaskStatus::Completed)
        .map(|subtask| subtask.estimated_cost_usd)
        .sum();
    let total_cost_usd = if completed_cost == 0.0 {
        0.0
    } else {
        completed_cost
    };

    // Honest caveat: subtask execution is not wired to the runtime yet, so
    // there is no per-subtask start/finish timestamp to measure a real
    // duration from. We report wall-clock time elapsed since the plan was
    // created and say so explicitly rather than inventing a number.
    let elapsed = Utc::now().signed_duration_since(plan.created_at);
    let total_duration_ms = elapsed.num_milliseconds().max(0) as u64;

    let unified_branch = format!(
        "{}/swarm-{}",
        plan.goal.target_branch,
        short_uuid(&plan.goal.goal_id)
    );

    let summary = format!(
        "Objetivo de swarm '{}' ({}): {} de {} subtareas completadas, {} fallidas. Coste estimado de lo completado: ${:.5} USD. Duración reportada: {} ms de tiempo transcurrido desde la creación del plan, NO tiempo de ejecución medido (la ejecución por subtarea aún no está conectada al runtime). La consolidación requiere aprobación humana.",
        plan.goal.title,
        plan.goal.goal_id,
        completed_subtasks,
        total_subtasks,
        failed_subtasks,
        total_cost_usd,
        total_duration_ms
    );

    ConsolidationReport {
        goal_id: plan.goal.goal_id,
        total_subtasks,
        completed_subtasks,
        failed_subtasks,
        total_cost_usd,
        total_duration_ms,
        unified_branch,
        summary,
        requires_human_approval: true,
    }
}

/// First 8 characters of a UUID's canonical string form. Never panics, unlike
/// slicing `&uuid.to_string()[..8]`.
fn short_uuid(id: &Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use executors::catalog::ModelProviderCatalog;

    fn goal(title: &str) -> SwarmGoal {
        SwarmGoal {
            goal_id: Uuid::new_v4(),
            title: title.to_string(),
            description: String::new(),
            project_id: None,
            repo_id: Uuid::new_v4(),
            target_branch: "main".to_string(),
            work_mode_id: None,
            max_budget_usd: None,
        }
    }

    /// A real plan whose first N subtasks carry known statuses and known costs,
    /// so the consolidation report can be checked against explicit numbers.
    fn plan_with(statuses: &[(SwarmSubTaskStatus, f64)]) -> SwarmPlan {
        let mut plan = SwarmPlanner::plan(
            goal("Implementar el servicio de informes"),
            &ModelProviderCatalog::default(),
        )
        .expect("el catálogo por defecto debe producir un plan");

        assert!(
            statuses.len() <= plan.subtasks.len(),
            "el plan por defecto solo tiene {} subtareas",
            plan.subtasks.len()
        );

        for (subtask, (status, cost)) in plan.subtasks.iter_mut().zip(statuses.iter()) {
            subtask.status = status.clone();
            subtask.estimated_cost_usd = *cost;
        }
        plan
    }

    /// Regression guard for the defect that only live HTTP testing exposed.
    ///
    /// `Iterator::sum()` over an *empty* `f64` iterator yields `-0.0` on this
    /// toolchain. It serialised as `"total_cost_usd":-0.0` and printed as
    /// `$-0.00000` in the summary. Neither `cargo check` nor `tsc` can catch
    /// that; this test can.
    #[test]
    fn consolidation_of_a_plan_with_no_completed_work_reports_a_positive_zero() {
        let plan = plan_with(&[
            (SwarmSubTaskStatus::Pending, 0.01),
            (SwarmSubTaskStatus::Ready, 0.02),
            (SwarmSubTaskStatus::Running, 0.03),
        ]);

        let report = build_consolidation_report(&plan);

        assert_eq!(report.total_subtasks, 3);
        assert_eq!(report.completed_subtasks, 0);
        assert_eq!(report.failed_subtasks, 0);
        assert_eq!(report.total_cost_usd, 0.0);
        assert!(
            !report.total_cost_usd.is_sign_negative(),
            "el coste se serializó como -0.0"
        );
        assert!(
            !report.summary.contains("-0.00000"),
            "el resumen no debe imprimir un cero negativo: {}",
            report.summary
        );
        assert!(
            report.summary.contains("$0.00000"),
            "el resumen debe imprimir un cero limpio: {}",
            report.summary
        );
    }

    #[test]
    fn consolidation_counts_statuses_and_charges_only_for_completed_work() {
        let plan = plan_with(&[
            (SwarmSubTaskStatus::Completed, 0.01),
            (SwarmSubTaskStatus::Failed, 0.02),
            (SwarmSubTaskStatus::Completed, 0.03),
        ]);

        let report = build_consolidation_report(&plan);

        assert_eq!(report.total_subtasks, 3);
        assert_eq!(report.completed_subtasks, 2);
        assert_eq!(report.failed_subtasks, 1);
        assert!(
            (report.total_cost_usd - 0.04).abs() < 1e-12,
            "solo el trabajo completado cuenta como coste realizado (0.01 + 0.03), no {}",
            report.total_cost_usd
        );
    }

    #[test]
    fn consolidation_does_not_confuse_skipped_work_with_completed_or_failed_work() {
        let plan = plan_with(&[
            (SwarmSubTaskStatus::Completed, 0.02),
            (SwarmSubTaskStatus::Skipped, 0.05),
            (SwarmSubTaskStatus::Failed, 0.07),
        ]);

        let report = build_consolidation_report(&plan);

        assert_eq!(report.total_subtasks, 3);
        assert_eq!(report.completed_subtasks, 1);
        assert_eq!(report.failed_subtasks, 1);
        assert!(
            (report.total_cost_usd - 0.02).abs() < 1e-12,
            "una tarea omitida no es coste realizado: {}",
            report.total_cost_usd
        );
    }

    #[test]
    fn consolidation_always_requires_human_approval() {
        let plan = plan_with(&[(SwarmSubTaskStatus::Completed, 0.01)]);
        assert!(build_consolidation_report(&plan).requires_human_approval);
    }

    #[test]
    fn consolidation_builds_the_unified_branch_from_target_branch_and_short_goal_id() {
        let plan = plan_with(&[(SwarmSubTaskStatus::Completed, 0.01)]);
        let report = build_consolidation_report(&plan);

        assert_eq!(report.goal_id, plan.goal.goal_id);
        assert_eq!(
            report.unified_branch,
            format!("main/swarm-{}", short_uuid(&plan.goal.goal_id))
        );
    }

    #[test]
    fn consolidation_is_honest_about_duration_not_being_measured_execution_time() {
        let mut plan = plan_with(&[(SwarmSubTaskStatus::Completed, 0.01)]);
        plan.created_at = Utc::now() - chrono::Duration::milliseconds(250);

        let report = build_consolidation_report(&plan);

        assert!(
            report.total_duration_ms >= 250,
            "la duración debe reflejar el tiempo transcurrido desde created_at: {}",
            report.total_duration_ms
        );
        assert!(
            report.summary.contains("NO tiempo de ejecución medido"),
            "el resumen debe declarar que la duración no está medida: {}",
            report.summary
        );
        assert!(
            report.summary.contains("aprobación humana"),
            "el resumen debe declarar la barrera de aprobación humana: {}",
            report.summary
        );
    }

    #[test]
    fn short_uuid_takes_eight_characters_and_never_panics() {
        let known = Uuid::parse_str("9a1c7b67-f448-4278-9304-23252500c185").unwrap();
        assert_eq!(short_uuid(&known), "9a1c7b67");

        assert_eq!(short_uuid(&Uuid::nil()), "00000000");

        let random = Uuid::new_v4();
        let short = short_uuid(&random);
        assert_eq!(short.len(), 8);
        assert!(random.to_string().starts_with(&short));
    }
}
