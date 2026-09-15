use engram::{MemoryStore, default_db_path, mcp::EngramMcpServer};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
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
        })
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
