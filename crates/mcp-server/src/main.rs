mod server;
mod tools;
mod resources;
mod prompts;

use anyhow::Result;
use serde_json::json;
use std::io::{self, BufRead, Write};
use tracing_subscriber;

use server::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing (stderr only, as stdout is used for MCP protocol)
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("BackupForge MCP Server starting...");

    let mut server = McpServer::new().await?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        tracing::debug!("Received: {}", line);

        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(request) => {
                let response = server.handle_request(request).await;
                let response_str = serde_json::to_string(&response)?;

                tracing::debug!("Sending: {}", response_str);

                writeln!(stdout, "{}", response_str)?;
                stdout.flush()?;
            }
            Err(e) => {
                tracing::error!("Failed to parse request: {}", e);
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
