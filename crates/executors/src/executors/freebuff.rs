//! Freebuff executor — drives the Freebuff TUI through a ConPTY and tracks
//! the session by tailing its structured JSONL log.
//!
//! The `freebuff` CLI is purely interactive (no `--print`/`--json`/stdin mode:
//! verified against the native binary, and consistent with the upstream
//! `FREEBUFF_MODE` build flag that strips non-interactive flags). The `@codebuff/sdk`
//! rejects free device-OAuth tokens (HTTP 402), so the only viable free path is
//! driving the TUI itself.
//!
//! Design (mirrors the proven `Freebuff-telegram-bot` tmux automation):
//! - spawn `freebuff --cwd <worktree>` under a ConPTY (`portable_pty`)
//! - submit prompts with terminal keys (`C-u`, text, `Enter`)
//! - NEVER parse content off the screen: every turn is tracked from
//!   `~/.config/manicode/projects/<slug>/chats/<ts>/log.jsonl`
//!   (`Start agent` with our prompt = accepted run,
//!    `End agent` with `agentId == "main-agent"` = progress,
//!    `Main prompt finished` = turn complete)
//! - the container requires a real group-spawned child with piped stdio, so a
//!   lightweight companion process (`powershell -c "$Input | Out-Null"`) fills
//!   the `SpawnedChild.child` slot; the real PTY child is owned by a supervisor
//!   task wired to the exit signal and the cancellation token.

use std::{
    collections::HashMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use derivative::Derivative;
use portable_pty::{CommandBuilder as PtyCommandBuilder, NativePtySystem, PtySize, PtySystem};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use ts_rs::TS;
use workspace_utils::{
    command_ext::GroupSpawnNoWindowExt, msg_store::MsgStore, path::make_path_relative,
};

use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, ExecutorExitResult,
        SpawnedChild, StandardCodingAgentExecutor,
    },
    logs::{
        ActionType, NormalizedEntry, NormalizedEntryError, NormalizedEntryType, TokenUsageInfo,
        ToolResult, ToolStatus,
        stderr_processor::normalize_stderr_logs,
        utils::{
            EntryIndexProvider,
            patch::{add_normalized_entry, replace_normalized_entry, upsert_normalized_entry},
        },
    },
    model_selector::{ModelInfo, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

/// Maps a worktree path to the freebuff `log.jsonl` of the live session, so
/// `normalize_logs` (which runs without spawn context) can find the transcript.
static SESSION_LOG_REGISTRY: std::sync::LazyLock<RwLock<HashMap<PathBuf, PathBuf>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

fn canonical_key(worktree: &Path) -> PathBuf {
    let s = worktree.to_string_lossy();
    // Windows paths are case-insensitive; normalize so spawn-time and
    // normalize-time lookups hit the same entry.
    PathBuf::from(s.to_lowercase())
}

fn register_session_log(worktree: &Path, log: &Path) {
    if let Ok(mut reg) = SESSION_LOG_REGISTRY.write() {
        reg.insert(canonical_key(worktree), log.to_path_buf());
    }
}

fn session_log_for(worktree: &Path) -> Option<PathBuf> {
    SESSION_LOG_REGISTRY
        .read()
        .ok()
        .and_then(|reg| reg.get(&canonical_key(worktree)).cloned())
}

/// Last-resort discovery when the spawn-time registration is missing
/// (e.g. server restart between spawn and normalize): newest chat log for
/// the project slug touched in the last hour.
fn fallback_session_log(worktree: &Path) -> Option<PathBuf> {
    let slug = project_slug(worktree);
    let chats = manicode_dir()?.join("projects").join(&slug).join("chats");
    let cutoff = SystemTime::now() - Duration::from_secs(3600);
    let entries = std::fs::read_dir(&chats).ok()?;
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let log = entry.path().join("log.jsonl");
        if !log.is_file() {
            continue;
        }
        let mtime = std::fs::metadata(&log).and_then(|m| m.modified()).ok()?;
        if mtime < cutoff {
            continue;
        }
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, log));
        }
    }
    best.map(|(_, p)| p)
}

#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct Freebuff {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    /// Model id used to steer the TUI model picker (e.g. `glm-5-3-flash`).
    /// The CLI accepts no `--model` flag; `None` keeps the TUI default.
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

fn manicode_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config").join("manicode"))
}

fn find_freebuff_binary() -> String {
    if let Some(exe) = manicode_dir().map(|d| d.join("freebuff.exe")) {
        if exe.is_file() {
            return exe.to_string_lossy().to_string();
        }
    }
    "freebuff".to_string()
}

fn project_slug(worktree: &Path) -> String {
    worktree
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string()
}

impl Freebuff {
    pub fn build_command_builder(&self) -> Result<CommandBuilder, CommandBuildError> {
        // NOTE: the freebuff CLI accepts (almost) no flags of its own — only
        // `login`, `--continue`, `--cwd`, `-v`. Model / approval options are
        // handled through the TUI (picker / settings), never argv.
        let program = find_freebuff_binary();
        let builder = CommandBuilder::new(program);
        apply_overrides(builder, &self.cmd)
    }
}

// ---------------------------------------------------------------------------
// TUI screen classification (control flow only; content comes from JSONL)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiScreen {
    ModelPicker,
    Chat,
    Unknown,
}

fn classify_screen(text: &str) -> TuiScreen {
    if text.contains("Freebucks/hr") || text.contains("Freebucks /hr") {
        return TuiScreen::ModelPicker;
    }
    if text.trim().len() < 20 {
        return TuiScreen::Unknown;
    }
    TuiScreen::Chat
}

