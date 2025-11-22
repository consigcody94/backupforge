use anyhow::Result;
use backupforge_agent::BackupAgent;
use backupforge_core::BackupConfig;
use backupforge_storage::StorageConfig;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::resources;
use crate::tools;
use crate::prompts;

pub struct McpServer {
    agent: Arc<RwLock<Option<BackupAgent>>>,
    config_path: Option<String>,
}

impl McpServer {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            agent: Arc::new(RwLock::new(None)),
            config_path: None,
        })
    }

    pub async fn handle_request(&mut self, request: Value) -> Value {
        let method = request["method"].as_str().unwrap_or("");
        let id = request["id"].clone();

        match method {
            "initialize" => self.handle_initialize(id, &request).await,
            "tools/list" => self.handle_list_tools(id).await,
            "tools/call" => self.handle_call_tool(id, &request).await,
            "resources/list" => self.handle_list_resources(id).await,
            "resources/read" => self.handle_read_resource(id, &request).await,
            "prompts/list" => self.handle_list_prompts(id).await,
            "prompts/get" => self.handle_get_prompt(id, &request).await,
            _ => json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                },
                "id": id
            }),
        }
    }

    async fn handle_initialize(&mut self, id: Value, request: &Value) -> Value {
        tracing::info!("Initializing MCP server");

        // Initialize default agent with local storage
        let backup_config = BackupConfig::default();
        let storage_config = StorageConfig::Local {
            path: std::env::var("BACKUPFORGE_STORAGE")
                .unwrap_or_else(|_| "/var/lib/backupforge/storage".to_string()),
        };

        match BackupAgent::new(backup_config, storage_config).await {
            Ok(agent) => {
                *self.agent.write().await = Some(agent);
            }
            Err(e) => {
                tracing::warn!("Failed to initialize agent: {}", e);
            }
        }

        json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "backupforge-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            },
            "id": id
        })
    }

    async fn handle_list_tools(&self, id: Value) -> Value {
        let tools = tools::get_tools_list();

        json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": tools
            },
            "id": id
        })
    }

    async fn handle_call_tool(&self, id: Value, request: &Value) -> Value {
        let tool_name = request["params"]["name"].as_str().unwrap_or("");
        let arguments = &request["params"]["arguments"];

        tracing::info!("Calling tool: {}", tool_name);

        let agent_lock = self.agent.read().await;
        let result = tools::execute_tool(tool_name, arguments, &agent_lock).await;

        match result {
            Ok(content) => json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": content
                },
                "id": id
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32000,
                    "message": format!("Tool execution failed: {}", e)
                },
                "id": id
            }),
        }
    }

    async fn handle_list_resources(&self, id: Value) -> Value {
        let resources = resources::get_resources_list();

        json!({
            "jsonrpc": "2.0",
            "result": {
                "resources": resources
            },
            "id": id
        })
    }

    async fn handle_read_resource(&self, id: Value, request: &Value) -> Value {
        let uri = request["params"]["uri"].as_str().unwrap_or("");

        tracing::info!("Reading resource: {}", uri);

        let agent_lock = self.agent.read().await;
        let result = resources::read_resource(uri, &agent_lock).await;

        match result {
            Ok(content) => json!({
                "jsonrpc": "2.0",
                "result": {
                    "contents": content
                },
                "id": id
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32000,
                    "message": format!("Resource read failed: {}", e)
                },
                "id": id
            }),
        }
    }

    async fn handle_list_prompts(&self, id: Value) -> Value {
        let prompts = prompts::get_prompts_list();

        json!({
            "jsonrpc": "2.0",
            "result": {
                "prompts": prompts
            },
            "id": id
        })
    }

    async fn handle_get_prompt(&self, id: Value, request: &Value) -> Value {
        let prompt_name = request["params"]["name"].as_str().unwrap_or("");
        let arguments = &request["params"]["arguments"];

        tracing::info!("Getting prompt: {}", prompt_name);

        let result = prompts::get_prompt(prompt_name, arguments);

        match result {
            Ok(messages) => json!({
                "jsonrpc": "2.0",
                "result": {
                    "messages": messages
                },
                "id": id
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32000,
                    "message": format!("Prompt retrieval failed: {}", e)
                },
                "id": id
            }),
        }
    }
}
