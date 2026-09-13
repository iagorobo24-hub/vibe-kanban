//! Ejecutor para Google Antigravity CLI (`agy`).
//!
//! `agy` no habla ACP de forma nativa — el feature request oficial
//! (google-antigravity/antigravity-cli#31) sigue abierto, y `agy 1.2.2` no tiene
//! subcomando `acp`. Usamos el adaptador `agy-acp` (Apache-2.0,
//! https://github.com/shindgew/agy-acp), que envuelve el binario `agy` instalado y
//! expone ACP por stdio, de modo que reutilizamos el harness ACP compartido igual
//! que Gemini, Qwen y Copilot.
//!
//! Esto además esquiva el problema conocido de `agy -p`, que escribe la respuesta
//! al terminal controlador en vez de a stdout: el adaptador no usa el modo print.
//!
//! ## Por qué NO el servidor ACP oficial de Google
//!
//! El registro ACP (`agentclientprotocol/registry`) publica `agy_acp_server.exe`,
//! firmado por Google LLC. Lo probamos hablándole ACP por stdio y descartamos
//! migrar, por dos motivos medidos:
//!
//! 1. **Sólo expone modelos Gemini** (11). Pierde `claude-opus-4-6-thinking`,
//!    `claude-sonnet-4-6`, `gpt-oss-120b-medium` y `gemini-3.1-pro-high`, que sí
//!    da el adaptador vía `agy` (14 modelos). Perder Claude y GPT-OSS elimina el
//!    arbitraje entre proveedores dentro de una sola suscripción.
//! 2. **Autenticación propia e independiente.** Ignora `~/.gemini/oauth_creds.json`
//!    de `agy` y exige `authenticate` por ACP o `auth.type` en
//!    `~/.gemini/antigravity-acp/settings.json`. El harness compartido nunca llama a
//!    `authenticate` (descarta el resultado de `initialize`), así que serían dos
//!    logins que mantener y un cambio en el harness común.
//!
//! A favor del oficial, para cuando se reconsidere: anuncia `mcpCapabilities`
//! `{http: true, sse: true}` frente a `{http: false, sse: false}` del adaptador, y
//! pesa 562 MB en disco frente a 0 del adaptador, que reutiliza el `agy` ya instalado.
//!
//! ## Versión fijada
//!
//! `agy-acp` va clavado a una versión igual que el resto de ejecutores npm del repo
//! (`@google/gemini-cli@0.29.3`, `opencode-ai@1.4.7`, `@github/copilot@0.0.403`).
//! Sin versión, `npx -y` resuelve `latest` en **cada arranque de agente**, lo que
//! ejecutaría cualquier publicación futura del paquete sin revisarla.

use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

pub use super::acp::AcpAgentHarness;
use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor,
    },
    logs::utils::patch,
    model_selector::{ModelInfo, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

/// Ruido de arranque del adaptador que no aporta nada al log del usuario.
const SUPPRESSED_STDERR_PATTERNS: &[&str] =
    &["logging before google.Init", "Fetching available models..."];

#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct Antigravity {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
    #[serde(skip)]
    #[ts(skip)]
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub approvals: Option<Arc<dyn ExecutorApprovalService>>,
}

