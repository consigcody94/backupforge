use backupforge_server::{create_router, AppState, ServerConfig};
use std::net::SocketAddr;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create server configuration
    let config = ServerConfig::default();
    let bind_addr = format!("{}:{}", config.bind_address, config.port);

    // Create application state
    let state = AppState::new(config.clone()).await?;

    // Create router
    let app = create_router(state);

    // Parse socket address
    let addr: SocketAddr = bind_addr.parse()?;

    tracing::info!("BackupForge server starting on http://{}", addr);
    tracing::info!("Dashboard available at: http://{}", addr);
    tracing::info!("API available at: http://{}/api", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
