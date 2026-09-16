use chrono::{DateTime, Utc};
use executors::catalog::{ModelProviderCatalog, RoutingRecommendation, RoutingRequest};
use executors::executors::BaseCodingAgent;
use serde::{Deserialize, Serialize};
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

impl SwarmPlanner {
    pub fn plan(goal: SwarmGoal) -> SwarmPlan {
        let catalog = ModelProviderCatalog::default();
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
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.3),
                ),
                Self::create_subtask(
                    "task-2",
                    "Implementación de refactorización",
                    "Aplicar los cambios estructurales preservando el comportamiento de las interfaces públicas.",
                    "Desarrollador Principal",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.5),
                ),
                Self::create_subtask(
                    "task-3",
                    "Verificación de regresiones y suites de pruebas",
                    "Ejecutar pruebas unitarias y linters para certificar que ningún contrato se ha roto.",
                    "Ingeniero de Calidad (QA)",
                    vec!["task-2".to_string()],
                    "free_only",
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.2),
                ),
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
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.4),
                ),
                Self::create_subtask(
                    "task-2",
                    "Redacción y formateo de documentación",
                    "Elaborar las guías de uso, diagramas conceptuales y ejemplos prácticos en Markdown.",
                    "Escritor Técnico",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.6),
                ),
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
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.35),
                ),
                Self::create_subtask(
                    "task-2",
                    "Implementación de funcionalidad y servicios",
                    &format!("Construir la lógica funcional y endpoints para: {}", goal.title),
                    "Ingeniero de Software",
                    vec!["task-1".to_string()],
                    "quick_execution",
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.45),
                ),
                Self::create_subtask(
                    "task-3",
                    "Pruebas unitarias, tipado y validación E2E",
                    &format!("Implementar pruebas automatizadas y validar compilación para: {}", goal.title),
                    "Ingeniero de QA / Testing",
                    vec!["task-2".to_string()],
                    "free_only",
                    &catalog,
                    goal.max_budget_usd.map(|b| b * 0.20),
                ),
            ]
        };

        let total_cost = subtasks.iter().map(|t| t.estimated_cost_usd).sum();

        SwarmPlan {
            goal,
            subtasks,
            total_estimated_cost_usd: total_cost,
            status: SwarmPlanStatus::Draft,
            created_at: Utc::now(),
        }
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
    ) -> SwarmSubTask {
        let rec = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some(mode.to_string()),
                task_type: Some(title.to_string()),
                max_budget_usd: budget,
                required_capabilities: None,
            })
            .unwrap_or_else(|_| {
                let default_route = catalog.find_route("gemini-3-8-flash").unwrap();
                RoutingRecommendation {
                    selected_route: default_route.clone(),
                    selected_executor: default_route.executor,
                    selected_model_id: default_route.real_model_id,
                    work_mode: catalog.find_work_mode("balanced").unwrap(),
                    estimated_cost_usd: 0.0,
                    explanation: "Ruta de fallback por defecto".to_string(),
                    fallback_chain: vec![],
                }
            });

        SwarmSubTask {
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
        }
    }
}
