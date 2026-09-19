use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::post,
};
use chrono::Utc;
use db::models::{
    execution_process::{ExecutionProcess, ExecutionProcessRunReason},
    requests::WorkspaceRepoInput,
    session::{CreateSession, Session},
    workspace::Workspace,
    workspace_repo::WorkspaceRepo,
};
use deployment::Deployment;
use executors::{
    actions::{
        ExecutorAction, ExecutorActionType, coding_agent_initial::CodingAgentInitialRequest,
    },
    executors::BaseCodingAgent,
    profile::ExecutorConfig,
};
use serde::Deserialize;
use services::services::{
    container::ContainerService,
    swarm::{
        ConsolidationReport, SwarmGoal, SwarmPlan, SwarmPlanStatus, SwarmPlanner, SwarmSubTask,
        SwarmSubTaskStatus, plan_status_from_subtasks, ready_subtasks, skip_unreachable_subtasks,
        subtask_status_from_process,
    },
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, routes::workspaces::create::create_workspace_record};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/plan", post(create_swarm_plan))
        .route("/approve/{goal_id}", post(approve_swarm_goal))
        .route("/execute/{goal_id}", post(execute_swarm_goal))
        .route("/reconcile/{goal_id}", post(reconcile_swarm_goal))
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
    deployment.swarm_store().insert(plan.clone()).await?;
    Ok(ResponseJson(ApiResponse::success(plan)))
}