/// Names of the picker rows plus the index of the highlighted (`›`) row.
fn picker_rows(screen: &str) -> (Vec<String>, Option<usize>) {
    let mut names = Vec::new();
    let mut cursor = None;
    for line in screen.lines() {
        if line.contains('│') && (line.contains('·') || line.contains("Freebucks")) {
            let name = line
                .chars()
                .filter(|c| !matches!(c, '│' | '›' | '┌' | '┐' | '└' | '┘' | '─'))
                .collect::<String>()
                .split('·')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if name.is_empty() {
                continue;
            }
            if line.contains('›') {
                cursor = Some(names.len());
            }
            names.push(name);
        }
    }
    (names, cursor)
}

// ---------------------------------------------------------------------------
// Turn tracker (port of the telegram-bot JsonlTurnTracker protocol)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum TurnUpdate {
    AwaitingAcceptance,
    Accepted {
        run_id: String,
    },
    Running {
        run_id: String,
        candidate_len: usize,
    },
    Completed {
        run_id: String,
        response: String,
    },
    Unverified {
        reason: &'static str,
    },
}

struct TurnTracker {
    normalized_prompt: String,
    iteration_runs: HashMap<u64, String>,
    accepted_run: Option<String>,
    latest_candidate: Option<String>,
    terminal: Option<TurnUpdate>,
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

impl TurnTracker {
    fn new(prompt: &str) -> Self {
        Self {
            normalized_prompt: normalize_newlines(prompt),
            iteration_runs: HashMap::new(),
            accepted_run: None,
            latest_candidate: None,
            terminal: None,
        }
    }

    fn update(&self) -> TurnUpdate {
        if let Some(t) = &self.terminal {
            return t.clone();
        }
        if let Some(run_id) = &self.accepted_run {
            if let Some(c) = &self.latest_candidate {
                return TurnUpdate::Running {
                    run_id: run_id.clone(),
                    candidate_len: c.len(),
                };
            }
            return TurnUpdate::Accepted {
                run_id: run_id.clone(),
            };
        }
        TurnUpdate::AwaitingAcceptance
    }

    fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }

    /// Feed one parsed JSONL object (`msg` + `data`). Returns true when terminal.
    fn consume(&mut self, msg: &str, data: &serde_json::Map<String, serde_json::Value>) -> bool {
        if self.is_terminal() {
            return true;
        }
        if msg.starts_with("Start agent") {
            let iteration = data.get("iteration").and_then(|v| v.as_u64());
            let run_id = data.get("runId").and_then(|v| v.as_str());
            match (iteration, run_id) {
                (Some(it), Some(rid)) => {
                    self.iteration_runs.insert(it, rid.to_string());
                    if self.accepted_run.is_none()
                        && data
                            .get("prompt")
                            .and_then(|v| v.as_str())
                            .map(|p| normalize_newlines(p) == self.normalized_prompt)
                            .unwrap_or(false)
                    {
                        self.accepted_run = Some(rid.to_string());
                    } else if self.accepted_run.as_deref() != Some(rid)
                        && self.accepted_run.is_some()
                    {
                        self.terminal = Some(TurnUpdate::Unverified {
                            reason: "competing_run",
                        });
                    }
                }
                _ => {
                    if self.accepted_run.is_some() {
                        self.terminal = Some(TurnUpdate::Unverified {
                            reason: "run_identity_lost",
                        });
                    }
                }
            }
            return self.is_terminal();
        }
        if self.accepted_run.is_none() {
            return false;
        }
        if msg.starts_with("End agent") {
            let main = data.get("agentId").and_then(|v| v.as_str()) == Some("main-agent");
            if !main {
                return false;
            }
            let iteration = data.get("iteration").and_then(|v| v.as_u64());
            let full = data
                .get("fullResponse")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if full.is_empty() {
                return false;
            }
            match iteration.and_then(|it| self.iteration_runs.get(&it).cloned()) {
                Some(rid) if Some(rid.as_str()) == self.accepted_run.as_deref() => {
                    self.latest_candidate = Some(full.to_string());
                }
                _ => {
                    self.terminal = Some(TurnUpdate::Unverified {
                        reason: "run_identity_lost",
                    });
                }
            }
            return self.is_terminal();
        }
        if msg == "Main prompt finished" {
            match self.latest_candidate.clone() {
                Some(response) => {
                    let run_id = self.accepted_run.clone().unwrap_or_default();
                    self.terminal = Some(TurnUpdate::Completed { run_id, response });
                }
                None => {
                    self.terminal = Some(TurnUpdate::Unverified {
                        reason: "terminal_without_response",
                    });
                }
            }
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// JSONL session-log discovery + tail
// ---------------------------------------------------------------------------

fn file_contains_pid(log: &Path, pid: u32) -> bool {
    let content = match std::fs::read(log) {
        Ok(c) => c,
        Err(_) => return false,
    };
    if content.len() > 8 * 1024 * 1024 {
        return false;
    }
    let needle = format!("\"pid\":{pid}");
    content
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

fn newest_chat_log(dir: &Path, since: SystemTime, pid: u32) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let log = entry.path().join("log.jsonl");
        if !log.is_file() {
            continue;
        }
        let mtime = std::fs::metadata(&log).and_then(|m| m.modified()).ok()?;
        if mtime < since {
            continue;
        }
        // Confirm ownership via the `pid` field before accepting the file.
        if !file_contains_pid(&log, pid) {
            continue;
        }
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, log));
        }
    }
    best.map(|(_, p)| p)
}

/// Newest chat log by mtime only (no pid ownership check). Used as a
/// fallback when the log is too young to carry the pid field yet.
fn newest_chat_log_any(dir: &Path, since: SystemTime) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let log = entry.path().join("log.jsonl");
        if !log.is_file() {
            continue;
        }
        let mtime = std::fs::metadata(&log).and_then(|m| m.modified()).ok()?;
        if mtime < since {
            continue;
        }
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, log));
        }
    }
    best.map(|(_, p)| p)
}

