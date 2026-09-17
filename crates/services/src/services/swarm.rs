use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use executors::catalog::{
    DEFAULT_FALLBACK_ROUTE_ID, DEFAULT_WORK_MODE_ID, ModelProviderCatalog,
    RoutingRecommendation, RoutingRequest,
};
use executors::executors::BaseCodingAgent;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SwarmPlan {
    pub goal: SwarmGoal,
    pub subtasks: Vec<SwarmSubTask>,
    pub total_estimated_cost_usd: f64,
    pub status: SwarmPlanStatus,
    pub created_at: DateTime<Utc>,
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

pub struct SwarmPlanner;

/// In-memory store for swarm plans (review S2/S3).
///
/// The plan produced by `POST /api/swarm/plan` is kept here so that
/// `POST /api/swarm/consolidate/{goal_id}` can consolidate the *real* plan
/// instead of returning fabricated numbers.
///
/// This is deliberately an in-memory store and therefore process-scoped:
/// plans do not survive a server restart. Migrating to a `swarm_plans` table
/// in SQLite is tracked as documented debt in
/// `docs/agentos/05-ESTADO-Y-GATES.md`.
#[derive(Clone, Default)]
pub struct SwarmStore {
    plans: Arc<RwLock<HashMap<Uuid, SwarmPlan>>>,
}

impl SwarmStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a plan, keyed by its goal id. Returns the key it was stored under.
    pub async fn insert(&self, plan: SwarmPlan) -> Uuid {
        let goal_id = plan.goal.goal_id;
        self.plans.write().await.insert(goal_id, plan);
        goal_id
    }

    /// Fetch a plan by goal id.
    pub async fn get(&self, goal_id: &Uuid) -> Option<SwarmPlan> {
        self.plans.read().await.get(goal_id).cloned()
    }

    /// Update the lifecycle status of a stored plan.
    ///
    /// Returns `false` when no plan with that id exists.
    pub async fn set_status(&self, goal_id: &Uuid, status: SwarmPlanStatus) -> bool {
        let mut plans = self.plans.write().await;
        match plans.get_mut(goal_id) {
            Some(plan) => {
                plan.status = status;
                true
            }
            None => false,
        }
    }

    /// All stored plans, oldest first by creation time.
    pub async fn list(&self) -> Vec<SwarmPlan> {
        let mut plans: Vec<SwarmPlan> = self.plans.read().await.values().cloned().collect();
        plans.sort_by_key(|plan| plan.created_at);
        plans
    }

    pub async fn len(&self) -> usize {
        self.plans.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.plans.read().await.is_empty()
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn plan_created_at(created_at: DateTime<Utc>) -> SwarmPlan {
        let mut plan = default_plan("Objetivo de prueba");
        plan.created_at = created_at;
        plan
    }

    // --- SwarmStore (review S2/S3) -----------------------------------------

    #[tokio::test]
    async fn store_roundtrips_a_plan_keyed_by_its_goal_id() {
        let store = SwarmStore::new();
        let plan = default_plan("Implementar un endpoint");
        let goal_id = plan.goal.goal_id;

        assert!(store.is_empty().await);

        assert_eq!(store.insert(plan.clone()).await, goal_id);
        assert_eq!(store.len().await, 1);

        let fetched = store
            .get(&goal_id)
            .await
            .expect("el plan debe estar almacenado");
        assert_eq!(fetched.goal.goal_id, goal_id);
        assert_eq!(fetched.goal.title, plan.goal.title);
        assert_eq!(fetched.subtasks.len(), plan.subtasks.len());
        assert_eq!(fetched.total_estimated_cost_usd, plan.total_estimated_cost_usd);
        assert_eq!(fetched.status, SwarmPlanStatus::Draft);
    }

    #[tokio::test]
    async fn store_returns_none_for_an_unknown_goal() {
        let store = SwarmStore::new();
        store.insert(default_plan("Objetivo almacenado")).await;

        assert!(store.get(&Uuid::new_v4()).await.is_none());
    }

    #[tokio::test]
    async fn store_updates_the_status_of_a_known_plan_only() {
        let store = SwarmStore::new();
        let plan = default_plan("Objetivo aprobable");
        let goal_id = plan.goal.goal_id;
        store.insert(plan).await;

        assert!(store.set_status(&goal_id, SwarmPlanStatus::Approved).await);
        assert_eq!(
            store.get(&goal_id).await.unwrap().status,
            SwarmPlanStatus::Approved
        );

        assert!(
            !store
                .set_status(&Uuid::new_v4(), SwarmPlanStatus::Approved)
                .await,
            "actualizar un plan inexistente debe devolver false, no crearlo"
        );
        assert_eq!(store.len().await, 1);
    }

    /// Guards against a future refactor turning the store into a process-wide
    /// singleton, which would leak swarm plans across deployments.
    #[tokio::test]
    async fn store_instances_do_not_share_state() {
        let first = SwarmStore::new();
        let second = SwarmStore::new();

        first.insert(default_plan("Objetivo del primer store")).await;

        assert_eq!(first.len().await, 1);
        assert!(
            second.is_empty().await,
            "dos instancias de SwarmStore no deben compartir planes"
        );
    }

    #[tokio::test]
    async fn store_lists_plans_oldest_first() {
        let store = SwarmStore::new();
        let now = Utc::now();

        let newest = plan_created_at(now);
        let oldest = plan_created_at(now - chrono::Duration::minutes(10));
        let middle = plan_created_at(now - chrono::Duration::minutes(5));

        let newest_id = newest.goal.goal_id;
        let oldest_id = oldest.goal.goal_id;
        let middle_id = middle.goal.goal_id;

        store.insert(newest).await;
        store.insert(oldest).await;
        store.insert(middle).await;

        let listed: Vec<Uuid> = store
            .list()
            .await
            .iter()
            .map(|plan| plan.goal.goal_id)
            .collect();
        assert_eq!(listed, vec![oldest_id, middle_id, newest_id]);
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