impl Antigravity {
    fn build_command_builder(&self) -> Result<CommandBuilder, CommandBuildError> {
        // El adaptador detecta el binario `agy` instalado; si no existe, lo instala
        // desde los releases oficiales durante `initialize`.
        let mut builder = CommandBuilder::new("npx -y agy-acp@0.5.2");

        // agy-acp mapea su opción `model` al flag `--model` de agy.
        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        apply_overrides(builder, &self.cmd)
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for Antigravity {
    fn apply_overrides(&mut self, executor_config: &ExecutorConfig) {
        if let Some(model_id) = &executor_config.model_id {
            self.model = Some(model_id.clone());
        }
    }

    fn use_approvals(&mut self, approvals: Arc<dyn ExecutorApprovalService>) {
        self.approvals = Some(approvals);
    }

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let harness = AcpAgentHarness::new();
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let agy_command = self.build_command_builder()?.build_initial()?;
        harness
            .spawn_with_command(
                current_dir,
                combined_prompt,
                agy_command,
                env,
                &self.cmd,
                self.approvals.clone(),
            )
            .await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        _reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let harness = AcpAgentHarness::new();
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let agy_command = self.build_command_builder()?.build_follow_up(&[])?;
        harness
            .spawn_follow_up_with_command(
                current_dir,
                combined_prompt,
                session_id,
                agy_command,
                env,
                &self.cmd,
                self.approvals.clone(),
            )
            .await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        super::acp::normalize_logs_with_suppressed_stderr_patterns(
            msg_store,
            worktree_path,
            SUPPRESSED_STDERR_PATTERNS,
        )
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        // agy hereda el árbol de configuración de Gemini CLI.
        dirs::home_dir().map(|home| home.join(".gemini").join("config").join("mcp_config.json"))
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        if let Some(timestamp) = dirs::home_dir()
            .and_then(|home| std::fs::metadata(home.join(".gemini").join("oauth_creds.json")).ok())
            .and_then(|m| m.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
        {
            return AvailabilityInfo::LoginDetected {
                last_auth_timestamp: timestamp,
            };
        }

        let mcp_config_found = self
            .default_mcp_config_path()
            .map(|p| p.exists())
            .unwrap_or(false);

        let installation_indicator_found = dirs::home_dir()
            .map(|home| home.join(".gemini").join("installation_id").exists())
            .unwrap_or(false);

        if mcp_config_found || installation_indicator_found {
            AvailabilityInfo::InstallationFound
        } else {
            AvailabilityInfo::NotFound
        }
    }

    fn get_preset_options(&self) -> ExecutorConfig {
        ExecutorConfig {
            executor: BaseCodingAgent::Antigravity,
            variant: None,
            model_id: self.model.clone(),
            agent_id: None,
            reasoning_id: None,
            // Las aprobaciones van por ACP a través del harness, no por flags de agy.
            permission_policy: Some(PermissionPolicy::Supervised),
        }
    }

    async fn discover_options(
        &self,
        _workdir: Option<&std::path::Path>,
        _repo_path: Option<&std::path::Path>,
    ) -> Result<futures::stream::BoxStream<'static, json_patch::Patch>, ExecutorError> {
        // Los 14 modelos que expone `agy models` (CLI 1.2.2), verificados ejecutándolo.
        let options = ExecutorDiscoveredOptions {
            model_selector: ModelSelectorConfig {
                models: vec![
                    ModelInfo {
                        id: "gemini-3.8-flash-high".to_string(),
                        name: "Gemini 3.8 Flash (High)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.8-flash-medium".to_string(),
                        name: "Gemini 3.8 Flash (Medium)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.8-flash-low".to_string(),
                        name: "Gemini 3.8 Flash (Low)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.7-flash-high".to_string(),
                        name: "Gemini 3.7 Flash (High)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.7-flash-medium".to_string(),
                        name: "Gemini 3.7 Flash (Medium)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.7-flash-low".to_string(),
                        name: "Gemini 3.7 Flash (Low)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.6-flash-high".to_string(),
                        name: "Gemini 3.6 Flash (High)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.6-flash-medium".to_string(),
                        name: "Gemini 3.6 Flash (Medium)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.6-flash-low".to_string(),
                        name: "Gemini 3.6 Flash (Low)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.1-pro-high".to_string(),
                        name: "Gemini 3.1 Pro (High)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gemini-3.1-pro-low".to_string(),
                        name: "Gemini 3.1 Pro (Low)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "claude-sonnet-4-6".to_string(),
                        name: "Claude Sonnet 4.6 (Thinking)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "claude-opus-4-6-thinking".to_string(),
                        name: "Claude Opus 4.6 (Thinking)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                    ModelInfo {
                        id: "gpt-oss-120b-medium".to_string(),
                        name: "GPT-OSS 120B (Medium)".to_string(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                ],
                default_model: Some("gemini-3.1-pro-high".to_string()),
                permissions: vec![PermissionPolicy::Supervised],
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(Box::pin(futures::stream::once(async move {
            patch::executor_discovered_options(options)
        })))
    }
}
