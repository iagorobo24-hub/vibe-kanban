//! MCP stdio server exposing Engram's memory store as three tools:
//! `engram_write`, `engram_read`, `engram_forget`. Kept deliberately small —
//! the contract in `docs/agentos/02-ARQUITECTURA.md` calls for an explicit,
//! caducable grant per read/write, not a free-form query surface.

use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EngramError, MemoryStore};

type ToolCallResult = Result<CallToolResult, ErrorData>;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WriteRequest {
    #[schemars(description = "Explicit, caducable grant issued by the control plane.")]
    grant_id: Uuid,
    #[schemars(description = "Identity of the agent receiving the grant.")]
    recipient_id: String,
    #[schemars(description = "AgentOS session authorized by the grant.")]
    session_id: String,
    #[schemars(
        description = "Project namespace this memory belongs to. Never leaks across namespaces."
    )]
    namespace: String,
    #[schemars(
        description = "Identity of the writer (e.g. agent name or session id), for provenance."
    )]
    origin: String,
    #[schemars(description = "The memory content to store.")]
    content: String,
    #[schemars(
        description = "Optional time-to-live in seconds. Omit for an entry that does not expire on its own."
    )]
    ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReadRequest {
    #[schemars(description = "Explicit, caducable grant issued by the control plane.")]
    grant_id: Uuid,
    #[schemars(description = "Identity of the agent receiving the grant.")]
    recipient_id: String,
    #[schemars(description = "AgentOS session authorized by the grant.")]
    session_id: String,
    #[schemars(
        description = "Project namespace to read memory from. Only entries in this namespace are returned."
    )]
    namespace: String,
    #[schemars(
        description = "Maximum number of entries to return, most recent first. Defaults to 20."
    )]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ForgetRequest {
    #[schemars(description = "Explicit, caducable grant issued by the control plane.")]
    grant_id: Uuid,
    #[schemars(description = "Identity of the agent receiving the grant.")]
    recipient_id: String,
    #[schemars(description = "AgentOS session authorized by the grant.")]
    session_id: String,
    #[schemars(description = "Project namespace containing the entry.")]
    namespace: String,
    #[schemars(
        description = "Id of the memory entry to remove (returned by engram_write/engram_read)."
    )]
    id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ForgetResponse {
    #[schemars(description = "Whether an entry with this id existed and was removed.")]
    existed: bool,
}

#[derive(Clone)]
pub struct EngramMcpServer {
    store: MemoryStore,
    tool_router: ToolRouter<EngramMcpServer>,
}

#[tool_router]
impl EngramMcpServer {
    pub fn new(store: MemoryStore) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Write a memory entry to a project namespace using an explicit active ContextGrant. Fails cleanly (never crashes the caller) if Engram is unavailable or authorization is denied."
    )]
    async fn engram_write(&self, Parameters(req): Parameters<WriteRequest>) -> ToolCallResult {
        match self
            .store
            .write_with_grant(
                req.grant_id,
                &req.recipient_id,
                &req.session_id,
                &req.namespace,
                &req.origin,
                &req.content,
                req.ttl_seconds,
            )
            .await
        {
            Ok(entry) => Self::success(&entry),
            Err(error) => Self::err(error),
        }
    }

    #[tool(
        description = "Read current (non-expired) memory entries using an explicit active ContextGrant. Never returns entries from another namespace."
    )]
    async fn engram_read(&self, Parameters(req): Parameters<ReadRequest>) -> ToolCallResult {
        let limit = req.limit.unwrap_or(20);
        match self
            .store
            .read_with_grant(
                req.grant_id,
                &req.recipient_id,
                &req.session_id,
                &req.namespace,
                limit,
            )
            .await
        {
            Ok(entries) => Self::success(&entries),
            Err(error) => Self::err(error),
        }
    }

    #[tool(
        description = "Remove a memory entry by id using an explicit write ContextGrant — the correction mechanism for a wrong or stale entry."
    )]
    async fn engram_forget(&self, Parameters(req): Parameters<ForgetRequest>) -> ToolCallResult {
        match self
            .store
            .forget_with_grant(
                req.grant_id,
                &req.recipient_id,
                &req.session_id,
                &req.namespace,
                req.id,
            )
            .await
        {
            Ok(existed) => Self::success(&ForgetResponse { existed }),
            Err(error) => Self::err(error),
        }
    }
}

impl EngramMcpServer {
    fn success<T: Serialize>(data: &T) -> ToolCallResult {
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| "Failed to serialize response".to_string()),
        )]))
    }

    fn err(error: EngramError) -> ToolCallResult {
        let value = serde_json::json!({
            "success": false,
            "error": error.to_string(),
        });
        Ok(CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&value)
                .unwrap_or_else(|_| "Failed to serialize error".to_string()),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for EngramMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("engram-mcp", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
            .with_instructions(
                "Shared memory across AgentOS agents, scoped by project namespace. \
                 The control plane must issue an explicit, caducable ContextGrant \
                 before engram_write, engram_read or engram_forget can access memory. \
                 Each request carries the grant, recipient, session and namespace; \
                 mismatches or expiry are denied. Namespaces are isolated: reading \
                 one namespace never returns another project's entries. If Engram \
                 itself is unavailable, tools return a clean error instead of \
                 failing the calling session.",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exposes_exactly_the_three_contract_tools() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = MemoryStore::open(&dir.path().join("engram.db"))
            .await
            .expect("open store");
        let server = EngramMcpServer::new(store);

        let names: std::collections::BTreeSet<String> = server
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();

        assert_eq!(
            names,
            std::collections::BTreeSet::from([
                "engram_write".to_string(),
                "engram_read".to_string(),
                "engram_forget".to_string(),
            ])
        );
    }

    #[test]
    fn memory_requests_carry_explicit_grant_identity() {
        let missing_grant = serde_json::from_value::<WriteRequest>(serde_json::json!({
            "namespace": "proj-a",
            "origin": "agent-1",
            "content": "note"
        }));
        assert!(missing_grant.is_err());

        let missing_recipient = serde_json::from_value::<ReadRequest>(serde_json::json!({
            "grant_id": Uuid::new_v4(),
            "session_id": "session-1",
            "namespace": "proj-a"
        }));
        assert!(missing_recipient.is_err());
    }
}
