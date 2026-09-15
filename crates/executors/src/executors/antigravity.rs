//! Ejecutor nativo para Google Antigravity CLI (`agy`).
//!
//! En lugar de depender de adaptadores ACP externos inestables (como `agy-acp` npm),
//! este ejecutor se comunica directamente con el binario nativo `agy.exe` de Google Antigravity
//! mediante `--output-format stream-json`, leyendo eventos estructurados NDJSON por stdout
//! y suministrando el prompt vía stdin con `-p -`.

use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use futures::StreamExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::{
    command_ext::GroupSpawnNoWindowExt, msg_store::MsgStore, path::make_path_relative,
};

use crate::{
    approvals::ExecutorApprovalService,
    command::{apply_overrides, CmdOverrides, CommandBuildError, CommandBuilder, CommandParts},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor,
    },
    logs::{
        stderr_processor::normalize_stderr_logs,
        utils::{
            patch::{add_normalized_entry, replace_normalized_entry, upsert_normalized_entry},
            EntryIndexProvider,
        },
        ActionType, CommandExitStatus, CommandRunResult, FileChange, NormalizedEntry,
        NormalizedEntryError, NormalizedEntryType, TokenUsageInfo, ToolResult, ToolStatus,
    },
    model_selector::{ModelInfo, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct Antigravity {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub permission_policy: Option<PermissionPolicy>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
    #[serde(skip)]
    #[ts(skip)]
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub approvals: Option<Arc<dyn ExecutorApprovalService>>,
}

fn find_agy_binary() -> String {
    if let Some(local_app_data) = dirs::data_local_dir() {
        let candidate = local_app_data.join("agy").join("bin").join("agy.exe");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    "agy".to_string()
}

impl Antigravity {
    pub fn build_command_builder(&self) -> Result<CommandBuilder, CommandBuildError> {
        let program = find_agy_binary();
        let mut builder =
            CommandBuilder::new(program).params(["-p", "-", "--output-format", "stream-json"]);

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        match self.permission_policy {
            Some(PermissionPolicy::Auto) => {
                builder = builder.extend_params(["--dangerously-skip-permissions"]);
            }
            Some(PermissionPolicy::Plan) => {
                builder = builder.extend_params(["--mode", "plan"]);
            }
            Some(PermissionPolicy::Supervised) | None => {
                // Modo supervisado / predeterminado de agy
            }
        }

        apply_overrides(builder, &self.cmd)
    }
}

async fn spawn_agy(
    command_parts: CommandParts,
    prompt: &str,
    current_dir: &Path,
    env: &ExecutionEnv,
    cmd_overrides: &CmdOverrides,
) -> Result<SpawnedChild, ExecutorError> {
    let (program_path, args) = command_parts.into_resolved().await?;

    let mut command = tokio::process::Command::new(program_path);
    command
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(current_dir)
        .args(args);

    env.clone()
        .with_profile(cmd_overrides)
        .apply_to_command(&mut command);

    let mut child = command.group_spawn_no_window()?;

    if let Some(mut stdin) = child.inner().stdin.take() {
        tokio::io::AsyncWriteExt::write_all(&mut stdin, prompt.as_bytes()).await?;
        tokio::io::AsyncWriteExt::shutdown(&mut stdin).await?;
    }

    Ok(child.into())
}

#[async_trait]
impl StandardCodingAgentExecutor for Antigravity {
    fn apply_overrides(&mut self, executor_config: &ExecutorConfig) {
        if let Some(model_id) = &executor_config.model_id {
            self.model = Some(model_id.clone());
        }
        if let Some(permission_policy) = &executor_config.permission_policy {
            self.permission_policy = Some(permission_policy.clone());
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
        let agy_command = self.build_command_builder()?.build_initial()?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);

        spawn_agy(agy_command, &combined_prompt, current_dir, env, &self.cmd).await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        _reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let continue_cmd = self
            .build_command_builder()?
            .build_follow_up(&["--conversation".to_string(), session_id.to_string()])?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);

        spawn_agy(continue_cmd, &combined_prompt, current_dir, env, &self.cmd).await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        normalize_logs(
            msg_store.clone(),
            worktree_path,
            EntryIndexProvider::start_from(&msg_store),
        )
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
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

        if let Some(local_app_data) = dirs::data_local_dir() {
            if local_app_data.join("agy").join("bin").join("agy.exe").exists() {
                return AvailabilityInfo::InstallationFound;
            }
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
            permission_policy: Some(
                self.permission_policy
                    .clone()
                    .unwrap_or(PermissionPolicy::Auto),
            ),
        }
    }

    async fn discover_options(
        &self,
        _workdir: Option<&std::path::Path>,
        _repo_path: Option<&std::path::Path>,
    ) -> Result<futures::stream::BoxStream<'static, json_patch::Patch>, ExecutorError> {
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
                default_model: Some("gemini-3.8-flash-high".to_string()),
                permissions: vec![
                    PermissionPolicy::Auto,
                    PermissionPolicy::Supervised,
                    PermissionPolicy::Plan,
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(Box::pin(futures::stream::once(async move {
            crate::logs::utils::patch::executor_discovered_options(options)
        })))
    }
}

// =============================================================================
// Estructuras de deserialización de eventos agy stream-json
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AgyEvent {
    Init {
        conversation_id: Option<String>,
        #[serde(default)]
        init: Option<AgyInitData>,
    },
    StepUpdate {
        step_update: AgyStepUpdate,
    },
    Result {
        result: AgyResult,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AgyInitData {
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub permission_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgyStepUpdate {
    pub conversation_id: Option<String>,
    pub step_index: usize,
    pub state: String,
    pub step_type: String,
    pub tool_name: Option<String>,
    pub tool_info: Option<AgyToolInfo>,
    pub text_delta: Option<String>,
    pub duration_seconds: Option<f64>,
    pub usage: Option<AgyUsage>,
}

#[derive(Debug, Deserialize)]
pub struct AgyToolInfo {
    pub name: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub output: Option<String>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AgyUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub thinking_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AgyResult {
    pub conversation_id: Option<String>,
    pub status: String,
    pub response: Option<String>,
    pub duration_seconds: Option<f64>,
    pub num_turns: Option<u64>,
    pub usage: Option<AgyUsage>,
}

// =============================================================================
// Normalización de Logs y Streaming
// =============================================================================

#[derive(Debug, Clone)]
struct StreamingText {
    index: usize,
    content: String,
}

#[derive(Debug, Clone, Copy)]
enum UpdateMode {
    Append,
}

fn update_streaming_text(
    entry_index: &EntryIndexProvider,
    text: &str,
    entry_type: NormalizedEntryType,
    stream_key: &str,
    map: &mut HashMap<String, StreamingText>,
    msg_store: &Arc<MsgStore>,
    _mode: UpdateMode,
) {
    if text.is_empty() {
        return;
    }

    let is_new = !map.contains_key(stream_key);
    if is_new && text == "\n" {
        return;
    }

    let state = map
        .entry(stream_key.to_string())
        .or_insert_with(|| StreamingText {
            index: entry_index.next(),
            content: String::new(),
        });

    state.content.push_str(text);

    let entry = NormalizedEntry {
        timestamp: None,
        entry_type,
        content: state.content.clone(),
        metadata: None,
    };
    upsert_normalized_entry(msg_store, state.index, entry, is_new);
}

pub fn map_agy_tool(
    tool_name: &str,
    tool_info: Option<&AgyToolInfo>,
    state_str: &str,
    worktree_path_str: &str,
) -> (ActionType, ToolStatus, String) {
    let is_done = state_str == "DONE";
    let is_error = state_str == "ERROR";
    let status = if is_done {
        ToolStatus::Success
    } else if is_error {
        ToolStatus::Failed
    } else {
        ToolStatus::Created
    };

    let params = tool_info
        .and_then(|info| info.parameters.as_ref())
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let output = tool_info.and_then(|info| info.output.clone());

    match tool_name {
        "run_command" => {
            let command = params
                .get("CommandLine")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let category = crate::logs::utils::shell_command_parsing::CommandCategory::from_command(&command);
            let result = if is_done || is_error {
                Some(CommandRunResult {
                    exit_status: Some(CommandExitStatus::ExitCode {
                        code: if is_error { 1 } else { 0 },
                    }),
                    output: output.clone(),
                })
            } else {
                None
            };
            let summary = format!("Running command: {}", command);
            (
                ActionType::CommandRun {
                    command,
                    result,
                    category,
                },
                status,
                summary,
            )
        }
        "replace_file_content" | "multi_replace_file_content" => {
            let target_file = params
                .get("TargetFile")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rel_path = make_path_relative(&target_file, worktree_path_str);
            let changes = if let (Some(target), Some(replacement)) = (
                params.get("TargetContent").and_then(|v| v.as_str()),
                params.get("ReplacementContent").and_then(|v| v.as_str()),
            ) {
                let diff =
                    workspace_utils::diff::create_unified_diff(&target_file, target, replacement);
                vec![FileChange::Edit {
                    unified_diff: diff,
                    has_line_numbers: false,
                }]
            } else {
                vec![]
            };
            let summary = format!("Editing {}", rel_path);
            (
                ActionType::FileEdit {
                    path: rel_path,
                    changes,
                },
                status,
                summary,
            )
        }
        "write_to_file" => {
            let target_file = params
                .get("TargetFile")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rel_path = make_path_relative(&target_file, worktree_path_str);
            let content = params
                .get("CodeContent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let summary = format!("Writing {}", rel_path);
            (
                ActionType::FileEdit {
                    path: rel_path,
                    changes: vec![FileChange::Write { content }],
                },
                status,
                summary,
            )
        }
        "view_file" | "list_dir" | "read_url_content" => {
            let path_str = params
                .get("AbsolutePath")
                .or_else(|| params.get("DirectoryPath"))
                .or_else(|| params.get("Url"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_path = make_path_relative(path_str, worktree_path_str);
            let summary = format!("Reading {}", rel_path);
            (ActionType::FileRead { path: rel_path }, status, summary)
        }
        "grep_search" | "find_by_name" => {
            let query = params
                .get("Query")
                .or_else(|| params.get("Pattern"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let summary = format!("Searching for {}", query);
            (ActionType::Search { query }, status, summary)
        }
        _ => {
            let res = output.map(ToolResult::markdown);
            let summary = format!("Tool call: {}", tool_name);
            (
                ActionType::Tool {
                    tool_name: tool_name.to_string(),
                    arguments: if params.is_null() { None } else { Some(params) },
                    result: res,
                },
                status,
                summary,
            )
        }
    }
}

pub fn normalize_logs(
    msg_store: Arc<MsgStore>,
    worktree_path: &Path,
    entry_index_provider: EntryIndexProvider,
) -> Vec<tokio::task::JoinHandle<()>> {
    let h1 = normalize_stderr_logs(msg_store.clone(), entry_index_provider.clone());

    let worktree_path = worktree_path.to_path_buf();
    let h2 = tokio::spawn(async move {
        let worktree_path_str = worktree_path.to_string_lossy().to_string();
        let mut session_id_pushed = false;
        let mut tool_entries: HashMap<usize, usize> = HashMap::new();
        let mut assistant_text: HashMap<String, StreamingText> = HashMap::new();
        let mut token_usage_reported = false;

        let mut stdout_lines = msg_store.stdout_lines_stream();
        while let Some(Ok(line)) = stdout_lines.next().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let agy_event = match serde_json::from_str::<AgyEvent>(trimmed) {
                Ok(ev) => ev,
                Err(_) => {
                    let clean = strip_ansi_escapes::strip_str(trimmed);
                    if !clean.is_empty() {
                        add_normalized_entry(
                            &msg_store,
                            &entry_index_provider,
                            NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::SystemMessage,
                                content: clean.to_string(),
                                metadata: None,
                            },
                        );
                    }
                    continue;
                }
            };

            // Extraer y asociar ID de conversación
            if !session_id_pushed {
                let sid = match &agy_event {
                    AgyEvent::Init {
                        conversation_id, ..
                    } => conversation_id.as_deref(),
                    AgyEvent::StepUpdate { step_update } => {
                        step_update.conversation_id.as_deref()
                    }
                    AgyEvent::Result { result } => result.conversation_id.as_deref(),
                    _ => None,
                };
                if let Some(sid) = sid {
                    msg_store.push_session_id(sid.to_string());
                    session_id_pushed = true;
                }
            }

            match agy_event {
                AgyEvent::Init { init, .. } => {
                    if let Some(init_data) = init {
                        if let Some(model) = init_data.model {
                            add_normalized_entry(
                                &msg_store,
                                &entry_index_provider,
                                NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::SystemMessage,
                                    content: format!("model: {}", model),
                                    metadata: None,
                                },
                            );
                        }
                    }
                }
                AgyEvent::StepUpdate { step_update } => {
                    match step_update.step_type.as_str() {
                        "agent_response" => {
                            let key = format!("step-{}", step_update.step_index);
                            if let Some(text) = &step_update.text_delta {
                                update_streaming_text(
                                    &entry_index_provider,
                                    text,
                                    NormalizedEntryType::AssistantMessage,
                                    &key,
                                    &mut assistant_text,
                                    &msg_store,
                                    UpdateMode::Append,
                                );
                            }
                            if let Some(usage) = &step_update.usage {
                                if !token_usage_reported && let Some(total) = usage.total_tokens {
                                    token_usage_reported = true;
                                    add_normalized_entry(
                                        &msg_store,
                                        &entry_index_provider,
                                        NormalizedEntry {
                                            timestamp: None,
                                            entry_type: NormalizedEntryType::TokenUsageInfo(
                                                TokenUsageInfo {
                                                    total_tokens: total as u32,
                                                    model_context_window: 1_000_000,
                                                },
                                            ),
                                            content: format!("Tokens used: {}", total),
                                            metadata: None,
                                        },
                                    );
                                }
                            }
                        }
                        "tool" => {
                            let tool_name = step_update.tool_name.as_deref().unwrap_or("tool");
                            let (action_type, status, summary) = map_agy_tool(
                                tool_name,
                                step_update.tool_info.as_ref(),
                                &step_update.state,
                                &worktree_path_str,
                            );

                            let entry = NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::ToolUse {
                                    tool_name: tool_name.to_string(),
                                    action_type,
                                    status,
                                },
                                content: summary,
                                metadata: None,
                            };

                            if let Some(existing_idx) = tool_entries.get(&step_update.step_index) {
                                replace_normalized_entry(&msg_store, *existing_idx, entry);
                            } else {
                                let idx =
                                    add_normalized_entry(&msg_store, &entry_index_provider, entry);
                                tool_entries.insert(step_update.step_index, idx);
                            }
                        }
                        _ => {}
                    }
                }
                AgyEvent::Result { result } => {
                    if result.status != "SUCCESS" {
                        let err_msg = result.response.unwrap_or_else(|| {
                            format!("Antigravity completed with status: {}", result.status)
                        });
                        add_normalized_entry(
                            &msg_store,
                            &entry_index_provider,
                            NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::ErrorMessage {
                                    error_type: NormalizedEntryError::Other,
                                },
                                content: err_msg,
                                metadata: None,
                            },
                        );
                    }
                }
                AgyEvent::Unknown => {}
            }
        }
    });

    vec![h1, h2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_builder_flags() {
        let mut agy = Antigravity {
            append_prompt: AppendPrompt::default(),
            model: Some("gemini-3.8-flash-high".to_string()),
            permission_policy: Some(PermissionPolicy::Auto),
            cmd: CmdOverrides::default(),
            approvals: None,
        };

        let initial_parts = agy.build_command_builder().unwrap().build_initial().unwrap();
        assert!(initial_parts.args.contains(&"-p".to_string()));
        assert!(initial_parts.args.contains(&"-".to_string()));
        assert!(initial_parts.args.contains(&"--output-format".to_string()));
        assert!(initial_parts.args.contains(&"stream-json".to_string()));
        assert!(initial_parts.args.contains(&"--model".to_string()));
        assert!(initial_parts.args.contains(&"gemini-3.8-flash-high".to_string()));
        assert!(initial_parts.args.contains(&"--dangerously-skip-permissions".to_string()));

        let follow_up_parts = agy
            .build_command_builder()
            .unwrap()
            .build_follow_up(&["--conversation".to_string(), "test-conv-123".to_string()])
            .unwrap();
        assert!(follow_up_parts.args.contains(&"--conversation".to_string()));
        assert!(follow_up_parts.args.contains(&"test-conv-123".to_string()));

        // Plan mode
        agy.permission_policy = Some(PermissionPolicy::Plan);
        let plan_parts = agy.build_command_builder().unwrap().build_initial().unwrap();
        assert!(plan_parts.args.contains(&"--mode".to_string()));
        assert!(plan_parts.args.contains(&"plan".to_string()));
    }

    #[test]
    fn test_parse_agy_event_lifecycle() {
        let init_json = r#"{"event":"init","conversation_id":"conv-abc","init":{"model":"gemini-3.8-flash-high","cwd":"C:\\test","permission_mode":"always-proceed"}}"#;
        let event: AgyEvent = serde_json::from_str(init_json).unwrap();
        match event {
            AgyEvent::Init { conversation_id, init } => {
                assert_eq!(conversation_id.as_deref(), Some("conv-abc"));
                assert_eq!(init.unwrap().model.as_deref(), Some("gemini-3.8-flash-high"));
            }
            _ => panic!("Expected Init event"),
        }

        let step_tool_json = r#"{"event":"step_update","step_update":{"conversation_id":"conv-abc","step_index":2,"state":"ACTIVE","step_type":"tool","tool_name":"run_command","tool_info":{"name":"run_command","parameters":{"CommandLine":"git status"}}}}"#;
        let event: AgyEvent = serde_json::from_str(step_tool_json).unwrap();
        match event {
            AgyEvent::StepUpdate { step_update } => {
                assert_eq!(step_update.step_index, 2);
                assert_eq!(step_update.state, "ACTIVE");
                assert_eq!(step_update.tool_name.as_deref(), Some("run_command"));
            }
            _ => panic!("Expected StepUpdate event"),
        }

        let result_json = r#"{"event":"result","result":{"conversation_id":"conv-abc","status":"SUCCESS","response":"Done!","duration_seconds":1.23,"num_turns":1}}"#;
        let event: AgyEvent = serde_json::from_str(result_json).unwrap();
        match event {
            AgyEvent::Result { result } => {
                assert_eq!(result.status, "SUCCESS");
                assert_eq!(result.response.as_deref(), Some("Done!"));
            }
            _ => panic!("Expected Result event"),
        }
    }
}
