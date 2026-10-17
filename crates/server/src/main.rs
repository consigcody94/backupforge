use backupforge_server::{create_router, AppState, ServerConfig};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Prototype: login and registration are not implemented and the API has no
    // working authentication. Refuse to start unless explicitly enabled.
    if std::env::var("BACKUPFORGE_SERVER_ENABLE").as_deref() != Ok("1") {
        eprintln!(
            "backupforge-server is an unfinished prototype (no working authentication). \
             Set BACKUPFORGE_SERVER_ENABLE=1 to run it anyway."
        );
        std::process::exit(2);
    }

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