/// Approve a plan so it becomes eligible to run.
///
/// This is the human barrier made executable. `SwarmPlanStatus::Approved`
/// existed in the enum but was never reachable before: `/execute` refuses to
/// start anything that a person has not approved.
pub async fn approve_swarm_goal(
    State(deployment): State<DeploymentImpl>,
    Path(goal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<SwarmPlan>>, ApiError> {
    let store = deployment.swarm_store();

    let plan = store.get(&goal_id).await?.ok_or_else(|| unknown_goal(&goal_id))?;

    if plan.status != SwarmPlanStatus::Draft {
        return Err(ApiError::Conflict(format!(
            "El plan '{}' está en estado '{}' y solo se puede aprobar desde 'DRAFT'.",
            goal_id,
            plan.status.as_wire()
        )));
    }

    store.set_status(&goal_id, SwarmPlanStatus::Approved).await?;

    let approved = store.get(&goal_id).await?.ok_or_else(|| unknown_goal(&goal_id))?;
    Ok(ResponseJson(ApiResponse::success(approved)))
}

#[derive(Debug, Default, Deserialize)]
pub struct ExecuteSwarmPlanRequest {
    /// Run this plan with a specific executor instead of the one routing
    /// recommended per subtask.
    ///
    /// It exists because it is the only way to exercise the execution path
    /// end-to-end without spending quota: the `QA_MOCK` executor
    /// (`crates/executors/src/executors/qa_mock.rs`, behind the `qa-mode`
    /// feature) is a real process with a real `ExecutionProcess` row and a real
    /// status transition, but it costs nothing. Without an override the catalog
    /// has no `QA_MOCK` route and the mock is unreachable.
    pub executor_override: Option<BaseCodingAgent>,
    /// Model to request instead of the subtask's `recommended_model`.
    pub model_override: Option<String>,
}

/// Start the subtasks that are ready right now.
///
/// One workspace per goal, one session per subtask, subtasks in dependency
/// order. The call returns as soon as the process is spawned — it does **not**
/// wait for the agent to finish. Progress is read back with `/reconcile`.
pub async fn execute_swarm_goal(
    State(deployment): State<DeploymentImpl>,
    Path(goal_id): Path<Uuid>,
    payload: Option<Json<ExecuteSwarmPlanRequest>>,
) -> Result<ResponseJson<ApiResponse<SwarmPlan>>, ApiError> {
    let request = payload.map(|Json(request)| request).unwrap_or_default();
    let store = deployment.swarm_store();

    let mut plan = store.get(&goal_id).await?.ok_or_else(|| unknown_goal(&goal_id))?;

    match plan.status {
        SwarmPlanStatus::Approved | SwarmPlanStatus::Executing => {}
        SwarmPlanStatus::Draft => {
            return Err(ApiError::Conflict(format!(
                "El plan '{}' sigue en 'DRAFT'. Apruébalo con POST /api/swarm/approve/{} antes de ejecutarlo.",
                goal_id, goal_id
            )));
        }
        SwarmPlanStatus::Completed | SwarmPlanStatus::Failed => {
            return Err(ApiError::Conflict(format!(
                "El plan '{}' ya terminó en estado '{}' y no se reabre.",
                goal_id,
                plan.status.as_wire()
            )));
        }
    }

    // Declare what can never run before starting what can, so a blocked chain
    // is visible instead of silently pending.
    skip_unreachable_subtasks(&mut plan);

    let workspace = match plan.workspace_id {
        Some(existing) => Workspace::find_by_id(&deployment.db().pool, existing)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "El plan '{}' apunta al workspace '{}', que ya no existe.",
                    goal_id, existing
                ))
            })?,
        None => {
            let workspace =
                create_workspace_record(&deployment, Some(format!("swarm-{}", short_uuid(&goal_id))))
                    .await?;

            let mut managed = deployment
                .workspace_manager()
                .load_managed_workspace(workspace)
                .await?;

            managed
                .add_repository(
                    &WorkspaceRepoInput {
                        repo_id: plan.goal.repo_id,
                        target_branch: plan.goal.target_branch.clone(),
                    },
                    deployment.git(),
                )
                .await?;

            plan.workspace_id = Some(managed.workspace.id);
            managed.workspace.clone()
        }
    };

    // Idempotent setup, once for the whole goal.
    //
    // `start_workspace` must NOT be used here. It calls `create()`, which runs
    // `WorkspaceManager::create_workspace` unconditionally, so the second
    // subtask to run in the same workspace dies with "failed to write
    // reference 'refs/heads/...': a reference with that name already exists".
    // Every other "run something in an existing workspace" route goes through
    // `ensure_container_exists`, which is the idempotent entry point.
    deployment
        .container()
        .ensure_container_exists(&workspace)
        .await?;

    // `ensure_container_exists` creates the worktree and writes `container_ref`
    // to the database, but the copy in memory still carries `None` from
    // `create_workspace_record`. `start_execution` reads `container_ref` off the
    // struct it is handed and fails with "Container ref not found" otherwise,
    // so the workspace has to be re-read after the setup step.
    let workspace = Workspace::find_by_id(&deployment.db().pool, workspace.id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "El workspace '{}' desapareció al prepararlo.",
                workspace.id
            ))
        })?;

    let repos =
        WorkspaceRepo::find_repos_for_workspace(&deployment.db().pool, workspace.id).await?;

    let ready = ready_subtasks(&plan);

    for subtask_id in ready {
        let Some(index) = plan.subtasks.iter().position(|s| s.id == subtask_id) else {
            continue;
        };

        let executor = request
            .executor_override
            .unwrap_or(plan.subtasks[index].recommended_executor);

        let model = request.model_override.clone().or_else(|| {
            let recommended = plan.subtasks[index].recommended_model.clone();
            (!recommended.is_empty()).then_some(recommended)
        });

        let mut executor_config = ExecutorConfig::new(executor);
        executor_config.model_id = model.clone();

        let prompt = build_subtask_prompt(&plan, &plan.subtasks[index]);

        // One session per subtask, so each subtask's transcript stays separable
        // instead of every subtask sharing one conversation.
        let session = Session::create(
            &deployment.db().pool,
            &CreateSession {
                executor: Some(executor.to_string()),
                name: Some(format!("{} [{}]", plan.goal.title, subtask_id)),
            },
            Uuid::new_v4(),
            workspace.id,
        )
        .await?;

        // Recomputed per subtask rather than cloned: it only inspects the repo
        // rows, and this sidesteps needing `ExecutorAction: Clone`.
        let cleanup_action = deployment.container().cleanup_actions_for_repos(&repos);

        let action = ExecutorAction::new(
            ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                prompt,
                executor_config,
                working_dir: None,
            }),
            cleanup_action.map(Box::new),
        );

        let process = deployment
            .container()
            .start_execution(
                &workspace,
                &session,
                &action,
                &ExecutionProcessRunReason::CodingAgent,
            )
            .await?;

        let subtask = &mut plan.subtasks[index];
        subtask.workspace_id = Some(workspace.id);
        subtask.execution_process_id = Some(process.id);
        subtask.status = SwarmSubTaskStatus::Running;
        subtask.started_at = Some(process.started_at);
        subtask.completed_at = None;
        subtask.failure_reason = None;
        subtask.result_summary = Some(format!(
            "Lanzada con {} en el workspace {} (proceso {}). Modelo solicitado: {}.",
            executor,
            workspace.id,
            process.id,
            model.as_deref().unwrap_or("el por defecto del ejecutor")
        ));
    }

    plan.status = plan_status_from_subtasks(&plan);
    store.save(&plan).await?;

    Ok(ResponseJson(ApiResponse::success(plan)))
}

