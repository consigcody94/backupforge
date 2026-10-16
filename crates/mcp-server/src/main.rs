mod prompts;
mod resources;
mod server;
mod tools;

use anyhow::Result;
use serde_json::json;
use std::io::{self, BufRead, Write};

use server::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Prototype: most tools are not implemented, the agent is never initialised,
    // and there are no access controls on which paths a client may read or write.
    if std::env::var("BACKUPFORGE_MCP_ENABLE").as_deref() != Ok("1") {
        eprintln!(
            "backupforge-mcp is an unfinished prototype (most tools do not work and \
             there are no path access controls). Set BACKUPFORGE_MCP_ENABLE=1 to run it anyway."
        );
        std::process::exit(2);
    }

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
