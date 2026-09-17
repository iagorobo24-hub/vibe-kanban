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

/// A single model route exposed by a provider account.
///
/// `id` is the stable identifier the rest of AgentOS refers to (for example in
/// `WorkMode::preferred_routes`). `real_model_id` is what the provider actually
/// expects on the wire. They must not lie about each other: a route called
/// "Claude 3.7 Sonnet" that resolves to `claude-sonnet-4-6` is a defect, not an
/// alias.
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
    /// `true` for previous-generation models that are still selectable but are
    /// not the default recommendation for new work. Kept in the catalog so a
    /// `WorkMode` can fall back to them explicitly, never by accident.
    pub is_secondary: bool,
}

/// Shape of the "typical task" used to turn per-million-token prices into a
/// per-task cost estimate. These are deliberately explicit, named constants
/// instead of inline magic numbers: if the assumed task shape changes, the
/// budget filter and the explanation string must change together.
///
/// 15 000 input tokens  -> 15_000 / 1_000_000 = 0.015 M tokens
pub const TYPICAL_TASK_INPUT_M_TOKENS: f64 = 0.015;

/// 500 output tokens -> 500 / 1_000_000 = 0.0005 M tokens
pub const TYPICAL_TASK_OUTPUT_M_TOKENS: f64 = 0.0005;

/// Estimated cost in USD of running one typical task on `route`.
///
/// This is an *estimate for routing decisions only*. It is not billing data;
/// real cost comes from telemetry (Fase 4).
pub fn estimate_task_cost(route: &ModelRoute) -> f64 {
    (route.cost_per_m_in * TYPICAL_TASK_INPUT_M_TOKENS)
        + (route.cost_per_m_out * TYPICAL_TASK_OUTPUT_M_TOKENS)
}

/// Route used when a `WorkMode` cannot satisfy its own constraints and has to
/// fall back. Must always exist in the catalog.
pub const DEFAULT_FALLBACK_ROUTE_ID: &str = "gemini-3-8-flash";

/// Work mode used when no mode is requested and the task type is unknown.
pub const DEFAULT_WORK_MODE_ID: &str = "balanced";

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