/// Read the real `ExecutionProcess` of every started subtask and fold its state
/// back into the plan.
///
/// Reconciliation is pull-based on purpose: the alternative is a background
/// supervisor subscribing to process events, which is a much larger change and
/// is not needed to prove the path. The consequence is honest and documented —
/// a subtask's status is as fresh as the last `/reconcile` call.
pub async fn reconcile_swarm_goal(
    State(deployment): State<DeploymentImpl>,
    Path(goal_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<SwarmPlan>>, ApiError> {
    let store = deployment.swarm_store();
    let mut plan = store.get(&goal_id).await?.ok_or_else(|| unknown_goal(&goal_id))?;

    for subtask in plan.subtasks.iter_mut() {
        let Some(process_id) = subtask.execution_process_id else {
            continue;
        };

        let Some(process) = ExecutionProcess::find_by_id(&deployment.db().pool, process_id).await?
        else {
            subtask.status = SwarmSubTaskStatus::Failed;
            subtask.failure_reason = Some(format!(
                "El proceso de ejecución '{}' ya no existe en la base de datos; no se puede confirmar su resultado.",
                process_id
            ));
            continue;
        };

        let outcome = subtask_status_from_process(&process.status, process.exit_code);

        subtask.status = outcome.status.clone();
        subtask.started_at = Some(process.started_at);
        subtask.completed_at = process.completed_at;
        subtask.failure_reason = outcome.reason.clone();
    }

    skip_unreachable_subtasks(&mut plan);
    plan.status = plan_status_from_subtasks(&plan);

    store.save(&plan).await?;

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
        .await?
        .ok_or_else(|| unknown_goal(&goal_id))?;

    Ok(ResponseJson(ApiResponse::success(
        build_consolidation_report(&plan),
    )))
}

/// The prompt a subtask's agent actually receives.
///
/// Pure, so what the swarm asks an agent to do is reviewable in a test rather
/// than only visible in a live run.
fn build_subtask_prompt(plan: &SwarmPlan, subtask: &SwarmSubTask) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("Objetivo de swarm: {}\n", plan.goal.title));
    if !plan.goal.description.trim().is_empty() {
        prompt.push_str(&format!("Contexto del objetivo: {}\n", plan.goal.description));
    }
    prompt.push_str(&format!("Rama objetivo: {}\n", plan.goal.target_branch));
    prompt.push_str(&format!(
        "\nSubtarea {} de {} — rol: {}\n",
        subtask.id,
        plan.subtasks.len(),
        subtask.role
    ));
    prompt.push_str(&format!("Título: {}\n", subtask.title));
    prompt.push_str(&format!("Instrucciones: {}\n", subtask.description));

    if !subtask.dependencies.is_empty() {
        prompt.push_str(&format!(
            "Depende de: {}\n",
            subtask.dependencies.join(", ")
        ));
    }

    prompt
}

fn unknown_goal(goal_id: &Uuid) -> ApiError {
    ApiError::NotFound(format!(
        "No existe ningún plan de swarm para el objetivo '{}'. Créalo primero con POST /api/swarm/plan.",
        goal_id
    ))
}

