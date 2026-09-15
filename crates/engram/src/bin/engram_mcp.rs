use engram::{MemoryStore, default_db_path, mcp::{BoundIdentity, EngramMcpServer}};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{EnvFilter, prelude::*};
use uuid::Uuid;

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let first = args.first().cloned();
            match first.as_deref() {
                Some("grant") => {
                    args.remove(0);
                    run_grant_command(args).await
                }
                Some("revoke") => {
                    args.remove(0);
                    run_revoke_command(args).await
                }
                Some("serve") => {
                    args.remove(0);
                    run_mcp_server(args).await
                }
                _ => run_mcp_server(args).await,
            }
        })
}

/// Control-plane-only path to issue a `ContextGrant`. Deliberately not an
/// MCP tool (the MCP server never self-grants, see `grant_context` in
/// `engram::MemoryStore`) — this is a separate binary invocation meant to be
/// run by a human operator or by the AgentOS backend, never by an agent
/// session itself.
async fn run_grant_command(raw_args: Vec<String>) -> anyhow::Result<()> {
    let flags = GrantFlags::parse(&raw_args)?;

    let db_path = default_db_path();
    let store = MemoryStore::open(&db_path).await?;

    let grant = store
        .grant_context(
            &flags.recipient_id,
            &flags.session_id,
            &flags.project_id,
            &flags.scope,
            &flags.purpose,
            flags.ttl_seconds,
            &flags.granted_by,
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&grant)?);
    Ok(())
}

async fn run_revoke_command(raw_args: Vec<String>) -> anyhow::Result<()> {
    let flags = RevokeFlags::parse(&raw_args)?;

    let db_path = default_db_path();
    let store = MemoryStore::open(&db_path).await?;

    let revoked = store.revoke_grant(flags.grant_id, &flags.revoked_by).await?;
    if revoked {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "status": "revoked",
            "grant_id": flags.grant_id.to_string(),
            "revoked_by": flags.revoked_by
        }))?);
    } else {
        anyhow::bail!("grant {} not found or already revoked", flags.grant_id);
    }
    Ok(())
}

struct RevokeFlags {
    grant_id: Uuid,
    revoked_by: String,
}

impl RevokeFlags {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut grant_id = None;
        let mut revoked_by = None;

        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let mut value = || {
                it.next()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))
            };
            match flag.as_str() {
                "--grant" => grant_id = Some(Uuid::parse_str(&value()?)?),
                "--revoked-by" => revoked_by = Some(value()?),
                other => anyhow::bail!("unknown flag {other}"),
            }
        }

        Ok(Self {
            grant_id: grant_id.ok_or_else(|| anyhow::anyhow!("--grant is required"))?,
            revoked_by: revoked_by.ok_or_else(|| anyhow::anyhow!("--revoked-by is required"))?,
        })
    }
}

struct GrantFlags {
    recipient_id: String,
    session_id: String,
    project_id: String,
    scope: String,
    purpose: String,
    granted_by: String,
    ttl_seconds: i64,
}

impl GrantFlags {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut recipient_id = None;
        let mut session_id = None;
        let mut project_id = None;
        let mut scope = None;
        let mut purpose = None;
        let mut granted_by = None;
        let mut ttl_seconds: i64 = 3600;

        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let mut value = || {
                it.next()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))
            };
            match flag.as_str() {
                "--recipient" => recipient_id = Some(value()?),
                "--session" => session_id = Some(value()?),
                "--project" => project_id = Some(value()?),
                "--scope" => scope = Some(value()?),
                "--purpose" => purpose = Some(value()?),
                "--granted-by" => granted_by = Some(value()?),
                "--ttl-secs" => ttl_seconds = value()?.parse()?,
                other => anyhow::bail!("unknown flag {other}"),
            }
        }

        Ok(Self {
            recipient_id: recipient_id
                .ok_or_else(|| anyhow::anyhow!("--recipient is required"))?,
            session_id: session_id.ok_or_else(|| anyhow::anyhow!("--session is required"))?,
            project_id: project_id.ok_or_else(|| anyhow::anyhow!("--project is required"))?,
            scope: scope.ok_or_else(|| anyhow::anyhow!("--scope is required (memory:read | memory:write | memory:readwrite)"))?,
            purpose: purpose.ok_or_else(|| anyhow::anyhow!("--purpose is required"))?,
            granted_by: granted_by
                .ok_or_else(|| anyhow::anyhow!("--granted-by is required"))?,
            ttl_seconds,
        })
    }
}

struct ServerFlags {
    bound_recipient: Option<String>,
    bound_session: Option<String>,
    bound_project: Option<String>,
}

impl ServerFlags {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut bound_recipient = None;
        let mut bound_session = None;
        let mut bound_project = None;

        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let mut value = || {
                it.next()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))
            };
            match flag.as_str() {
                "--bound-recipient" => bound_recipient = Some(value()?),
                "--bound-session" => bound_session = Some(value()?),
                "--bound-project" => bound_project = Some(value()?),
                other => anyhow::bail!("unknown flag {other}"),
            }
        }

        Ok(Self {
            bound_recipient,
            bound_session,
            bound_project,
        })
    }
}

async fn run_mcp_server(raw_args: Vec<String>) -> anyhow::Result<()> {
    let server_flags = ServerFlags::parse(&raw_args)?;
    init_process_logging();

    let db_path = default_db_path();
    tracing::info!("[engram-mcp] opening store at {}", db_path.display());

    let store = MemoryStore::open(&db_path).await.map_err(|error| {
        tracing::error!("[engram-mcp] failed to open store: {}", error);
        error
    })?;

    let bound = BoundIdentity {
        recipient_id: server_flags.bound_recipient,
        session_id: server_flags.bound_session,
        project_id: server_flags.bound_project,
    };

    let server = EngramMcpServer::with_bound_identity(store, bound);
    let service = server.serve(stdio()).await.map_err(|error| {
        tracing::error!("[engram-mcp] serving error: {:?}", error);
        error
    })?;

    service.waiting().await?;
    Ok(())
}

fn init_process_logging() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(EnvFilter::new("info")),
        )
        .init();

    tracing::debug!(
        "[engram-mcp] starting Engram MCP server version {}...",
        env!("CARGO_PKG_VERSION")
    );
}