async fn locate_session_log(
    slug: &str,
    since: SystemTime,
    pid: u32,
    deadline: SystemTime,
) -> Option<PathBuf> {
    // Preferred: the project slug directory. Fallback: any project (the slug
    // rules are internal to freebuff and may differ per version).
    // pid ownership is preferred, but young logs may not carry the pid field
    // yet: after a grace period accept the newest fresh log regardless.
    let pid_grace = since + Duration::from_secs(30);
    while SystemTime::now() < deadline {
        let now = SystemTime::now();
        let mut fallback: Option<(SystemTime, PathBuf)> = None;
        let mut search = |dir: &Path| {
            if let Some(log) = newest_chat_log(dir, since, pid) {
                return Some(log);
            }
            if now > pid_grace {
                if let Some(log) = newest_chat_log_any(dir, since) {
                    let mtime = std::fs::metadata(&log)
                        .and_then(|m| m.modified())
                        .unwrap_or(since);
                    if fallback.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                        fallback = Some((mtime, log));
                    }
                }
            }
            None
        };
        if let Some(dir) = manicode_dir().map(|d| d.join("projects").join(slug).join("chats")) {
            if let Some(log) = search(&dir) {
                return Some(log);
            }
        }
        if let Some(projects) = manicode_dir().map(|d| d.join("projects")) {
            if let Ok(entries) = std::fs::read_dir(&projects) {
                for entry in entries.flatten() {
                    if let Some(log) = search(&entry.path().join("chats")) {
                        return Some(log);
                    }
                }
            }
        }
        if now > pid_grace {
            if let Some((_, log)) = fallback {
                return Some(log);
            }
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    None
}

// ---------------------------------------------------------------------------
// PTY supervisor
// ---------------------------------------------------------------------------

const BOOT_TIMEOUT: Duration = Duration::from_secs(90);
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(180);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(45 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const PROMPT_RESEND_EVERY: Duration = Duration::from_secs(25);
const MAX_PROMPT_SENDS: u32 = 4;

fn pty_write(writer: &mut Box<dyn Write + Send>, bytes: &[u8]) {
    let _ = writer.write_all(bytes);
    let _ = writer.flush();
}

async fn send_prompt(writer: &mut Box<dyn Write + Send>, prompt: &str) {
    pty_write(writer, &[0x15]); // C-u: clear the input line first
    tokio::time::sleep(Duration::from_millis(300)).await;
    pty_write(writer, prompt.as_bytes());
    // The TUI needs a beat between typing and submit; without it the
    // Enter is swallowed (verified by recon: 300ms gaps submit reliably).
    tokio::time::sleep(Duration::from_millis(300)).await;
    pty_write(writer, b"\r"); // Enter submits (freebuff >= 0.0.135)
}

async fn spawn_freebuff_pty(
    prompt: &str,
    model_hint: Option<String>,
    current_dir: &Path,
    env: &ExecutionEnv,
    cmd_overrides: &CmdOverrides,
    extra_args: &[String],
) -> Result<SpawnedChild, ExecutorError> {
    let binary = find_freebuff_binary();
    let mut pty_cmd = PtyCommandBuilder::new(&binary);
    pty_cmd.cwd(current_dir);
    pty_cmd.arg("--cwd");
    pty_cmd.arg(current_dir.to_string_lossy().as_ref());
    for arg in extra_args {
        pty_cmd.arg(arg);
    }
    pty_cmd.env("TERM", "xterm-256color");
    let profiled = env.clone().with_profile(cmd_overrides);
    for (key, value) in profiled.vars.iter() {
        pty_cmd.env(key, value);
    }

    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| ExecutorError::Io(std::io::Error::other(e.to_string())))?;
    let master = pair.master;
    let mut pty_child = pair
        .slave
        .spawn_command(pty_cmd)
        .map_err(|e| ExecutorError::Io(std::io::Error::other(e.to_string())))?;
    let pid = pty_child.process_id().unwrap_or(0);

    let mut writer = master
        .take_writer()
        .map_err(|e| ExecutorError::Io(std::io::Error::other(e.to_string())))?;
    let reader = master
        .try_clone_reader()
        .map_err(|e| ExecutorError::Io(std::io::Error::other(e.to_string())))?;

    // Screen-text channel for picker/chat detection.
    let (screen_tx, mut screen_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if screen_tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Companion process: the container requires a real group-spawned child with
    // piped stdio (`track_child_msgs_in_store` takes stdout/stderr). It consumes
    // stdin and emits nothing; session content comes from the JSONL log.
    let mut companion = Command::new("powershell.exe");
    companion
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(current_dir)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg("$Input | Out-Null");
    let companion = companion.group_spawn_no_window()?;

    let (exit_tx, exit_rx) = tokio::sync::oneshot::channel();
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_task = cancel.clone();
    let prompt_owned = prompt.to_string();
    let worktree = current_dir.to_path_buf();
    let spawn_started = SystemTime::now();

    tokio::spawn(async move {
        // Keep the master PTY alive for the whole session: dropping it
        // closes the pseudo-console, which terminates the attached process.
        let _pty_master = master;
        tracing::info!(target: "freebuff", pid, "PTY session started");
        let result = drive_session(
            &mut writer,
            &mut screen_rx,
            &prompt_owned,
            model_hint,
            &worktree,
            pid,
            spawn_started,
            cancel_task,
        )
        .await;
        tracing::info!(target: "freebuff", pid, ?result, "PTY session ended");
        let _ = pty_child.kill();
        let _ = exit_tx.send(result);
    });

    Ok(SpawnedChild {
        child: companion,
        exit_signal: Some(exit_rx),
        cancel: Some(cancel),
    })
}

async fn drive_session(
    writer: &mut Box<dyn Write + Send>,
    screen_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    prompt: &str,
    model_hint: Option<String>,
    worktree: &Path,
    pid: u32,
    spawn_started: SystemTime,
    cancel: tokio_util::sync::CancellationToken,
) -> ExecutorExitResult {
    let boot_deadline = SystemTime::now() + BOOT_TIMEOUT;
    let mut screen_text = String::new();
    let mut sends: u32 = 0;
    // The model picker appears for fresh project dirs even with a saved
    // default model; always dismiss it before typing into chat.
    let mut picker_done = false;

    // Phase 1: reach a submittable screen and send the prompt. JSONL
    // acceptance is ground truth; the screen only steers the picker.
    let mut prompt_accepted = false;
    while SystemTime::now() < boot_deadline && !prompt_accepted {
        if cancel.is_cancelled() {
            return ExecutorExitResult::Failure;
        }
        if let Ok(Some(bytes)) = tokio::time::timeout(POLL_INTERVAL, screen_rx.recv()).await {
            screen_text.push_str(&String::from_utf8_lossy(&bytes));
            if screen_text.len() > 64 * 1024 {
                let skip = screen_text.floor_char_boundary(screen_text.len() - 64 * 1024);
                screen_text.drain(..skip);
            }
        }
        match classify_screen(&screen_text) {
            TuiScreen::ModelPicker if !picker_done => {
                if let Some(want) = model_hint.as_deref() {
                    let (names, cursor) = picker_rows(&screen_text);
                    if let (Some(cur), Some(idx)) = (
                        cursor,
                        names
                            .iter()
                            .position(|n| n.to_lowercase().contains(&want.to_lowercase())),
                    ) {
                        let key = if idx > cur { b"\x1b[B" } else { b"\x1b[A" };
                        for _ in 0..idx.abs_diff(cur) {
                            pty_write(writer, key);
                            tokio::time::sleep(Duration::from_millis(150)).await;
                        }
                    }
                }
                pty_write(writer, b"\r");
                picker_done = true;
                screen_text.clear();
            }
            TuiScreen::Chat if sends < MAX_PROMPT_SENDS => {
                // Only type once the input box is really there; keys sent
                // during "Connecting…" are swallowed. Observed marker:
                // "Enter a coding task or / for commands".
                let ready = screen_text.contains("coding task")
                    || SystemTime::now()
                        .duration_since(spawn_started)
                        .unwrap_or_default()
                        > Duration::from_secs(30);
                if ready {
                    send_prompt(writer, prompt).await;
                    sends += 1;
                }

                if wait_for_acceptance(worktree_hint(worktree), prompt, pid).await {
                    prompt_accepted = true;
                }
            }
            _ => {}
        }
    }

    // Phase 2: locate the session log and track the turn to completion.
    // `since` is spawn time: the chat dir is created during boot, i.e.
    // possibly *before* this point.
    let slug = project_slug(worktree);
    let log_path =
        locate_session_log(&slug, spawn_started, pid, SystemTime::now() + ACCEPT_TIMEOUT).await;
    let log_path = match log_path {
        Some(p) => p,
        None => return ExecutorExitResult::Failure,
    };
    register_session_log(worktree, &log_path);
    tracing::info!(target: "freebuff", pid, log = %log_path.display(), "session log located");

    let mut tracker = TurnTracker::new(prompt);
    let mut offset: u64 = 0;
    let mut pending = String::new();
    let mut last_send = SystemTime::now();
    let accept_deadline = SystemTime::now() + ACCEPT_TIMEOUT;
    let response_deadline = SystemTime::now() + RESPONSE_TIMEOUT;
    loop {
        if cancel.is_cancelled() {
            return ExecutorExitResult::Failure;
        }
        let now = SystemTime::now();
        if now > response_deadline {
            return ExecutorExitResult::Failure;
        }
        if tracker.accepted_run.is_none() {
            if now > accept_deadline {
                return ExecutorExitResult::Failure;
            }
            // Resend while unaccepted (TUI may have swallowed keys).
            if sends < MAX_PROMPT_SENDS
                && now.duration_since(last_send).unwrap_or_default() > PROMPT_RESEND_EVERY
            {
                send_prompt(writer, prompt).await;
                sends += 1;
                last_send = now;
            }
        }
        match read_new_events(&log_path, &mut offset, &mut pending) {
            Ok(events) => {
                for (msg, data) in &events {
                    if tracker.consume(msg, data) {
                        break;
                    }
                }
                match tracker.update() {
                    TurnUpdate::Completed { run_id, .. } => {
                        tracing::info!(target: "freebuff", pid, %run_id, "turn completed");
                        return ExecutorExitResult::Success;
                    }
                    TurnUpdate::Unverified { reason } => {
                        tracing::warn!(target: "freebuff", pid, reason, "turn unverified");
                        return ExecutorExitResult::Failure;
                    }
                    _ => {}
                }
            }
            Err(_) => {
                tokio::time::sleep(POLL_INTERVAL).await;
                continue;
            }
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Best-effort early acceptance check used right after sending keys.
async fn wait_for_acceptance(_worktree: PathBuf, _prompt: &str, _pid: u32) -> bool {
    // Acceptance is authoritatively decided in phase 2 by the TurnTracker;
    // here we just yield so the TUI can process the keys.
    tokio::time::sleep(Duration::from_secs(2)).await;
    false
}

fn worktree_hint(worktree: &Path) -> PathBuf {
    worktree.to_path_buf()
}

fn read_new_events(
    log: &Path,
    offset: &mut u64,
    pending: &mut String,
) -> std::io::Result<Vec<(String, serde_json::Map<String, serde_json::Value>)>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(log)?;
    let size = file.metadata()?.len();
    if size < *offset {
        *offset = 0; // rotated/truncated: restart (rare mid-turn)
        pending.clear();
    }
    file.seek(SeekFrom::Start(*offset))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    *offset = size;
    pending.push_str(&String::from_utf8_lossy(&buf));
    let mut events = Vec::new();
    // Keep the trailing incomplete segment (no closing newline yet) for later.
    let complete_tail = pending.ends_with('\n');
    let mut lines: Vec<&str> = pending.split('\n').collect();
    let rest = if complete_tail {
        String::new()
    } else {
        lines.pop().unwrap_or("").to_string()
    };
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // corrupt line: skip, never fail the turn on it
        };
        let msg = v
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let data = v
            .get("data")
            .and_then(|d| d.as_object())
            .cloned()
            .unwrap_or_default();
        events.push((msg, data));
    }
    *pending = rest;
    Ok(events)
}

#[async_trait]
impl StandardCodingAgentExecutor for Freebuff {
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
        // Validates user overrides; the TUI itself takes no argv.
        let _ = self.build_command_builder()?.build_initial()?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        spawn_freebuff_pty(
            &combined_prompt,
            self.model.clone(),
            current_dir,
            env,
            &self.cmd,
            &[],
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
        let _ = self
            .build_command_builder()?
            .build_follow_up(&["--continue".to_string(), session_id.to_string()])?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        spawn_freebuff_pty(
            &combined_prompt,
            self.model.clone(),
            current_dir,
            env,
            &self.cmd,
            &["--continue".to_string(), session_id.to_string()],
        )
        .await
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
        // The TUI owns its MCP servers (global config); the server injects nothing.
        None
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        if manicode_dir()
            .map(|d| d.join("freebuff.exe").is_file())
            .unwrap_or(false)
        {
            return AvailabilityInfo::InstallationFound;
        }
        if manicode_dir().map(|d| d.is_dir()).unwrap_or(false) {
            return AvailabilityInfo::InstallationFound;
        }
        AvailabilityInfo::NotFound
    }

    fn get_preset_options(&self) -> ExecutorConfig {
        ExecutorConfig {
            executor: BaseCodingAgent::Freebuff,
            variant: None,
            model_id: self.model.clone(),
            agent_id: None,
            reasoning_id: None,
            permission_policy: Some(
                self.permission_policy
                    .clone()
                    .unwrap_or(PermissionPolicy::Supervised),
            ),
        }
    }

    async fn discover_options(
        &self,
        _workdir: Option<&std::path::Path>,
        _repo_path: Option<&std::path::Path>,
    ) -> Result<futures::stream::BoxStream<'static, json_patch::Patch>, ExecutorError> {
        // Model ids observed in the live TUI picker (names shown to the user).
        let models = [
            ("glm-5-3-flash", "GLM 5.3 Flash"),
            ("mimo-2-5", "MiMo 2.5"),
            ("deepseek-v4-flash-0731", "DeepSeek V4 Flash 07/31"),
        ];
        let options = ExecutorDiscoveredOptions {
            model_selector: ModelSelectorConfig {
                models: models
                    .into_iter()
                    .map(|(id, name)| ModelInfo {
                        id: id.to_string(),
                        name: name.to_string(),
                        provider_id: Some("freebuff".to_string()),
                        reasoning_options: vec![],
                        is_secondary: None,
                    })
                    .collect(),
                default_model: Some("glm-5-3-flash".to_string()),
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

// ---------------------------------------------------------------------------
// Log normalization from the JSONL session transcript
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct StreamingText {
    index: usize,
    content: String,
}

fn upsert_assistant_text(
    entry_index: &EntryIndexProvider,
    text: &str,
    stream_key: &str,
    map: &mut HashMap<String, StreamingText>,
    msg_store: &Arc<MsgStore>,
) {
    if text.is_empty() {
        return;
    }
    let is_new = !map.contains_key(stream_key);
    let state = map
        .entry(stream_key.to_string())
        .or_insert_with(|| StreamingText {
            index: entry_index.next(),
            content: String::new(),
        });
    state.content.push_str(text);
    let entry = NormalizedEntry {
        timestamp: None,
        entry_type: NormalizedEntryType::AssistantMessage,
        content: state.content.clone(),
        metadata: None,
    };
    upsert_normalized_entry(msg_store, state.index, entry, is_new);
}

fn map_tool_call(
    tool_name: &str,
    input: &serde_json::Value,
    worktree_path_str: &str,
) -> (ActionType, String) {
    match tool_name {
        "basher" | "run_command" | "run_terminal_command" => {
            let command = input
                .get("command")
                .or_else(|| input.get("CommandLine"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let category =
                crate::logs::utils::shell_command_parsing::CommandCategory::from_command(&command);
            let summary = format!("Running command: {command}");
            (
                ActionType::CommandRun {
                    command,
                    result: None,
                    category,
                },
                summary,
            )
        }
        "str_replace" | "replace_file_content" | "multi_replace_file_content" => {
            let target_file = input
                .get("path")
                .or_else(|| input.get("TargetFile"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rel_path = make_path_relative(&target_file, worktree_path_str);
            let summary = format!("Editing {rel_path}");
            (
                ActionType::FileEdit {
                    path: rel_path,
                    changes: vec![],
                },
                summary,
            )
        }
        "write_file" | "write_to_file" => {
            let target_file = input
                .get("path")
                .or_else(|| input.get("TargetFile"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rel_path = make_path_relative(&target_file, worktree_path_str);
            let summary = format!("Writing {rel_path}");
            (
                ActionType::FileEdit {
                    path: rel_path,
                    changes: vec![],
                },
                summary,
            )
        }
        "read_files" | "read_file" | "view_file" => {
            let target_file = input
                .get("path")
                .or_else(|| input.get("AbsolutePath"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rel_path = make_path_relative(&target_file, worktree_path_str);
            let summary = format!("Reading {rel_path}");
            (ActionType::FileRead { path: rel_path }, summary)
        }
        "code_searcher" | "grep_search" | "search" => {
            let query = input
                .get("query")
                .or_else(|| input.get("pattern"))
                .or_else(|| input.get("Query"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let summary = format!("Searching for {query}");
            (ActionType::Search { query }, summary)
        }
        _ => {
            let summary = format!("Tool call: {tool_name}");
            (
                ActionType::Tool {
                    tool_name: tool_name.to_string(),
                    arguments: if input.is_null() {
                        None
                    } else {
                        Some(input.clone())
                    },
                    result: None,
                },
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
        let worktree_str = worktree_path.to_string_lossy().to_string();
        let deadline = SystemTime::now() + Duration::from_secs(90);
        let mut log_path = None;
        while SystemTime::now() < deadline {
            if let Some(p) = session_log_for(&worktree_path).or_else(|| fallback_session_log(&worktree_path)) {
                log_path = Some(p);
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let Some(log_path) = log_path else {
            add_normalized_entry(
                &msg_store,
                &entry_index_provider,
                NormalizedEntry {
                    timestamp: None,
                    entry_type: NormalizedEntryType::ErrorMessage {
                        error_type: NormalizedEntryError::Other,
                    },
                    content: "Freebuff session log not found; transcript unavailable.".to_string(),
                    metadata: None,
                },
            );
            return;
        };

        let mut session_id_pushed = false;
        let mut assistant_text: HashMap<String, StreamingText> = HashMap::new();
        let mut tool_entries: HashMap<String, usize> = HashMap::new();
        let mut offset: u64 = 0;
        let mut pending = String::new();
        let mut finished = false;

        while !finished {
            let events = read_new_events(&log_path, &mut offset, &mut pending).unwrap_or_default();
            if events.is_empty() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            for (msg, data) in &events {
                if msg.starts_with("Start agent") {
                    if !session_id_pushed {
                        if let Some(run_id) = data.get("runId").and_then(|v| v.as_str()) {
                            msg_store.push_session_id(run_id.to_string());
                            session_id_pushed = true;
                        }
                    }
                    if let Some(model) = data.get("model").and_then(|v| v.as_str()) {
                        add_normalized_entry(
                            &msg_store,
                            &entry_index_provider,
                            NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::SystemMessage,
                                content: format!("Freebuff model: {model}"),
                                metadata: None,
                            },
                        );
                    }
                } else if msg.starts_with("End agent") {
                    if data.get("agentId").and_then(|v| v.as_str()) != Some("main-agent") {
                        continue;
                    }
                    if let Some(full) = data.get("fullResponse").and_then(|v| v.as_str()) {
                        if !full.is_empty() {
                            upsert_assistant_text(
                                &entry_index_provider,
                                full,
                                "assistant",
                                &mut assistant_text,
                                &msg_store,
                            );
                        }
                    }
                    if let Some(calls) = data.get("toolCalls").and_then(|v| v.as_array()) {
                        let results = data
                            .get("toolResults")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        for (i, call) in calls.iter().enumerate() {
                            let name = call
                                .get("toolName")
                                .and_then(|v| v.as_str())
                                .unwrap_or("tool");
                            let input = call.get("input").cloned().unwrap_or_default();
                            let (action_type, summary) = map_tool_call(name, &input, &worktree_str);
                            let failed = results
                                .get(i)
                                .and_then(|r| r.get("error"))
                                .is_some_and(|e| !e.is_null());
                            let status = if failed {
                                ToolStatus::Failed
                            } else {
                                ToolStatus::Success
                            };
                            let output = results
                                .get(i)
                                .and_then(|r| r.get("output"))
                                .and_then(|v| v.as_str())
                                .map(|s| ToolResult::markdown(s.to_string()));
                            let action_type = match action_type {
                                ActionType::Tool {
                                    tool_name,
                                    arguments,
                                    ..
                                } => ActionType::Tool {
                                    tool_name,
                                    arguments,
                                    result: output,
                                },
                                other => other,
                            };
                            let entry = NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::ToolUse {
                                    tool_name: name.to_string(),
                                    action_type,
                                    status,
                                },
                                content: summary,
                                metadata: None,
                            };
                            let key = call
                                .get("toolCallId")
                                .and_then(|v| v.as_str())
                                .unwrap_or(name)
                                .to_string();
                            if let Some(idx) = tool_entries.get(&key) {
                                replace_normalized_entry(&msg_store, *idx, entry);
                            } else {
                                let idx =
                                    add_normalized_entry(&msg_store, &entry_index_provider, entry);
                                tool_entries.insert(key, idx);
                            }
                        }
                    }
                    if let Some(tokens) = data.get("contextTokenCount").and_then(|v| v.as_u64()) {
                        add_normalized_entry(
                            &msg_store,
                            &entry_index_provider,
                            NormalizedEntry {
                                timestamp: None,
                                entry_type: NormalizedEntryType::TokenUsageInfo(TokenUsageInfo {
                                    total_tokens: tokens.try_into().unwrap_or(u32::MAX),
                                    input_tokens: None,
                                    output_tokens: None,
                                    model_context_window: 1_000_000,
                                    ..Default::default()
                                }),
                                content: format!("Context tokens: {tokens}"),
                                metadata: None,
                            },
                        );
                    }
                } else if msg == "Main prompt finished" {
                    finished = true;
                    break;
                }
            }
        }
    });

    vec![h1, h2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: &[(&str, serde_json::Value)]) -> serde_json::Map<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn start_msg(prompt: &str) -> (String, serde_json::Map<String, serde_json::Value>) {
        (
            "Start agent base3 step 1 (run-1 - Prompt: ..)".to_string(),
            obj(&[
                ("iteration", serde_json::json!(1)),
                ("runId", serde_json::json!("run-1")),
                ("prompt", serde_json::json!(prompt)),
            ]),
        )
    }

    #[test]
    fn test_turn_accepts_matching_prompt_and_completes() {
        let mut t = TurnTracker::new("hello world");
        let (m, d) = start_msg("hello world");
        assert!(!t.consume(&m, &d));
        assert_eq!(
            t.update(),
            TurnUpdate::Accepted {
                run_id: "run-1".to_string()
            }
        );
        let end = obj(&[
            ("iteration", serde_json::json!(1)),
            ("agentId", serde_json::json!("main-agent")),
            ("shouldEndTurn", serde_json::json!(true)),
            ("fullResponse", serde_json::json!("PONG")),
        ]);
        assert!(!t.consume("End agent base3 step 1 (run-1)", &end));
        assert!(matches!(t.update(), TurnUpdate::Running { .. }));
        assert!(t.consume("Main prompt finished", &obj(&[])));
        assert_eq!(
            t.update(),
            TurnUpdate::Completed {
                run_id: "run-1".to_string(),
                response: "PONG".to_string()
            }
        );
    }

    #[test]
    fn test_turn_ignores_foreign_prompt_and_competing_run() {
        let mut t = TurnTracker::new("my prompt");
        let (m, d) = start_msg("someone else");
        assert!(!t.consume(&m, &d));
        assert_eq!(t.update(), TurnUpdate::AwaitingAcceptance);
        let (m2, mut d2) = start_msg("my prompt");
        d2.insert("runId".to_string(), serde_json::json!("run-9"));
        assert!(!t.consume(&m2, &d2));
        let (m3, mut d3) = start_msg("my prompt");
        d3.insert("runId".to_string(), serde_json::json!("run-evil"));
        assert!(t.consume(&m3, &d3));
        assert!(matches!(
            t.update(),
            TurnUpdate::Unverified {
                reason: "competing_run"
            }
        ));
    }

    #[test]
    fn test_turn_terminal_without_response_is_unverified() {
        let mut t = TurnTracker::new("p");
        let (m, d) = start_msg("p");
        t.consume(&m, &d);
        assert!(t.consume("Main prompt finished", &obj(&[])));
        assert!(matches!(
            t.update(),
            TurnUpdate::Unverified {
                reason: "terminal_without_response"
            }
        ));
    }

    #[test]
    fn test_classify_picker_and_chat() {
        let picker =
            "│  GLM 5.3 Flash · Deep reasoning │\n│› MiMo 2.5 · Balanced │\n│10 Freebucks/hr│";
        assert_eq!(classify_screen(picker), TuiScreen::ModelPicker);
        assert_eq!(
            classify_screen("some chat text with enough length to pass unknown"),
            TuiScreen::Chat
        );
        assert_eq!(classify_screen(""), TuiScreen::Unknown);
    }

    #[test]
    fn test_find_freebuff_binary_nonempty() {
        assert!(!find_freebuff_binary().is_empty());
    }

    #[test]
    fn test_build_command_builder_ok() {
        let fb = Freebuff {
            append_prompt: AppendPrompt::default(),
            model: None,
            permission_policy: None,
            cmd: CmdOverrides::default(),
            approvals: None,
        };
        assert!(fb.build_command_builder().is_ok());
    }

    /// Live end-to-end test against the real Freebuff TUI. Costs one free
    /// session and needs an authenticated CLI, so it runs ONLY with
    /// `FREEBUFF_LIVE_TEST=1` (never in CI).
    #[tokio::test]
    async fn test_live_session_smoke() {
        if std::env::var("FREEBUFF_LIVE_TEST").as_deref() != Ok("1") {
            return;
        }
        use crate::env::{ExecutionEnv, RepoContext};

        let workdir = std::env::temp_dir().join(format!("fb-live-{}", std::process::id()));
        std::fs::create_dir_all(&workdir).unwrap();
        let started = SystemTime::now();

        let fb = Freebuff {
            append_prompt: AppendPrompt::default(),
            model: None,
            permission_policy: None,
            cmd: CmdOverrides::default(),
            approvals: None,
        };
        let env = ExecutionEnv::new(
            RepoContext::new(workdir.clone(), vec![]),
            false,
            String::new(),
        );
        let prompt = "Reply with exactly: TEST OK. Do not modify any files. Do not call any tools.";
        let mut spawned = fb.spawn(&workdir, prompt, &env).await.expect("spawn PTY");

        // 1. The TUI must boot: a session log appears under manicode/projects.
        let log_path = tokio::time::timeout(Duration::from_secs(180), async {
            loop {
                if let Some(p) = newest_log_since(started) {
                    return p;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
        .await
        .expect("freebuff TUI never wrote a session log (PTY child died?)");

        // 2. The turn must complete with success.
        let exit_rx = spawned.exit_signal.take().expect("exit signal");
        let result = tokio::time::timeout(Duration::from_secs(600), exit_rx)
            .await
            .expect("turn timed out")
            .expect("exit sender dropped");
        assert!(
            matches!(result, ExecutorExitResult::Success),
            "turn failed: {result:?}"
        );

        // 3. Transcript must contain the finish marker and a real response.
        let content = std::fs::read_to_string(&log_path).expect("read log");
        assert!(content.contains("Main prompt finished"));
        let mut saw_response = false;
        for line in content.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let is_end = v
                .get("msg")
                .and_then(|m| m.as_str())
                .map(|m| m.starts_with("End agent"))
                .unwrap_or(false);
            let main = v
                .get("data")
                .and_then(|d| d.get("agentId"))
                .and_then(|a| a.as_str())
                == Some("main-agent");
            let full = v
                .get("data")
                .and_then(|d| d.get("fullResponse"))
                .and_then(|r| r.as_str())
                .unwrap_or("");
            if is_end && main && !full.is_empty() {
                saw_response = true;
                break;
            }
        }
        assert!(saw_response, "no main-agent fullResponse in transcript");

        // 4. Normalization must emit entries into the MsgStore.
        let store = Arc::new(MsgStore::new());
        register_session_log(&workdir, &log_path);
        let _handles = fb.normalize_logs(store.clone(), &workdir);
        let deadline = SystemTime::now() + Duration::from_secs(30);
        loop {
            let patches = store
                .get_history()
                .iter()
                .filter(|m| matches!(m, workspace_utils::log_msg::LogMsg::JsonPatch(_)))
                .count();
            if patches > 0 || SystemTime::now() > deadline {
                assert!(patches > 0, "normalize emitted no entries");
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let _ = std::fs::remove_dir_all(&workdir);
    }

    /// Recon: open the TUI under PTY, send NO keys, dump screens for 20s.
    /// No session starts (billing happens at session start), costs nothing.
    /// Run with `FREEBUFF_RECON=1` to see what the TUI shows at boot.
    #[tokio::test]
    async fn test_pty_recon() {
        let mode = std::env::var("FREEBUFF_RECON").unwrap_or_default();
        if mode != "1" && mode != "keys" && mode != "submit" {
            return;
        }
        let workdir = std::env::temp_dir().join(format!("fb-recon-{}", std::process::id()));
        std::fs::create_dir_all(&workdir).unwrap();

        let binary = find_freebuff_binary();
        let mut pty_cmd = PtyCommandBuilder::new(&binary);
        pty_cmd.cwd(&workdir);
        pty_cmd.arg("--cwd");
        pty_cmd.arg(workdir.to_string_lossy().as_ref());
        pty_cmd.env("TERM", "xterm-256color");
        let pair = NativePtySystem::default()
            .openpty(PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let master = pair.master;
        let mut child = pair.slave.spawn_command(pty_cmd).expect("spawn");
        let pid = child.process_id();
        println!("SPAWNED pid={pid:?}");
        let reader = master.try_clone_reader().expect("reader");
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let mut screen = Vec::new();
        let send_keys = std::env::var("FREEBUFF_RECON").as_deref() == Ok("keys");
        let do_submit = std::env::var("FREEBUFF_RECON").as_deref() == Ok("submit");
        let wait_secs = if do_submit { 60 } else { 20 };
        let t0 = SystemTime::now();
        let deadline = t0 + Duration::from_secs(wait_secs);
        let mut writer = master.take_writer().ok();
        let mut sent = false;
        while SystemTime::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(b) => screen.extend_from_slice(&b),
                Err(_) => {}
            }
            if (send_keys || do_submit)
                && !sent
                && t0.elapsed().unwrap_or_default() > Duration::from_secs(12)
            {
                if let Some(w) = writer.as_mut() {
                    if do_submit {
                        let _ = w.write_all(b"\x15");
                        let _ = w.flush();
                        std::thread::sleep(Duration::from_millis(300));
                        let _ =
                            w.write_all(b"RECON-SUBMIT-PROBE read-only: reply HI and do nothing");
                        let _ = w.flush();
                        std::thread::sleep(Duration::from_millis(300));
                        let _ = w.write_all(b"\r");
                        let _ = w.flush();
                        println!("SUBMIT-SENT");
                    } else {
                        let _ = w.write_all(b"HELLOPROBE-NOSUBMIT");
                        let _ = w.flush();
                        println!("KEYS-SENT");
                    }
                    sent = true;
                }
            }
        }
        let text = String::from_utf8_lossy(&screen);
        // Strip ANSI escapes for readability.
        let clean: String = strip_ansi(&text);
        println!("SCREEN-LEN raw={} clean={}", screen.len(), clean.len());
        let tail: String = clean.chars().rev().take(2500).collect::<String>().chars().rev().collect();
        println!("SCREEN-TAIL:\n{tail}");
        println!("classify={:?}", classify_screen(&clean));
        let _ = child.kill();
        let _ = std::fs::remove_dir_all(&workdir);
        let _ = master;
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // CSI ... letter, or OSC ... BEL, or single-char sequence.
                match chars.peek() {
                    Some('[') => {
                        chars.next();
                        for c2 in chars.by_ref() {
                            if c2.is_ascii_alphabetic() {
                                break;
                            }
                        }
                    }
                    Some(']') => {
                        chars.next();
                        for c2 in chars.by_ref() {
                            if c2 == '\x07' {
                                break;
                            }
                        }
                    }
                    Some(_) => {
                        chars.next();
                    }
                    None => {}
                }
            } else if c == '\x07' || c == '\r' {
                continue;
            } else {
                out.push(c);
            }
        }
        out
    }

    fn newest_log_since(since: SystemTime) -> Option<PathBuf> {        let projects = manicode_dir()?.join("projects");
        let entries = std::fs::read_dir(&projects).ok()?;
        let mut best: Option<(SystemTime, PathBuf)> = None;
        for proj in entries.flatten() {
            let chats = proj.path().join("chats");
            let chat_entries = match std::fs::read_dir(&chats) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for chat in chat_entries.flatten() {
                let log = chat.path().join("log.jsonl");
                if !log.is_file() {
                    continue;
                }
                let mtime = std::fs::metadata(&log).and_then(|m| m.modified()).ok()?;
                if mtime < since {
                    continue;
                }
                if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                    best = Some((mtime, log));
                }
            }
        }
        best.map(|(_, p)| p)
    }
}
