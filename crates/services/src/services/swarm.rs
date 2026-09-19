use chrono::{DateTime, Utc};
use db::models::{
    execution_process::ExecutionProcessStatus,
    swarm_plan::{SwarmPlanRecord, SwarmPlanRecordError},
};
use executors::catalog::{
    DEFAULT_FALLBACK_ROUTE_ID, DEFAULT_WORK_MODE_ID, ModelProviderCatalog,
    RoutingRecommendation, RoutingRequest,
};
use executors::executors::BaseCodingAgent;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwarmSubTaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwarmPlanStatus {
    Draft,
    Approved,
    Executing,
    Completed,
    Failed,
}

impl SwarmPlanStatus {
    /// The text persisted in `swarm_plans.status`.
    ///
    /// Hand-written on purpose, and pinned by
    /// `every_plan_status_round_trips_through_its_wire_form` to both the serde
    /// representation and the table's `CHECK` constraint. Review D4 showed what
    /// happens when a wire form is assumed rather than asserted: the `?role=`
    /// filter compared against Rust's `Debug` output (`Planning`), returned an
    /// empty list with HTTP 200, and nobody noticed.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Approved => "APPROVED",
            Self::Executing => "EXECUTING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
        }
    }

    /// Inverse of [`SwarmPlanStatus::as_wire`]. `None` for anything else, so an
    /// unknown value from the database surfaces as an error instead of
    /// defaulting to something plausible.
    pub fn from_wire(raw: &str) -> Option<Self> {
        match raw {
            "DRAFT" => Some(Self::Draft),
            "APPROVED" => Some(Self::Approved),
            "EXECUTING" => Some(Self::Executing),
            "COMPLETED" => Some(Self::Completed),
            "FAILED" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SwarmGoal {
    pub goal_id: Uuid,
    pub title: String,
    pub description: String,
    pub project_id: Option<Uuid>,
    pub repo_id: Uuid,
    pub target_branch: String,
    pub work_mode_id: Option<String>,
    pub max_budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SwarmSubTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub role: String,
    pub dependencies: Vec<String>,
    pub recommended_executor: BaseCodingAgent,
    pub recommended_model: String,
    pub estimated_cost_usd: f64,
    pub explanation: String,
    pub status: SwarmSubTaskStatus,
    pub workspace_id: Option<Uuid>,
    pub execution_process_id: Option<Uuid>,
    pub result_summary: Option<String>,
    /// When the real `ExecutionProcess` backing this subtask started. `None`
    /// until it actually runs — never invented from the plan's creation time.
    pub started_at: Option<DateTime<Utc>>,
    /// When that process reached a terminal state.
    pub completed_at: Option<DateTime<Utc>>,
    /// Why the subtask is not a clean success. Always present for `Failed`,
    /// including when the process was killed rather than genuinely failing.
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SwarmPlan {
    pub goal: SwarmGoal,
    pub subtasks: Vec<SwarmSubTask>,
    pub total_estimated_cost_usd: f64,
    pub status: SwarmPlanStatus,
    pub created_at: DateTime<Utc>,
    /// The single worktree every subtask of this goal runs in.
    ///
    /// One workspace per goal, not per subtask: the subtasks of a goal are a
    /// dependency chain over the same branch, and the consolidation report
    /// already advertises a single `unified_branch`. `None` until the plan is
    /// first executed.
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ConsolidationReport {
    pub goal_id: Uuid,
    pub total_subtasks: usize,
    pub completed_subtasks: usize,
    pub failed_subtasks: usize,
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
    pub unified_branch: String,
    pub summary: String,
    pub requires_human_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutcome {
    pub status: SwarmSubTaskStatus,
    /// Why, whenever the status is not a clean success. A `Failed` without a
    /// stated reason would be indistinguishable from a bug.
    pub reason: Option<String>,
}

/// Map the state of a real `ExecutionProcess` onto a subtask status.
///
/// Pure, and deliberately so: this is the function that decides whether work
/// counted as done, which is exactly the kind of decision that must be
/// reachable from a unit test without standing up a deployment (the same
/// reasoning that produced `filter_work_modes` after review D4).
///
/// It refuses to launder outcomes:
///
/// - a process that reports `Completed` with a **non-zero** exit code is a
///   failure, not a success;
/// - a process that reports `Completed` with **no** exit code is a failure,
///   because success cannot be confirmed and the project does not claim what it
///   cannot evidence;
/// - a process that was **killed** is a failure with a reason that says so, so
///   it never reads as the subtask having genuinely broken.
pub fn subtask_status_from_process(
    process_status: &ExecutionProcessStatus,
    exit_code: Option<i64>,
) -> ProcessOutcome {
    match process_status {
        ExecutionProcessStatus::Running => ProcessOutcome {
            status: SwarmSubTaskStatus::Running,
            reason: None,
        },
        ExecutionProcessStatus::Completed => match exit_code {
            Some(0) => ProcessOutcome {
                status: SwarmSubTaskStatus::Completed,
                reason: None,
            },
            Some(code) => ProcessOutcome {
                status: SwarmSubTaskStatus::Failed,
                reason: Some(format!(
                    "el proceso se declaró completado pero terminó con exit code {code}"
                )),
            },
            None => ProcessOutcome {
                status: SwarmSubTaskStatus::Failed,
                reason: Some(
                    "el proceso se declaró completado sin exit code registrado; no se puede confirmar el éxito"
                        .to_string(),
                ),
            },
        },
        ExecutionProcessStatus::Failed => ProcessOutcome {
            status: SwarmSubTaskStatus::Failed,
            reason: Some(match exit_code {
                Some(code) => format!("el proceso terminó en estado 'failed' con exit code {code}"),
                None => "el proceso terminó en estado 'failed' sin exit code".to_string(),
            }),
        },
        ExecutionProcessStatus::Killed => ProcessOutcome {
            status: SwarmSubTaskStatus::Failed,
            reason: Some(
                "el proceso fue terminado (killed); no falló por sí mismo ni llegó a completarse"
                    .to_string(),
            ),
        },
    }
}

/// Subtasks that can be started right now: still `Pending`, with every
/// dependency already `Completed`.
///
/// A dependency that does not exist in the plan is treated as unsatisfied, so a
/// typo in the DAG leaves the subtask pending and visible instead of starting
/// work whose prerequisites were never defined.
pub fn ready_subtasks(plan: &SwarmPlan) -> Vec<String> {
    plan.subtasks
        .iter()
        .filter(|subtask| subtask.status == SwarmSubTaskStatus::Pending)
        .filter(|subtask| {
            subtask.dependencies.iter().all(|dependency| {
                plan.subtasks.iter().any(|candidate| {
                    &candidate.id == dependency && candidate.status == SwarmSubTaskStatus::Completed
                })
            })
        })
        .map(|subtask| subtask.id.clone())
        .collect()
}

/// `Pending` subtasks that can never run, because a dependency already failed or
/// was skipped. They exist so the plan can say "this will never happen" instead
/// of leaving subtasks pending forever with no explanation.
pub fn unreachable_subtasks(plan: &SwarmPlan) -> Vec<String> {
    plan.subtasks
        .iter()
        .filter(|subtask| subtask.status == SwarmSubTaskStatus::Pending)
        .filter(|subtask| {
            subtask.dependencies.iter().any(|dependency| {
                plan.subtasks.iter().any(|candidate| {
                    &candidate.id == dependency
                        && matches!(
                            candidate.status,
                            SwarmSubTaskStatus::Failed | SwarmSubTaskStatus::Skipped
                        )
                })
            })
        })
        .map(|subtask| subtask.id.clone())
        .collect()
}

/// Mark `Pending` subtasks that can never run as `Skipped`, with a reason, and
/// return the ids that were skipped.
///
/// Without this they stay `Pending` forever and the plan reads as stalled
/// rather than blocked. It runs to a fixpoint on purpose: skipping `task-2`
/// makes `task-3`, which depended on it, unreachable in turn. Each pass
/// converts at least one `Pending` into `Skipped`, so it always terminates.
pub fn skip_unreachable_subtasks(plan: &mut SwarmPlan) -> Vec<String> {
    let mut skipped = Vec::new();

    loop {
        let batch = unreachable_subtasks(plan);
        if batch.is_empty() {
            break;
        }

        for subtask_id in &batch {
            if let Some(subtask) = plan.subtasks.iter_mut().find(|s| &s.id == subtask_id) {
                subtask.status = SwarmSubTaskStatus::Skipped;
                subtask.failure_reason = Some(format!(
                    "No puede ejecutarse: una dependencia falló o fue omitida ({})",
                    subtask.dependencies.join(", ")
                ));
            }
        }

        skipped.extend(batch);
    }

    skipped
}

/// Derive the plan's lifecycle status from its subtasks.
///
/// Work still in flight wins over an already-recorded failure: a plan with one
/// failed subtask and another still running is `Executing`, not `Failed`.
pub fn plan_status_from_subtasks(plan: &SwarmPlan) -> SwarmPlanStatus {
    if plan
        .subtasks
        .iter()
        .any(|subtask| matches!(subtask.status, SwarmSubTaskStatus::Running | SwarmSubTaskStatus::Ready))
    {
        return SwarmPlanStatus::Executing;
    }

    if plan
        .subtasks
        .iter()
        .any(|subtask| subtask.status == SwarmSubTaskStatus::Failed)
    {
        return SwarmPlanStatus::Failed;
    }

    // A plan with no subtasks is vacuously complete: there is no outstanding
    // work. `SwarmPlanner` never emits one, and the empty case is asserted in a
    // test so this stays a decision rather than an accident of `all()`.
    if plan
        .subtasks
        .iter()
        .all(|subtask| {
            matches!(
                subtask.status,
                SwarmSubTaskStatus::Completed | SwarmSubTaskStatus::Skipped
            )
        })
    {
        return SwarmPlanStatus::Completed;
    }

    SwarmPlanStatus::Executing
}

pub struct SwarmPlanner;

#[derive(Debug, Error)]
pub enum SwarmStoreError {
    #[error(transparent)]
    Record(#[from] SwarmPlanRecordError),
    #[error("El plan de swarm de '{0}' no se pudo deserializar desde la base de datos: {1}")]
    CorruptPlan(Uuid, #[source] serde_json::Error),
    #[error("El plan de swarm de '{0}' no se pudo serializar: {1}")]
    SerializePlan(Uuid, #[source] serde_json::Error),
    #[error("El plan de swarm de '{0}' tiene un estado almacenado desconocido: '{1}'")]
    UnknownStoredStatus(Uuid, String),
}

/// Persistent store for swarm plans.
///
/// Review S2/S3 first made `/plan` persist its plan so that `/consolidate`
/// could stop returning fabricated numbers. That store was an in-memory
/// `HashMap`, which ADR-017 accepted **only** while the swarm did not execute
/// anything. Now that subtasks run against the runtime, a plan lost on restart
/// would leave a live process with no plan to explain it, so the store is
/// SQLite-backed and the ADR-017 debt is settled.
///
/// Status lives in two places — the `status` column and the serialised document
/// — and they are written together in every path, so they cannot disagree.
#[derive(Clone)]
pub struct SwarmStore {
    pool: SqlitePool,
}

impl SwarmStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Store a plan, keyed by its goal id. Returns the key it was stored under.
    pub async fn insert(&self, plan: SwarmPlan) -> Result<Uuid, SwarmStoreError> {
        let goal_id = plan.goal.goal_id;
        self.save(&plan).await?;
        Ok(goal_id)
    }

    /// Persist the whole document, including its current status.
    pub async fn save(&self, plan: &SwarmPlan) -> Result<(), SwarmStoreError> {
        let goal_id = plan.goal.goal_id;
        let json = serde_json::to_string(plan)
            .map_err(|err| SwarmStoreError::SerializePlan(goal_id, err))?;

        SwarmPlanRecord::upsert(&self.pool, goal_id, &json, plan.status.as_wire()).await?;

        Ok(())
    }

    pub async fn get(&self, goal_id: &Uuid) -> Result<Option<SwarmPlan>, SwarmStoreError> {
        let Some(record) = SwarmPlanRecord::find(&self.pool, *goal_id).await? else {
            return Ok(None);
        };

        let mut plan: SwarmPlan = serde_json::from_str(&record.plan_json)
            .map_err(|err| SwarmStoreError::CorruptPlan(*goal_id, err))?;

        // The column is authoritative for status, and `save`/`set_status` keep
        // the document in step with it. Reading it back from the column means a
        // hand-edited database cannot make the two silently disagree.
        plan.status = SwarmPlanStatus::from_wire(&record.status)
            .ok_or_else(|| SwarmStoreError::UnknownStoredStatus(*goal_id, record.status.clone()))?;

        Ok(Some(plan))
    }

    /// Update the lifecycle status of a stored plan.
    ///
    /// Returns `false` when no plan with that id exists. The document is
    /// rewritten too, so the JSON and the column never diverge.
    pub async fn set_status(
        &self,
        goal_id: &Uuid,
        status: SwarmPlanStatus,
    ) -> Result<bool, SwarmStoreError> {
        let Some(mut plan) = self.get(goal_id).await? else {
            return Ok(false);
        };

        plan.status = status;
        self.save(&plan).await?;

        Ok(true)
    }

    /// All stored plans, oldest first.
    pub async fn list(&self) -> Result<Vec<SwarmPlan>, SwarmStoreError> {
        let mut plans = Vec::new();
        for record in SwarmPlanRecord::list(&self.pool).await? {
            plans.push(self.decode(&record.goal_id, &record.plan_json, &record.status)?);
        }

        Ok(plans)
    }

    pub async fn len(&self) -> Result<usize, SwarmStoreError> {
        Ok(SwarmPlanRecord::count(&self.pool).await? as usize)
    }

    pub async fn is_empty(&self) -> Result<bool, SwarmStoreError> {
        Ok(self.len().await? == 0)
    }

    fn decode(
        &self,
        goal_id: &Uuid,
        plan_json: &str,
        status: &str,
    ) -> Result<SwarmPlan, SwarmStoreError> {
        let mut plan: SwarmPlan = serde_json::from_str(plan_json)
            .map_err(|err| SwarmStoreError::CorruptPlan(*goal_id, err))?;
        plan.status = SwarmPlanStatus::from_wire(status)
            .ok_or_else(|| SwarmStoreError::UnknownStoredStatus(*goal_id, status.to_string()))?;

        Ok(plan)
    }
}

impl SwarmPlanner {
    /// Decompose a goal into a DAG of subtasks.
    ///
    /// Returns `Err` only when the catalog itself is unusable (a route or work
    /// mode that must exist is missing). A routing miss on an individual
    /// subtask degrades to the documented default route instead of failing the
    /// whole plan.
    pub fn plan(goal: SwarmGoal, catalog: &ModelProviderCatalog) -> Result<SwarmPlan, String> {
        let title_lower = goal.title.to_lowercase();
        let desc_lower = goal.description.to_lowercase();
        let combined = format!("{} {}", title_lower, desc_lower);

        let subtasks = if combined.contains("refactor")
            || combined.contains("limpieza")
            || combined.contains("optimizar")
        {
            vec![
                Self::create_subtask(
                    "task-1",
                    "Análisis de impacto y detección de hotspots",
                    "Explorar el repositorio para identificar dependencias críticas y puntos de refactorización.",
                    "Analista de Código",
                    vec![],
                    "planning",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.3),
                )?,
                Self::create_subtask(
                    "task-2",
                    "Implementación de refactorización",
                    "Aplicar los cambios estructurales preservando el comportamiento de las interfaces públicas.",
                    "Desarrollador Principal",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.5),
                )?,
                Self::create_subtask(
                    "task-3",
                    "Verificación de regresiones y suites de pruebas",
                    "Ejecutar pruebas unitarias y linters para certificar que ningún contrato se ha roto.",
                    "Ingeniero de Calidad (QA)",
                    vec!["task-2".to_string()],
                    "free_only",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.2),
                )?,
            ]
        } else if combined.contains("doc")
            || combined.contains("guía")
            || combined.contains("manual")
        {
            vec![
                Self::create_subtask(
                    "task-1",
                    "Extracción técnica y análisis de interfaces",
                    "Inspeccionar firmas, tipos y módulos para extraer la estructura documental.",
                    "Arquitecto de Documentación",
                    vec![],
                    "planning",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.4),
                )?,
                Self::create_subtask(
                    "task-2",
                    "Redacción y formateo de documentación",
                    "Elaborar las guías de uso, diagramas conceptuales y ejemplos prácticos en Markdown.",
                    "Escritor Técnico",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.6),
                )?,
            ]
        } else {
            vec![
                Self::create_subtask(
                    "task-1",
                    "Diseño técnico y contratos de arquitectura",
                    &format!("Diseñar los tipos y contratos necesarios para: {}", goal.title),
                    "Arquitecto de Software",
                    vec![],
                    "planning",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.35),
                )?,
                Self::create_subtask(
                    "task-2",
                    "Implementación de funcionalidad y servicios",
                    &format!("Construir la lógica funcional y endpoints para: {}", goal.title),
                    "Ingeniero de Software",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.45),
                )?,
                Self::create_subtask(
                    "task-3",
                    "Pruebas unitarias, tipado y validación E2E",
                    &format!("Implementar pruebas automatizadas y validar compilación para: {}", goal.title),
                    "Ingeniero de QA / Testing",
                    vec!["task-2".to_string()],
                    "free_only",
                    catalog,
                    goal.max_budget_usd.map(|b| b * 0.20),
                )?,
            ]
        };

        let total_cost = subtasks.iter().map(|t| t.estimated_cost_usd).sum();

        Ok(SwarmPlan {
            goal,
            subtasks,
            total_estimated_cost_usd: total_cost,
            status: SwarmPlanStatus::Draft,
            created_at: Utc::now(),
            workspace_id: None,
        })
    }

    fn create_subtask(
        id: &str,
        title: &str,
        description: &str,
        role: &str,
        dependencies: Vec<String>,
        mode: &str,
        catalog: &ModelProviderCatalog,
        budget: Option<f64>,
    ) -> Result<SwarmSubTask, String> {
        let rec = match catalog.recommend(&RoutingRequest {
            work_mode_id: Some(mode.to_string()),
            task_type: Some(title.to_string()),
            max_budget_usd: budget,
            required_capabilities: None,
        }) {
            Ok(rec) => rec,
            // S5: no `.unwrap()` here. A routing miss degrades to the
            // documented default route; if even that entry is missing the
            // catalog is misconfigured and we return an error the HTTP layer
            // surfaces as a 400 rather than panicking the server.
            Err(err) => {
                let default_route = catalog
                    .find_route(DEFAULT_FALLBACK_ROUTE_ID)
                    .ok_or_else(|| {
                        format!(
                            "Ruta de fallback '{}' ausente del catálogo (motivo del fallback: {})",
                            DEFAULT_FALLBACK_ROUTE_ID, err
                        )
                    })?;

                let work_mode = catalog
                    .find_work_mode(DEFAULT_WORK_MODE_ID)
                    .ok_or_else(|| {
                        format!(
                            "Modo de trabajo '{}' ausente del catálogo",
                            DEFAULT_WORK_MODE_ID
                        )
                    })?;

                RoutingRecommendation {
                    selected_route: default_route.clone(),
                    selected_executor: default_route.executor,
                    selected_model_id: default_route.real_model_id,
                    work_mode,
                    estimated_cost_usd: 0.0,
                    explanation: format!(
                        "Ruta de fallback por defecto ('{}'). Motivo del fallback: {}",
                        DEFAULT_FALLBACK_ROUTE_ID, err
                    ),
                    fallback_chain: vec![],
                }
            }
        };

        Ok(SwarmSubTask {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            role: role.to_string(),
            dependencies,
            recommended_executor: rec.selected_executor,
            recommended_model: rec.selected_model_id,
            estimated_cost_usd: rec.estimated_cost_usd,
            explanation: rec.explanation,
            status: SwarmSubTaskStatus::Pending,
            workspace_id: None,
            execution_process_id: None,
            result_summary: None,
            started_at: None,
            completed_at: None,
            failure_reason: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::swarm_plan::VALID_SWARM_PLAN_STATUSES;

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

    fn default_plan(title: &str) -> SwarmPlan {
        SwarmPlanner::plan(goal(title), &ModelProviderCatalog::default())
            .expect("el catálogo por defecto debe producir siempre un plan")
    }

    /// A plan whose subtask statuses are set explicitly, so the pure
    /// dependency logic can be exercised against known states.
    fn plan_with_statuses(statuses: &[SwarmSubTaskStatus]) -> SwarmPlan {
        let mut plan = default_plan("Implementar el servicio de informes");
        assert!(
            statuses.len() <= plan.subtasks.len(),
            "el plan por defecto solo tiene {} subtareas",
            plan.subtasks.len()
        );

        for (subtask, status) in plan.subtasks.iter_mut().zip(statuses.iter()) {
            subtask.status = status.clone();
        }
        plan
    }

    // --- Persistencia: el documento debe sobrevivir el viaje por JSON -------

    /// The store persists the plan as a JSON document. If a field were lost in
    /// that round trip, consolidation would quietly report on a plan that is
    /// not the one that ran.
    #[test]
    fn a_plan_survives_the_json_round_trip_used_by_the_store() {
        let mut plan = default_plan("Implementar un endpoint");
        plan.subtasks[0].workspace_id = Some(Uuid::new_v4());
        plan.subtasks[0].execution_process_id = Some(Uuid::new_v4());
        plan.subtasks[0].status = SwarmSubTaskStatus::Running;
        plan.subtasks[0].started_at = Some(Utc::now());
        plan.subtasks[1].failure_reason = Some("motivo explícito".to_string());

        let json = serde_json::to_string(&plan).expect("serializar");
        let back: SwarmPlan = serde_json::from_str(&json).expect("deserializar");

        assert_eq!(back.goal.goal_id, plan.goal.goal_id);
        assert_eq!(back.status, plan.status);
        assert_eq!(back.subtasks.len(), plan.subtasks.len());
        assert_eq!(back.subtasks[0].status, SwarmSubTaskStatus::Running);
        assert_eq!(
            back.subtasks[0].execution_process_id,
            plan.subtasks[0].execution_process_id
        );
        assert_eq!(back.subtasks[0].started_at, plan.subtasks[0].started_at);
        assert_eq!(
            back.subtasks[1].failure_reason.as_deref(),
            Some("motivo explícito")
        );
    }

    /// The persisted status text, the serde form and the table's `CHECK`
    /// constraint must be the same three strings. Review D4 is the precedent:
    /// a wire form that was assumed instead of asserted produced a silent wrong
    /// answer.
    #[test]
    fn every_plan_status_round_trips_through_its_wire_form() {
        let all = [
            SwarmPlanStatus::Draft,
            SwarmPlanStatus::Approved,
            SwarmPlanStatus::Executing,
            SwarmPlanStatus::Completed,
            SwarmPlanStatus::Failed,
        ];

        for status in all {
            let wire = status.as_wire();

            let from_serde: SwarmPlanStatus =
                serde_json::from_str(&serde_json::to_string(&status).expect("serializar"))
                    .expect("deserializar");
            assert_eq!(
                from_serde, status,
                "el serde de {status:?} no vuelve a ser el mismo valor"
            );

            let serde_form = serde_json::to_string(&status).expect("serializar");
            assert_eq!(
                serde_form,
                format!("\"{wire}\""),
                "la forma serde de {status:?} y su forma persistida no coinciden"
            );

            assert!(
                VALID_SWARM_PLAN_STATUSES.contains(&wire),
                "'{wire}' no satisface el CHECK de swarm_plans.status"
            );

            assert_eq!(
                SwarmPlanStatus::from_wire(wire),
                Some(status.clone()),
                "'{wire}' no vuelve a ser {status:?}"
            );
        }

        assert_eq!(SwarmPlanStatus::from_wire("BOGUS"), None);
        assert_eq!(SwarmPlanStatus::from_wire("Draft"), None);
        assert_eq!(SwarmPlanStatus::from_wire(""), None);
    }

    // --- Mapeo proceso -> subtarea ----------------------------------------

    #[test]
    fn a_running_process_maps_to_a_running_subtask_with_no_failure_reason() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Running, None);

        assert_eq!(outcome.status, SwarmSubTaskStatus::Running);
        assert_eq!(outcome.reason, None);
    }

    #[test]
    fn a_completed_process_with_exit_code_zero_is_a_clean_success() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Completed, Some(0));

        assert_eq!(outcome.status, SwarmSubTaskStatus::Completed);
        assert_eq!(
            outcome.reason, None,
            "un éxito limpio no debe llevar motivo de fallo"
        );
    }

    /// A process that claims to be completed but exited non-zero did not
    /// succeed. Laundering it into `Completed` would be the same class of
    /// defect as the Antigravity false positive recorded in Fase 4.
    #[test]
    fn a_completed_process_with_a_non_zero_exit_code_is_a_failure() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Completed, Some(1));

        assert_eq!(outcome.status, SwarmSubTaskStatus::Failed);
        assert!(
            outcome
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("exit code 1")),
            "el motivo debe nombrar el exit code real: {:?}",
            outcome.reason
        );
    }

    /// Success that cannot be evidenced is not success.
    #[test]
    fn a_completed_process_without_an_exit_code_is_not_treated_as_success() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Completed, None);

        assert_eq!(outcome.status, SwarmSubTaskStatus::Failed);
        assert!(
            outcome
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("sin exit code")),
            "el motivo debe declarar que falta el exit code: {:?}",
            outcome.reason
        );
    }

    #[test]
    fn a_failed_process_is_a_failure_and_says_so() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Failed, Some(101));

        assert_eq!(outcome.status, SwarmSubTaskStatus::Failed);
        assert!(
            outcome
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("101")),
            "el motivo debe incluir el exit code cuando existe: {:?}",
            outcome.reason
        );
    }

    /// A killed process is a failure, but never one that reads as the subtask
    /// having genuinely broken on its own.
    #[test]
    fn a_killed_process_is_a_failure_that_declares_it_was_killed() {
        let outcome = subtask_status_from_process(&ExecutionProcessStatus::Killed, None);

        assert_eq!(outcome.status, SwarmSubTaskStatus::Failed);
        assert!(
            outcome
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("killed")),
            "el motivo debe distinguir una parada de un fallo real: {:?}",
            outcome.reason
        );
    }

    #[test]
    fn no_process_state_ever_maps_to_a_silent_failure() {
        let states = [
            ExecutionProcessStatus::Running,
            ExecutionProcessStatus::Completed,
            ExecutionProcessStatus::Failed,
            ExecutionProcessStatus::Killed,
        ];

        for state in states {
            for exit_code in [None, Some(0), Some(1)] {
                let outcome = subtask_status_from_process(&state, exit_code);

                if outcome.status == SwarmSubTaskStatus::Failed {
                    assert!(
                        outcome.reason.is_some(),
                        "{state:?} con exit {exit_code:?} produjo un fallo sin motivo"
                    );
                }
                assert!(
                    outcome.status != SwarmSubTaskStatus::Completed
                        || (matches!(state, ExecutionProcessStatus::Completed)
                            && exit_code == Some(0)),
                    "{state:?} con exit {exit_code:?} no puede contar como completado"
                );
            }
        }
    }

    // --- Conjunto de subtareas listas -------------------------------------

    #[test]
    fn only_the_first_subtask_is_ready_in_a_fresh_plan() {
        let plan = default_plan("Implementar el servicio de informes");

        assert_eq!(ready_subtasks(&plan), vec!["task-1".to_string()]);
    }

    #[test]
    fn completing_a_dependency_makes_the_next_subtask_ready() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert_eq!(ready_subtasks(&plan), vec!["task-2".to_string()]);
    }

    #[test]
    fn a_running_subtask_is_not_offered_again() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Running,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert!(
            ready_subtasks(&plan).is_empty(),
            "una subtarea ya en marcha no debe volver a lanzarse"
        );
    }

    #[test]
    fn a_subtask_whose_dependency_does_not_exist_is_never_ready() {
        let mut plan = default_plan("Implementar el servicio de informes");
        plan.subtasks[0].dependencies = vec!["task-inexistente".to_string()];

        assert!(
            ready_subtasks(&plan).is_empty(),
            "una dependencia inexistente no puede darse por satisfecha"
        );
    }

    #[test]
    fn a_subtask_behind_a_failed_dependency_is_unreachable_not_ready() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Failed,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert!(ready_subtasks(&plan).is_empty());
        assert_eq!(unreachable_subtasks(&plan), vec!["task-2".to_string()]);
    }

    #[test]
    fn a_subtask_behind_a_skipped_dependency_is_also_unreachable() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Skipped,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert_eq!(unreachable_subtasks(&plan), vec!["task-2".to_string()]);
    }

    #[test]
    fn skipping_a_blocked_subtask_cascades_down_the_chain() {
        let mut plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Failed,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        let skipped = skip_unreachable_subtasks(&mut plan);

        assert_eq!(
            skipped,
            vec!["task-2".to_string(), "task-3".to_string()],
            "omitir task-2 debe arrastrar a task-3, que dependía de ella"
        );

        for subtask in &plan.subtasks {
            if subtask.id != "task-1" {
                assert_eq!(subtask.status, SwarmSubTaskStatus::Skipped);
                assert!(
                    subtask
                        .failure_reason
                        .as_deref()
                        .is_some_and(|reason| reason.contains("dependencia")),
                    "una subtarea omitida debe decir por qué: {:?}",
                    subtask.failure_reason
                );
                assert!(
                    subtask.completed_at.is_none(),
                    "una subtarea que nunca corrió no puede declarar cuándo terminó"
                );
            }
        }
    }

    #[test]
    fn nothing_is_skipped_while_the_chain_is_healthy() {
        let mut plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert!(skip_unreachable_subtasks(&mut plan).is_empty());
        assert_eq!(plan.subtasks[1].status, SwarmSubTaskStatus::Pending);
    }

    // --- Estado del plan --------------------------------------------------

    #[test]
    fn a_plan_with_work_in_flight_is_executing_even_if_something_already_failed() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Failed,
            SwarmSubTaskStatus::Running,
            SwarmSubTaskStatus::Pending,
        ]);

        assert_eq!(plan_status_from_subtasks(&plan), SwarmPlanStatus::Executing);
    }

    #[test]
    fn a_plan_with_a_failure_and_nothing_running_is_failed() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Failed,
            SwarmSubTaskStatus::Skipped,
        ]);

        assert_eq!(plan_status_from_subtasks(&plan), SwarmPlanStatus::Failed);
    }

    #[test]
    fn a_plan_whose_work_is_all_settled_is_completed() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Skipped,
        ]);

        assert_eq!(plan_status_from_subtasks(&plan), SwarmPlanStatus::Completed);
    }

    #[test]
    fn a_plan_with_pending_work_and_nothing_running_is_still_executing() {
        let plan = plan_with_statuses(&[
            SwarmSubTaskStatus::Completed,
            SwarmSubTaskStatus::Pending,
            SwarmSubTaskStatus::Pending,
        ]);

        assert_eq!(plan_status_from_subtasks(&plan), SwarmPlanStatus::Executing);
    }

    /// `all()` over an empty collection is `true`, which would make a plan with
    /// no subtasks report as `Completed` by accident. The planner never emits
    /// one, so the case is pinned deliberately instead of left implicit.
    #[test]
    fn a_plan_without_subtasks_is_vacuously_completed() {
        let mut plan = default_plan("Implementar el servicio de informes");
        plan.subtasks.clear();

        assert_eq!(plan_status_from_subtasks(&plan), SwarmPlanStatus::Completed);
    }

    // --- SwarmPlanner -------------------------------------------------------

    #[test]
    fn planner_builds_a_dependency_chain_of_pending_subtasks() {
        let catalog = ModelProviderCatalog::default();
        let plan = SwarmPlanner::plan(goal("Refactorizar el módulo de facturación"), &catalog)
            .expect("un objetivo de refactor debe planificarse");

        assert_eq!(plan.status, SwarmPlanStatus::Draft);
        assert_eq!(plan.subtasks.len(), 3);

        let ids: Vec<&str> = plan.subtasks.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["task-1", "task-2", "task-3"]);

        assert!(
            plan.subtasks[0].dependencies.is_empty(),
            "la primera tarea no puede depender de nada"
        );
        assert_eq!(plan.subtasks[1].dependencies, vec!["task-1".to_string()]);
        assert_eq!(plan.subtasks[2].dependencies, vec!["task-2".to_string()]);

        for subtask in &plan.subtasks {
            assert_eq!(
                subtask.status,
                SwarmSubTaskStatus::Pending,
                "la subtarea '{}' no debe nacer en otro estado",
                subtask.id
            );
            assert!(!subtask.role.is_empty());
            assert!(!subtask.explanation.is_empty());
            assert!(
                subtask.execution_process_id.is_none()
                    && subtask.workspace_id.is_none()
                    && subtask.started_at.is_none()
                    && subtask.completed_at.is_none(),
                "una subtarea recién planificada no puede declarar ejecución real: '{}'",
                subtask.id
            );
        }
    }

    #[test]
    fn planner_routes_every_subtask_to_a_model_that_exists_in_the_catalog() {
        let catalog = ModelProviderCatalog::default();
        let plan = SwarmPlanner::plan(goal("Documentar la API pública"), &catalog).unwrap();

        for subtask in &plan.subtasks {
            assert!(
                !subtask.recommended_model.is_empty(),
                "la subtarea '{}' no tiene modelo recomendado",
                subtask.id
            );
            assert!(
                catalog
                    .providers
                    .iter()
                    .flat_map(|provider| &provider.routes)
                    .any(|route| route.real_model_id == subtask.recommended_model),
                "la subtarea '{}' apunta al modelo '{}', que no existe en el catálogo",
                subtask.id,
                subtask.recommended_model
            );
            assert!(subtask.estimated_cost_usd >= 0.0);
        }
    }

    #[test]
    fn planner_total_matches_the_sum_of_its_subtask_estimates() {
        let catalog = ModelProviderCatalog::default();
        let plan =
            SwarmPlanner::plan(goal("Implementar el servicio de informes"), &catalog).unwrap();

        let summed: f64 = plan.subtasks.iter().map(|s| s.estimated_cost_usd).sum();
        assert!(
            (plan.total_estimated_cost_usd - summed).abs() < 1e-12,
            "el total del plan ({}) no coincide con la suma de subtareas ({})",
            plan.total_estimated_cost_usd,
            summed
        );
    }

    #[test]
    fn planner_uses_a_free_route_for_the_zero_cost_subtask() {
        let catalog = ModelProviderCatalog::default();
        let plan =
            SwarmPlanner::plan(goal("Implementar el servicio de informes"), &catalog).unwrap();

        let qa = plan.subtasks.last().unwrap();
        assert_eq!(qa.estimated_cost_usd, 0.0, "la tarea de QA debe ser gratuita");
        assert_eq!(qa.recommended_model, "gemini-3.8-flash-high");
        assert!(
            !qa.explanation.contains("FALLBACK"),
            "una ruta gratuita no debería necesitar fallback: {}",
            qa.explanation
        );
    }

    /// Regression guard for review S5: the previous implementation called
    /// `catalog.find_route("gemini-3-8-flash").unwrap()` and could panic the
    /// whole server. A missing catalog entry must surface as an error.
    #[test]
    fn planner_returns_an_error_instead_of_panicking_when_the_catalog_is_empty() {
        let empty_catalog = ModelProviderCatalog {
            providers: vec![],
            work_modes: vec![],
        };

        let err = SwarmPlanner::plan(goal("Implementar algo"), &empty_catalog)
            .expect_err("un catálogo vacío debe ser un error, no un panic");

        assert!(
            err.contains(DEFAULT_FALLBACK_ROUTE_ID),
            "el error debe nombrar la ruta ausente '{DEFAULT_FALLBACK_ROUTE_ID}': {err}"
        );
    }

    /// Exercises the S5 degrade branch that is otherwise unreachable in
    /// production: a work mode missing from the catalog must fall back to the
    /// documented default route instead of aborting the whole plan.
    #[test]
    fn planner_degrades_to_the_default_route_when_a_work_mode_is_missing() {
        let mut catalog = ModelProviderCatalog::default();
        catalog.work_modes.retain(|mode| mode.id == DEFAULT_WORK_MODE_ID);

        let plan = SwarmPlanner::plan(goal("Implementar el servicio de informes"), &catalog)
            .expect("la degradación a la ruta por defecto debe mantener el plan vivo");

        assert_eq!(plan.subtasks.len(), 3);

        for subtask in &plan.subtasks {
            assert_eq!(
                subtask.recommended_model, "gemini-3.8-flash-high",
                "la subtarea '{}' debería haber degradado a la ruta por defecto",
                subtask.id
            );
            assert_eq!(subtask.estimated_cost_usd, 0.0);
            assert!(
                subtask.explanation.contains("Ruta de fallback por defecto"),
                "la explicación debe declarar el motivo de la degradación: {}",
                subtask.explanation
            );
        }
    }

    #[test]
    fn planner_survives_an_impossible_budget_and_says_why() {
        let catalog = ModelProviderCatalog::default();
        let mut impossible = goal("Implementar el servicio de informes");
        impossible.max_budget_usd = Some(0.0);

        let plan = SwarmPlanner::plan(impossible, &catalog)
            .expect("un presupuesto imposible no debe tumbar la planificación");

        assert_eq!(plan.subtasks.len(), 3);

        let planning = &plan.subtasks[0];
        assert!(
            planning.explanation.contains("FALLBACK"),
            "la subtarea de planificación debe declarar el fallback: {}",
            planning.explanation
        );
        assert!(
            (planning.estimated_cost_usd - 0.0875).abs() < 1e-12,
            "el coste reportado debe ser el de la ruta realmente elegida: {}",
            planning.estimated_cost_usd
        );

        let qa = plan.subtasks.last().unwrap();
        assert_eq!(
            qa.estimated_cost_usd, 0.0,
            "una ruta gratuita sigue cabiendo en un presupuesto de cero"
        );
        assert!(!qa.explanation.contains("FALLBACK"));
    }
}
