use engram::{MemoryStore, default_db_path, mcp::EngramMcpServer};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            match args.next().as_deref() {
                Some("grant") => run_grant_command(args.collect()).await,
                _ => run_mcp_server().await,
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

async fn run_mcp_server() -> anyhow::Result<()> {
    init_process_logging();

    let db_path = default_db_path();
    tracing::info!("[engram-mcp] opening store at {}", db_path.display());

    let store = MemoryStore::open(&db_path).await.map_err(|error| {
        tracing::error!("[engram-mcp] failed to open store: {}", error);
        error
    })?;

    let server = EngramMcpServer::new(store);
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