/// Derive a consolidation report from a stored plan.
///
/// Every figure comes from the plan itself. Where a figure cannot be measured
/// honestly, the summary says so instead of inventing a value.
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

    // A real measured duration, when subtasks have actually run and carry the
    // timestamps of the processes behind them. Before that, fall back to
    // wall-clock time since the plan was created and say plainly that it is not
    // execution time.
    let mut measured_ms: i64 = 0;
    let mut measured_any = false;
    for subtask in &plan.subtasks {
        if let (Some(started), Some(completed)) = (subtask.started_at, subtask.completed_at) {
            measured_ms += completed
                .signed_duration_since(started)
                .num_milliseconds()
                .max(0);
            measured_any = true;
        }
    }

    let (total_duration_ms, duration_note) = if measured_any {
        (
            measured_ms as u64,
            "suma de las duraciones medidas de las subtareas que han terminado".to_string(),
        )
    } else {
        let elapsed = Utc::now().signed_duration_since(plan.created_at);
        (
            elapsed.num_milliseconds().max(0) as u64,
            "tiempo transcurrido desde la creación del plan, NO tiempo de ejecución medido (ninguna subtarea ha terminado todavía)"
                .to_string(),
        )
    };

    let unified_branch = format!(
        "{}/swarm-{}",
        plan.goal.target_branch,
        short_uuid(&plan.goal.goal_id)
    );

    let summary = format!(
        "Objetivo de swarm '{}' ({}): {} de {} subtareas completadas, {} fallidas. Coste estimado de lo completado: ${:.5} USD. Duración reportada: {} ms, {}). La consolidación requiere aprobación humana.",
        plan.goal.title,
        plan.goal.goal_id,
        completed_subtasks,
        total_subtasks,
        failed_subtasks,
        total_cost_usd,
        total_duration_ms,
        duration_note
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

    /// Once subtasks have really run, the duration must stop being
    /// wall-clock-since-creation and become the measured execution time.
    #[test]
    fn consolidation_reports_the_measured_duration_once_subtasks_have_run() {
        let mut plan = plan_with(&[
            (SwarmSubTaskStatus::Completed, 0.01),
            (SwarmSubTaskStatus::Pending, 0.0),
            (SwarmSubTaskStatus::Pending, 0.0),
        ]);

        // Created long ago, but only ~400 ms of real work happened.
        plan.created_at = Utc::now() - chrono::Duration::hours(3);
        plan.subtasks[0].started_at = Some(Utc::now() - chrono::Duration::milliseconds(400));
        plan.subtasks[0].completed_at = Some(Utc::now());

        let report = build_consolidation_report(&plan);

        assert!(
            (300..=2000).contains(&report.total_duration_ms),
            "la duración debe ser el tiempo medido de la subtarea (~400 ms), no las 3 horas desde created_at: {}",
            report.total_duration_ms
        );
        assert!(
            report.summary.contains("duraciones medidas"),
            "el resumen debe declarar que la duración está medida: {}",
            report.summary
        );
        assert!(
            !report.summary.contains("NO tiempo de ejecución medido"),
            "no debe seguir declarando que la duración no está medida: {}",
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

    /// What the swarm actually asks an agent to do must be reviewable without
    /// launching one.
    #[test]
    fn the_subtask_prompt_carries_the_goal_the_subtask_and_its_role() {
        let mut plan = plan_with(&[(SwarmSubTaskStatus::Pending, 0.0)]);
        plan.goal.title = "Refactorizar el módulo de facturación".to_string();
        plan.goal.description = "Sin cambiar la API pública".to_string();
        plan.goal.target_branch = "main".to_string();

        let subtask = plan.subtasks[0].clone();
        let prompt = build_subtask_prompt(&plan, &subtask);

        assert!(prompt.contains("Refactorizar el módulo de facturación"));
        assert!(prompt.contains("Sin cambiar la API pública"));
        assert!(prompt.contains("Rama objetivo: main"));
        assert!(prompt.contains(&subtask.title));
        assert!(prompt.contains(&subtask.description));
        assert!(prompt.contains(&subtask.role));
        assert!(
            prompt.contains("Subtarea task-1 de 3"),
            "el prompt debe situar la subtarea dentro del plan: {prompt}"
        );
    }

    #[test]
    fn the_subtask_prompt_declares_dependencies_when_there_are_any() {
        let plan = plan_with(&[
            (SwarmSubTaskStatus::Completed, 0.0),
            (SwarmSubTaskStatus::Pending, 0.0),
            (SwarmSubTaskStatus::Pending, 0.0),
        ]);

        let dependent = plan.subtasks[1].clone();
        let prompt = build_subtask_prompt(&plan, &dependent);
        assert!(
            prompt.contains("Depende de: task-1"),
            "una subtarea con dependencias debe declararlas: {prompt}"
        );

        let first = plan.subtasks[0].clone();
        let prompt_without_deps = build_subtask_prompt(&plan, &first);
        assert!(
            !prompt_without_deps.contains("Depende de:"),
            "una subtarea sin dependencias no debe declarar ninguna: {prompt_without_deps}"
        );
    }
}
