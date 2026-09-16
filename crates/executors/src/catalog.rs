use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::executors::BaseCodingAgent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelProviderKind {
    Anthropic,
    OpenaiCompatible,
    AnthropicCompatibleGateway,
    NvidiaNim,
    OpenCodeZen,
    FreeAggregator,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CostClass {
    Free,
    Cheap,
    Mid,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpeedClass {
    Fast,
    Standard,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityTag {
    ToolUse,
    LongContext,
    Vision,
    Streaming,
    Reasoning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ModelRoute {
    pub id: String,
    pub provider_id: String,
    pub route_name: String,
    pub real_model_id: String,
    pub executor: BaseCodingAgent,
    pub cost_class: CostClass,
    pub speed_class: SpeedClass,
    pub capabilities: Vec<CapabilityTag>,
    pub cost_per_m_in: f64,
    pub cost_per_m_out: f64,
    pub description: String,
    pub verified_tool_use: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ProviderAccount {
    pub id: String,
    pub provider_kind: ModelProviderKind,
    pub display_name: String,
    pub base_url: Option<String>,
    pub auth_method: String,
    pub status: String,
    pub cost_class: CostClass,
    pub routes: Vec<ModelRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkModeRole {
    Planning,
    Execution,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct WorkMode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub role: WorkModeRole,
    pub preferred_routes: Vec<String>,
    pub max_cost_usd_per_task: Option<f64>,
    pub fallback_reasons_allowed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct RoutingRequest {
    pub work_mode_id: Option<String>,
    pub task_type: Option<String>,
    pub max_budget_usd: Option<f64>,
    pub required_capabilities: Option<Vec<CapabilityTag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct RoutingRecommendation {
    pub selected_route: ModelRoute,
    pub selected_executor: BaseCodingAgent,
    pub selected_model_id: String,
    pub work_mode: WorkMode,
    pub estimated_cost_usd: f64,
    pub explanation: String,
    pub fallback_chain: Vec<ModelRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModelProviderCatalog {
    pub providers: Vec<ProviderAccount>,
    pub work_modes: Vec<WorkMode>,
}

impl Default for ModelProviderCatalog {
    fn default() -> Self {
        let anthropic_routes = vec![
            ModelRoute {
                id: "claude-3-7-sonnet".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude 3.7 Sonnet".to_string(),
                real_model_id: "claude-sonnet-4-6".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Mid,
                speed_class: SpeedClass::Standard,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                    CapabilityTag::Streaming,
                ],
                cost_per_m_in: 3.0,
                cost_per_m_out: 15.0,
                description: "Modelo insignia para arquitectura, código complejo y razonamiento adaptativo.".to_string(),
                verified_tool_use: true,
            },
            ModelRoute {
                id: "claude-opus-4-7".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Opus".to_string(),
                real_model_id: "opus".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Premium,
                speed_class: SpeedClass::Slow,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                ],
                cost_per_m_in: 15.0,
                cost_per_m_out: 75.0,
                description: "Máxima capacidad para síntesis conceptual profunda y decisiones estratégicas.".to_string(),
                verified_tool_use: true,
            },
            ModelRoute {
                id: "claude-haiku-4-5".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Haiku".to_string(),
                real_model_id: "haiku".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Cheap,
                speed_class: SpeedClass::Fast,
                capabilities: vec![CapabilityTag::ToolUse, CapabilityTag::Streaming],
                cost_per_m_in: 0.8,
                cost_per_m_out: 4.0,
                description: "Ejecución veloz y económica para consultas acotadas.".to_string(),
                verified_tool_use: true,
            },
        ];

        let antigravity_routes = vec![
            ModelRoute {
                id: "gemini-3-8-flash".to_string(),
                provider_id: "google_antigravity".to_string(),
                route_name: "Gemini 3.8 Flash (Antigravity)".to_string(),
                real_model_id: "gemini-3.8-flash-high".to_string(),
                executor: BaseCodingAgent::Antigravity,
                cost_class: CostClass::Free,
                speed_class: SpeedClass::Fast,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Vision,
                    CapabilityTag::Streaming,
                ],
                cost_per_m_in: 0.0,
                cost_per_m_out: 0.0,
                description: "Motor nativo stream-json de Google Antigravity con coste cero y alta velocidad.".to_string(),
                verified_tool_use: true,
            },
            ModelRoute {
                id: "gemini-2-5-pro".to_string(),
                provider_id: "google_antigravity".to_string(),
                route_name: "Gemini 2.5 Pro".to_string(),
                real_model_id: "gemini-2.5-pro".to_string(),
                executor: BaseCodingAgent::Antigravity,
                cost_class: CostClass::Mid,
                speed_class: SpeedClass::Standard,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                ],
                cost_per_m_in: 1.25,
                cost_per_m_out: 5.0,
                description: "Gran ventana de contexto y alta precisión multimodal.".to_string(),
                verified_tool_use: true,
            },
        ];

        let opencode_routes = vec![
            ModelRoute {
                id: "opencode-mimo-free".to_string(),
                provider_id: "opencode_zen".to_string(),
                route_name: "OpenCode Zen MIMO Free".to_string(),
                real_model_id: "opencode/mimo-v2.5-free".to_string(),
                executor: BaseCodingAgent::Opencode,
                cost_class: CostClass::Free,
                speed_class: SpeedClass::Fast,
                capabilities: vec![CapabilityTag::ToolUse, CapabilityTag::Streaming],
                cost_per_m_in: 0.0,
                cost_per_m_out: 0.0,
                description: "Inferencia gratuita comunitaria a través de OpenCode Zen.".to_string(),
                verified_tool_use: true,
            },
            ModelRoute {
                id: "opencode-big-pickle".to_string(),
                provider_id: "opencode_zen".to_string(),
                route_name: "OpenCode Big Pickle".to_string(),
                real_model_id: "opencode/big-pickle".to_string(),
                executor: BaseCodingAgent::Opencode,
                cost_class: CostClass::Free,
                speed_class: SpeedClass::Fast,
                capabilities: vec![CapabilityTag::ToolUse],
                cost_per_m_in: 0.0,
                cost_per_m_out: 0.0,
                description: "Ruta de respaldo sin coste en OpenCode Zen.".to_string(),
                verified_tool_use: true,
            },
        ];

        let deepseek_routes = vec![ModelRoute {
            id: "deepseek-chat".to_string(),
            provider_id: "deepseek_gateway".to_string(),
            route_name: "DeepSeek V3".to_string(),
            real_model_id: "deepseek-chat".to_string(),
            executor: BaseCodingAgent::Opencode,
            cost_class: CostClass::Cheap,
            speed_class: SpeedClass::Fast,
            capabilities: vec![
                CapabilityTag::ToolUse,
                CapabilityTag::Reasoning,
                CapabilityTag::Streaming,
            ],
            cost_per_m_in: 0.27,
            cost_per_m_out: 1.10,
            description: "Capacidad sobresaliente de código a una fracción del coste estándar.".to_string(),
            verified_tool_use: true,
        }];

        let providers = vec![
            ProviderAccount {
                id: "anthropic".to_string(),
                provider_kind: ModelProviderKind::Anthropic,
                display_name: "Anthropic Direct (Claude Code)".to_string(),
                base_url: None,
                auth_method: "cli_session".to_string(),
                status: "active".to_string(),
                cost_class: CostClass::Mid,
                routes: anthropic_routes,
            },
            ProviderAccount {
                id: "google_antigravity".to_string(),
                provider_kind: ModelProviderKind::Custom,
                display_name: "Google Antigravity Nativo".to_string(),
                base_url: None,
                auth_method: "agy_cli".to_string(),
                status: "active".to_string(),
                cost_class: CostClass::Free,
                routes: antigravity_routes,
            },
            ProviderAccount {
                id: "opencode_zen".to_string(),
                provider_kind: ModelProviderKind::OpenCodeZen,
                display_name: "OpenCode Zen Gateway".to_string(),
                base_url: Some("https://opencode.ai/zen/v1".to_string()),
                auth_method: "anonymous_zen".to_string(),
                status: "active".to_string(),
                cost_class: CostClass::Free,
                routes: opencode_routes,
            },
            ProviderAccount {
                id: "deepseek_gateway".to_string(),
                provider_kind: ModelProviderKind::OpenaiCompatible,
                display_name: "DeepSeek / NIM Gateway".to_string(),
                base_url: Some("https://api.deepseek.com/v1".to_string()),
                auth_method: "api_key".to_string(),
                status: "configured".to_string(),
                cost_class: CostClass::Cheap,
                routes: deepseek_routes,
            },
        ];

        let work_modes = vec![
            WorkMode {
                id: "planning".to_string(),
                name: "Planificación y Arquitectura".to_string(),
                description: "Enruta a modelos de razonamiento avanzado y alto contexto para análisis, diseño y arquitectura.".to_string(),
                role: WorkModeRole::Planning,
                preferred_routes: vec![
                    "claude-3-7-sonnet".to_string(),
                    "gemini-2-5-pro".to_string(),
                    "claude-opus-4-7".to_string(),
                ],
                max_cost_usd_per_task: Some(0.50),
                fallback_reasons_allowed: vec![
                    "rate_limited".to_string(),
                    "quota_exhausted".to_string(),
                    "error".to_string(),
                ],
            },
            WorkMode {
                id: "quick_execution".to_string(),
                name: "Ejecución Rápida y Económica".to_string(),
                description: "Enruta a modelos ultra-rápidos de bajo coste para tareas iterativas, refactors simples y scripts.".to_string(),
                role: WorkModeRole::Execution,
                preferred_routes: vec![
                    "gemini-3-8-flash".to_string(),
                    "claude-haiku-4-5".to_string(),
                    "opencode-mimo-free".to_string(),
                ],
                max_cost_usd_per_task: Some(0.05),
                fallback_reasons_allowed: vec![
                    "rate_limited".to_string(),
                    "budget_exceeded".to_string(),
                    "error".to_string(),
                ],
            },
            WorkMode {
                id: "free_only".to_string(),
                name: "Ejecución Gratuita (Coste Cero)".to_string(),
                description: "Usa exclusivamente proveedores y modelos gratuitos verificados sin ningún consumo de saldo.".to_string(),
                role: WorkModeRole::Execution,
                preferred_routes: vec![
                    "gemini-3-8-flash".to_string(),
                    "opencode-mimo-free".to_string(),
                    "opencode-big-pickle".to_string(),
                ],
                max_cost_usd_per_task: Some(0.00),
                fallback_reasons_allowed: vec![
                    "rate_limited".to_string(),
                    "error".to_string(),
                ],
            },
            WorkMode {
                id: "balanced".to_string(),
                name: "Equilibrado (Estándar)".to_string(),
                description: "Modo por defecto: balance entre velocidad y precisión, con fallback transparente.".to_string(),
                role: WorkModeRole::Hybrid,
                preferred_routes: vec![
                    "gemini-3-8-flash".to_string(),
                    "claude-3-7-sonnet".to_string(),
                    "opencode-mimo-free".to_string(),
                ],
                max_cost_usd_per_task: Some(0.20),
                fallback_reasons_allowed: vec![
                    "rate_limited".to_string(),
                    "budget_exceeded".to_string(),
                    "error".to_string(),
                ],
            },
        ];

        Self {
            providers,
            work_modes,
        }
    }
}

impl ModelProviderCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_route(&self, route_id: &str) -> Option<ModelRoute> {
        for provider in &self.providers {
            for route in &provider.routes {
                if route.id == route_id {
                    return Some(route.clone());
                }
            }
        }
        None
    }

    pub fn find_work_mode(&self, mode_id: &str) -> Option<WorkMode> {
        self.work_modes.iter().find(|m| m.id == mode_id).cloned()
    }

    pub fn recommend(&self, req: &RoutingRequest) -> Result<RoutingRecommendation, String> {
        // 1. Resolve work mode
        let target_mode_id = if let Some(mode_id) = &req.work_mode_id {
            mode_id.as_str()
        } else if let Some(task_type) = &req.task_type {
            match task_type.to_lowercase().as_str() {
                "planning" | "architecture" | "design" | "review" => "planning",
                "free" | "zero_cost" => "free_only",
                "refactor" | "tests" | "quick_fix" | "script" => "quick_execution",
                _ => "balanced",
            }
        } else {
            "balanced"
        };

        let mode = self
            .find_work_mode(target_mode_id)
            .ok_or_else(|| format!("Modo de trabajo desconocido: '{}'", target_mode_id))?;

        // 2. Filter matching routes in priority order
        let mut candidates = Vec::new();
        for route_id in &mode.preferred_routes {
            if let Some(route) = self.find_route(route_id) {
                // Must have tool_use verified
                if !route.verified_tool_use {
                    continue;
                }

                // Check capabilities
                if let Some(required_caps) = &req.required_capabilities {
                    let has_all = required_caps
                        .iter()
                        .all(|cap| route.capabilities.contains(cap));
                    if !has_all {
                        continue;
                    }
                }

                // Check budget: estimated task cost (typical task ~15k in, 500 out)
                let est_cost = (route.cost_per_m_in * 0.015) + (route.cost_per_m_out * 0.0005);
                if let Some(max_budget) = req.max_budget_usd {
                    if est_cost > max_budget {
                        continue;
                    }
                } else if let Some(mode_budget) = mode.max_cost_usd_per_task {
                    if est_cost > mode_budget {
                        continue;
                    }
                }

                candidates.push((route, est_cost));
            }
        }

        if candidates.is_empty() {
            // Fallback to the first preferred route of the mode anyway
            if let Some(first_id) = mode.preferred_routes.first() {
                if let Some(route) = self.find_route(first_id) {
                    let est_cost = (route.cost_per_m_in * 0.015) + (route.cost_per_m_out * 0.0005);
                    candidates.push((route, est_cost));
                }
            }
        }

        if candidates.is_empty() {
            return Err("No se encontraron rutas de modelo viables en el catálogo.".to_string());
        }

        let (selected, estimated_cost) = candidates.remove(0);
        let fallback_chain: Vec<ModelRoute> = candidates.into_iter().map(|(r, _)| r).collect();

        // 3. Build transparent and auditable explanation
        let explanation = format!(
            "Seleccionado '{}' ({}) bajo el modo '{}'. Coste estimado: ${:.5} USD. Justificación: {}. {}",
            selected.route_name,
            selected.executor,
            mode.name,
            estimated_cost,
            selected.description,
            if !fallback_chain.is_empty() {
                format!(
                    "Cadena de fallback configurada: {}.",
                    fallback_chain
                        .iter()
                        .map(|r| format!("{} ({})", r.route_name, r.executor))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                )
            } else {
                "Sin rutas alternativas en el presupuesto actual.".to_string()
            }
        );

        Ok(RoutingRecommendation {
            selected_route: selected.clone(),
            selected_executor: selected.executor.clone(),
            selected_model_id: selected.real_model_id,
            work_mode: mode,
            estimated_cost_usd: estimated_cost,
            explanation,
            fallback_chain,
        })
    }
}