impl WorkModeRole {
    /// Parse a `?role=` query value case-insensitively.
    ///
    /// The canonical wire form is SCREAMING_SNAKE_CASE (`PLANNING`), which is
    /// what serde emits and what the generated TypeScript types carry. Using
    /// Rust's `Debug` formatting here produces `Planning` and silently never
    /// matches the query param (review D4), so serde is the single source of
    /// truth instead.
    pub fn from_query(raw: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(raw.trim().to_ascii_uppercase())).ok()
    }

    /// Canonical wire representation, e.g. `PLANNING`.
    pub fn as_wire(&self) -> String {
        serde_json::to_string(self)
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_default()
    }
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
        // Anthropic routes.
        //
        // Model IDs, names and prices verified against Anthropic's published
        // pricing on 2026-09-17. The `id` / `route_name` / `real_model_id`
        // triple must stay consistent: a route whose name says one model and
        // whose `real_model_id` says another is a defect (review C1-C3).
        //
        // Primaries = current generation (Opus 5 / Sonnet 5 / Haiku 4.5).
        // Secondaries = previous generation, still selectable on purpose.
        let anthropic_routes = vec![
            ModelRoute {
                id: "claude-sonnet-5".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Sonnet 5".to_string(),
                real_model_id: "claude-sonnet-5".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Mid,
                speed_class: SpeedClass::Standard,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                    CapabilityTag::Streaming,
                ],
                cost_per_m_in: 2.0,
                cost_per_m_out: 10.0,
                description: "Generación actual: código, agéntica y razonamiento moderado a 1M de contexto. $2/$10 por millón de tokens.".to_string(),
                verified_tool_use: true,
                is_secondary: false,
            },
            ModelRoute {
                id: "claude-opus-5".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Opus 5".to_string(),
                real_model_id: "claude-opus-5".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Premium,
                speed_class: SpeedClass::Slow,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                    CapabilityTag::Streaming,
                ],
                cost_per_m_in: 5.0,
                cost_per_m_out: 25.0,
                description: "Máxima capacidad para síntesis conceptual profunda y decisiones estratégicas. $5/$25 por millón de tokens.".to_string(),
                verified_tool_use: true,
                is_secondary: false,
            },
            ModelRoute {
                id: "claude-haiku-4-5".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Haiku 4.5".to_string(),
                real_model_id: "claude-haiku-4-5".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Cheap,
                speed_class: SpeedClass::Fast,
                capabilities: vec![CapabilityTag::ToolUse, CapabilityTag::Streaming],
                cost_per_m_in: 1.0,
                cost_per_m_out: 5.0,
                description: "Ejecución veloz y económica para consultas acotadas. $1/$5 por millón de tokens.".to_string(),
                verified_tool_use: true,
                is_secondary: false,
            },
            // --- Generación anterior: seleccionables, nunca por defecto ---
            ModelRoute {
                id: "claude-sonnet-4-6".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Sonnet 4.6".to_string(),
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
                description: "Generación anterior. Sigue disponible para reproducir ejecuciones ya registradas en telemetría. $3/$15 por millón de tokens.".to_string(),
                verified_tool_use: true,
                is_secondary: true,
            },
            ModelRoute {
                id: "claude-opus-4-8".to_string(),
                provider_id: "anthropic".to_string(),
                route_name: "Claude Opus 4.8".to_string(),
                real_model_id: "claude-opus-4-8".to_string(),
                executor: BaseCodingAgent::ClaudeCode,
                cost_class: CostClass::Premium,
                speed_class: SpeedClass::Slow,
                capabilities: vec![
                    CapabilityTag::ToolUse,
                    CapabilityTag::LongContext,
                    CapabilityTag::Reasoning,
                ],
                cost_per_m_in: 5.0,
                cost_per_m_out: 25.0,
                description: "Generación anterior de Opus. Mismo precio que Opus 5, sin motivo para elegirla salvo reproducibilidad. $5/$25 por millón de tokens.".to_string(),
                verified_tool_use: true,
                is_secondary: true,
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
                is_secondary: false,
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
                is_secondary: false,
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
                is_secondary: false,
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
                is_secondary: false,
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
            is_secondary: false,
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
                    "claude-opus-5".to_string(),
                    "claude-sonnet-5".to_string(),
                    "gemini-2-5-pro".to_string(),
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
                    "claude-sonnet-5".to_string(),
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
                _ => DEFAULT_WORK_MODE_ID,
            }
        } else {
            DEFAULT_WORK_MODE_ID
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

                // Check budget against the documented "typical task" shape.
                let est_cost = estimate_task_cost(&route);
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

        // D3: the fallback must never be silent. If no candidate survived the
        // filters we still return something usable, but we record exactly why
        // so the explanation stays auditable (ADR-015: routing explicable).
        let mut fallback_reason: Option<String> = None;

        if candidates.is_empty() {
            // Fallback to the first preferred route of the mode anyway.
            if let Some(first_id) = mode.preferred_routes.first() {
                if let Some(route) = self.find_route(first_id) {
                    let est_cost = estimate_task_cost(&route);
                    let budget_limit = req.max_budget_usd.or(mode.max_cost_usd_per_task);

                    let tool_use_note = if route.verified_tool_use {
                        ""
                    } else {
                        " Aviso: la ruta de fallback no tiene tool-use verificado."
                    };

                    fallback_reason = Some(match budget_limit {
                        Some(limit) if est_cost > limit => format!(
                            "FALLBACK: se excedió el presupuesto solicitado (${:.5} USD estimados frente a ${:.5} USD de límite); ninguna ruta del modo cabía en presupuesto.{}",
                            est_cost, limit, tool_use_note
                        ),
                        _ => format!(
                            "FALLBACK: ninguna ruta del modo superó los filtros (capacidades requeridas o verificación de tool-use); se usa la primera ruta preferida.{}",
                            tool_use_note
                        ),
                    });

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
            "Seleccionado '{}' ({}) bajo el modo '{}'. Coste estimado: ${:.5} USD. Justificación: {}. {}{}",
            selected.route_name,
            selected.executor,
            mode.name,
            estimated_cost,
            selected.description,
            fallback_reason
                .as_deref()
                .map(|reason| format!("{} ", reason))
                .unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Normalised version signature of an identifier: every run of digits,
    /// joined with `-`. `4.6`, `4-6` and `4_6` all normalise to `4-6`, so a
    /// route name and a wire model id can be compared regardless of the
    /// punctuation the provider happens to use.
    fn version_signature(raw: &str) -> Option<String> {
        let mut runs: Vec<String> = Vec::new();
        let mut current = String::new();
        for ch in raw.chars() {
            if ch.is_ascii_digit() {
                current.push(ch);
            } else if !current.is_empty() {
                runs.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }
        (!runs.is_empty()).then(|| runs.join("-"))
    }

    /// Regression guard for review C1-C3.
    ///
    /// The defect was a route advertised as "Claude 3.7 Sonnet" whose
    /// `real_model_id` resolved to `claude-sonnet-4-6`: the route lied about
    /// which model would actually be invoked, so routing decisions and cost
    /// estimates were built on a false premise. Whenever both sides carry a
    /// version number they must agree.
    #[test]
    fn route_identity_never_lies_about_the_model_version() {
        let catalog = ModelProviderCatalog::default();
        let mut comparisons = 0usize;

        for provider in &catalog.providers {
            for route in &provider.routes {
                let Some(real) = version_signature(&route.real_model_id) else {
                    continue;
                };

                if let Some(from_id) = version_signature(&route.id) {
                    assert_eq!(
                        from_id, real,
                        "la ruta '{}' se identifica como '{}' pero su real_model_id es '{}'",
                        route.id, route.id, route.real_model_id
                    );
                    comparisons += 1;
                }

                if let Some(from_name) = version_signature(&route.route_name) {
                    assert_eq!(
                        from_name, real,
                        "la ruta '{}' se anuncia como '{}' pero su real_model_id es '{}'",
                        route.id, route.route_name, route.real_model_id
                    );
                    comparisons += 1;
                }
            }
        }

        // Guards against the loop silently degenerating into a no-op.
        assert!(
            comparisons >= 12,
            "la comprobación de identidad solo cubrió {comparisons} comparaciones"
        );
    }

    /// The obsolete routes the review flagged must not come back.
    #[test]
    fn the_obsolete_versionless_anthropic_routes_are_gone() {
        let catalog = ModelProviderCatalog::default();
        for id in ["claude-3-7-sonnet", "claude-3-5-sonnet", "opus", "haiku"] {
            assert!(
                catalog.find_route(id).is_none(),
                "la ruta obsoleta '{id}' sigue presente en el catálogo"
            );
        }
    }

    /// Regression guard for review C1-C3, pinned literally: current generation
    /// is primary, the previous generation survives only as `is_secondary`.
    #[test]
    fn anthropic_current_generation_is_primary_and_previous_is_secondary() {
        let catalog = ModelProviderCatalog::default();

        let expected = [
            ("claude-sonnet-5", "claude-sonnet-5", 2.0, 10.0, false),
            ("claude-opus-5", "claude-opus-5", 5.0, 25.0, false),
            ("claude-haiku-4-5", "claude-haiku-4-5", 1.0, 5.0, false),
            ("claude-sonnet-4-6", "claude-sonnet-4-6", 3.0, 15.0, true),
            ("claude-opus-4-8", "claude-opus-4-8", 5.0, 25.0, true),
        ];

        for (id, real_model_id, cost_in, cost_out, is_secondary) in expected {
            let route = catalog
                .find_route(id)
                .unwrap_or_else(|| panic!("falta la ruta '{id}' en el catálogo"));

            assert_eq!(route.provider_id, "anthropic", "proveedor de '{id}'");
            assert_eq!(route.real_model_id, real_model_id, "real_model_id de '{id}'");
            assert_eq!(route.cost_per_m_in, cost_in, "coste de entrada de '{id}'");
            assert_eq!(route.cost_per_m_out, cost_out, "coste de salida de '{id}'");
            assert_eq!(route.is_secondary, is_secondary, "is_secondary de '{id}'");
        }
    }

    #[test]
    fn provider_route_and_mode_ids_are_unique_and_consistent() {
        let catalog = ModelProviderCatalog::default();

        let mut provider_ids = HashSet::new();
        let mut route_ids = HashSet::new();

        for provider in &catalog.providers {
            assert!(
                provider_ids.insert(provider.id.clone()),
                "proveedor duplicado: {}",
                provider.id
            );

            for route in &provider.routes {
                assert!(
                    route_ids.insert(route.id.clone()),
                    "ruta duplicada: {}",
                    route.id
                );
                assert_eq!(
                    route.provider_id, provider.id,
                    "la ruta '{}' declara el proveedor '{}' pero está anidada en '{}'",
                    route.id, route.provider_id, provider.id
                );
            }
        }

        let mut mode_ids = HashSet::new();
        for mode in &catalog.work_modes {
            assert!(
                mode_ids.insert(mode.id.clone()),
                "modo de trabajo duplicado: {}",
                mode.id
            );
        }
    }

    #[test]
    fn every_work_mode_only_references_routes_that_exist() {
        let catalog = ModelProviderCatalog::default();
        for mode in &catalog.work_modes {
            assert!(
                !mode.preferred_routes.is_empty(),
                "el modo '{}' no tiene rutas preferidas",
                mode.id
            );
            for route_id in &mode.preferred_routes {
                assert!(
                    catalog.find_route(route_id).is_some(),
                    "el modo '{}' referencia la ruta inexistente '{}'",
                    mode.id,
                    route_id
                );
            }
        }
    }

    #[test]
    fn no_work_mode_defaults_to_a_previous_generation_route() {
        let catalog = ModelProviderCatalog::default();
        for mode in &catalog.work_modes {
            let first = mode
                .preferred_routes
                .first()
                .expect("la no-vacuidad ya está validada por otro test");
            let route = catalog
                .find_route(first)
                .expect("la existencia ya está validada por otro test");
            assert!(
                !route.is_secondary,
                "el modo '{}' arranca con la ruta de generación anterior '{}'",
                mode.id,
                first
            );
        }
    }

    /// Contract relied on by `SwarmPlanner::create_subtask` (review S5): the
    /// documented defaults must always resolve.
    #[test]
    fn documented_defaults_exist_in_the_catalog() {
        let catalog = ModelProviderCatalog::default();
        assert!(
            catalog.find_route(DEFAULT_FALLBACK_ROUTE_ID).is_some(),
            "falta la ruta de fallback por defecto '{DEFAULT_FALLBACK_ROUTE_ID}'"
        );
        assert!(
            catalog.find_work_mode(DEFAULT_WORK_MODE_ID).is_some(),
            "falta el modo de trabajo por defecto '{DEFAULT_WORK_MODE_ID}'"
        );
    }

    /// Regression guard for review D2: the budget filter used to be built on
    /// two inline magic numbers. They are now named constants with a documented
    /// task shape, and this test pins that shape so it cannot drift silently.
    #[test]
    fn task_cost_estimate_uses_the_documented_task_shape() {
        assert_eq!(TYPICAL_TASK_INPUT_M_TOKENS, 15_000.0 / 1_000_000.0);
        assert_eq!(TYPICAL_TASK_OUTPUT_M_TOKENS, 500.0 / 1_000_000.0);

        let catalog = ModelProviderCatalog::default();

        let opus = catalog.find_route("claude-opus-5").unwrap();
        let opus_estimate = estimate_task_cost(&opus);
        assert!(
            (opus_estimate - 0.0875).abs() < 1e-12,
            "estimación de Opus 5 inesperada: {opus_estimate}"
        );

        let free = catalog.find_route("gemini-3-8-flash").unwrap();
        assert_eq!(estimate_task_cost(&free), 0.0);
    }

    #[test]
    fn routes_marked_free_actually_cost_nothing() {
        let catalog = ModelProviderCatalog::default();
        for provider in &catalog.providers {
            for route in &provider.routes {
                assert!(
                    route.cost_per_m_in >= 0.0 && route.cost_per_m_out >= 0.0,
                    "la ruta '{}' tiene precios negativos",
                    route.id
                );
                if route.cost_class == CostClass::Free {
                    assert_eq!(
                        route.cost_per_m_in, 0.0,
                        "la ruta '{}' es FREE pero cobra entrada",
                        route.id
                    );
                    assert_eq!(
                        route.cost_per_m_out, 0.0,
                        "la ruta '{}' es FREE pero cobra salida",
                        route.id
                    );
                }
            }
        }
    }

    /// Regression guard for review D4.
    ///
    /// The `?role=` filter compared the raw query value against Rust's `Debug`
    /// output, which yields `Planning`, so `?role=PLANNING` matched nothing and
    /// the endpoint silently returned an empty list. serde is now the single
    /// source of truth for the wire format.
    #[test]
    fn work_mode_role_query_parsing_uses_the_wire_format() {
        assert_eq!(WorkModeRole::Planning.as_wire(), "PLANNING");
        assert_eq!(WorkModeRole::Execution.as_wire(), "EXECUTION");
        assert_eq!(WorkModeRole::Hybrid.as_wire(), "HYBRID");

        for raw in ["PLANNING", "planning", "Planning", "  planning  "] {
            assert_eq!(
                WorkModeRole::from_query(raw),
                Some(WorkModeRole::Planning),
                "no se pudo interpretar el rol '{raw}'"
            );
        }
        assert_eq!(
            WorkModeRole::from_query("execution"),
            Some(WorkModeRole::Execution)
        );
        assert_eq!(
            WorkModeRole::from_query("HYBRID"),
            Some(WorkModeRole::Hybrid)
        );

        for raw in ["", "bogus", "planning_mode", "plan"] {
            assert_eq!(
                WorkModeRole::from_query(raw),
                None,
                "el valor inválido '{raw}' no debería interpretarse"
            );
        }
    }

    /// Regression guard for review D3: when nothing fits the budget the
    /// fallback must be explicit in the explanation, never silent.
    #[test]
    fn recommend_never_falls_back_silently_on_budget() {
        let catalog = ModelProviderCatalog::default();

        let rec = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some("planning".to_string()),
                task_type: None,
                max_budget_usd: Some(0.0),
                required_capabilities: None,
            })
            .expect("el modo planning debe poder degradar a su primera ruta");

        assert!(
            rec.explanation.contains("FALLBACK"),
            "la explicación debe declarar el fallback: {}",
            rec.explanation
        );
        assert!(
            rec.explanation.contains("presupuesto"),
            "la explicación debe decir que se excedió el presupuesto: {}",
            rec.explanation
        );
        assert_eq!(
            rec.selected_route.id, "claude-opus-5",
            "el fallback debe usar la primera ruta preferida del modo"
        );
        assert_eq!(rec.selected_model_id, rec.selected_route.real_model_id);
    }

    #[test]
    fn recommend_does_not_announce_a_fallback_when_a_candidate_fits() {
        let catalog = ModelProviderCatalog::default();

        let rec = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some("balanced".to_string()),
                task_type: None,
                max_budget_usd: Some(0.01),
                required_capabilities: None,
            })
            .expect("el modo balanced debe encontrar una ruta gratuita");

        assert!(
            !rec.explanation.contains("FALLBACK"),
            "no debería anunciarse un fallback: {}",
            rec.explanation
        );
        assert_eq!(rec.selected_route.id, "gemini-3-8-flash");
        assert_eq!(rec.estimated_cost_usd, 0.0);
        assert_eq!(rec.selected_executor, rec.selected_route.executor);
        assert!(
            !rec.fallback_chain.is_empty(),
            "debe listar las alternativas que sí cabían en el presupuesto"
        );
    }

    #[test]
    fn recommend_honours_required_capabilities() {
        let catalog = ModelProviderCatalog::default();

        let rec = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some("balanced".to_string()),
                task_type: None,
                max_budget_usd: None,
                required_capabilities: Some(vec![CapabilityTag::Vision]),
            })
            .expect("debe existir una ruta con visión en el modo balanced");

        assert!(
            rec.selected_route.capabilities.contains(&CapabilityTag::Vision),
            "se seleccionó una ruta sin la capacidad exigida: {}",
            rec.selected_route.id
        );
        assert_eq!(rec.selected_route.id, "gemini-3-8-flash");
    }

    #[test]
    fn recommend_rejects_an_unknown_work_mode() {
        let catalog = ModelProviderCatalog::default();

        let err = catalog
            .recommend(&RoutingRequest {
                work_mode_id: Some("no_existe".to_string()),
                task_type: None,
                max_budget_usd: None,
                required_capabilities: None,
            })
            .expect_err("un modo inexistente debe ser un error, no un fallback");

        assert!(
            err.contains("no_existe"),
            "el error debe nombrar el modo solicitado: {err}"
        );
    }

    #[test]
    fn task_type_is_mapped_to_a_work_mode() {
        let catalog = ModelProviderCatalog::default();

        for (task_type, expected_mode) in [
            ("architecture", "planning"),
            ("review", "planning"),
            ("refactor", "quick_execution"),
            ("script", "quick_execution"),
            ("free", "free_only"),
            ("algo_desconocido", DEFAULT_WORK_MODE_ID),
        ] {
            let rec = catalog
                .recommend(&RoutingRequest {
                    work_mode_id: None,
                    task_type: Some(task_type.to_string()),
                    max_budget_usd: None,
                    required_capabilities: None,
                })
                .unwrap_or_else(|err| panic!("task_type '{task_type}' no enrutó: {err}"));

            assert_eq!(rec.work_mode.id, expected_mode, "task_type '{task_type}'");
        }
    }
}
