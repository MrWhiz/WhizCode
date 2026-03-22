use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
    pub auto_restart: Option<bool>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_name: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerStatus {
    pub name: String,
    pub enabled: bool,
    pub running: bool,
    pub tools_count: usize,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerMarketplaceItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub installed: bool,
    pub version: String,
    pub author: String,
    pub rating: f32,
    pub downloads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfiguration {
    pub server_name: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPMetrics {
    pub total_servers: usize,
    pub enabled_servers: usize,
    pub running_servers: usize,
    pub total_tools: usize,
    pub total_executions: u32,
    pub successful_executions: u32,
    pub failed_executions: u32,
    pub average_execution_time_ms: f32,
}

pub struct MCPService {
    servers: Arc<Mutex<HashMap<String, MCPServerConfig>>>,
    tools: Arc<Mutex<Vec<MCPToolDefinition>>>,
    server_status: Arc<Mutex<HashMap<String, MCPServerStatus>>>,
    configurations: Arc<Mutex<Vec<MCPConfiguration>>>,
    execution_history: Arc<Mutex<Vec<(String, u64, bool)>>>,
}

impl MCPService {
    pub fn new() -> Self {
        MCPService {
            servers: Arc::new(Mutex::new(HashMap::new())),
            tools: Arc::new(Mutex::new(Vec::new())),
            server_status: Arc::new(Mutex::new(HashMap::new())),
            configurations: Arc::new(Mutex::new(Vec::new())),
            execution_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn initialize(&self) -> Result<(), String> {
        // Initialize default servers
        let mut servers = self.servers.lock().unwrap();

        // Filesystem server
        servers.insert(
            "filesystem".to_string(),
            MCPServerConfig {
                name: "filesystem".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                    ".".to_string(),
                ],
                env: None,
                enabled: Some(true),
                auto_restart: Some(true),
                created_at: Self::current_timestamp(),
            },
        );

        // GitHub server
        servers.insert(
            "github".to_string(),
            MCPServerConfig {
                name: "github".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-github".to_string(),
                ],
                env: Some(HashMap::new()),
                enabled: Some(false),
                auto_restart: Some(true),
                created_at: Self::current_timestamp(),
            },
        );

        // Puppeteer server
        servers.insert(
            "puppeteer".to_string(),
            MCPServerConfig {
                name: "puppeteer".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-puppeteer".to_string(),
                ],
                env: None,
                enabled: Some(false),
                auto_restart: Some(true),
                created_at: Self::current_timestamp(),
            },
        );

        Ok(())
    }

    pub fn add_server(&self, mut config: MCPServerConfig) -> Result<(), String> {
        config.created_at = Self::current_timestamp();
        let mut servers = self.servers.lock().unwrap();
        servers.insert(config.name.clone(), config.clone());

        // Initialize server status
        let mut status = self.server_status.lock().unwrap();
        status.insert(
            config.name.clone(),
            MCPServerStatus {
                name: config.name,
                enabled: config.enabled.unwrap_or(true),
                running: false,
                tools_count: 0,
                last_error: None,
                uptime_seconds: 0,
                restart_count: 0,
            },
        );

        Ok(())
    }

    pub fn remove_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().unwrap();
        servers.remove(name);

        let mut status = self.server_status.lock().unwrap();
        status.remove(name);

        Ok(())
    }

    pub fn enable_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().unwrap();
        if let Some(server) = servers.get_mut(name) {
            server.enabled = Some(true);
            Ok(())
        } else {
            Err(format!("Server {} not found", name))
        }
    }

    pub fn disable_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().unwrap();
        if let Some(server) = servers.get_mut(name) {
            server.enabled = Some(false);
            Ok(())
        } else {
            Err(format!("Server {} not found", name))
        }
    }

    pub fn get_servers(&self) -> Result<Vec<MCPServerConfig>, String> {
        let servers = self.servers.lock().unwrap();
        Ok(servers.values().cloned().collect())
    }

    pub fn get_server_status(&self, name: &str) -> Result<MCPServerStatus, String> {
        let status = self.server_status.lock().unwrap();
        status
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Server {} not found", name))
    }

    pub fn get_all_server_status(&self) -> Result<Vec<MCPServerStatus>, String> {
        let status = self.server_status.lock().unwrap();
        Ok(status.values().cloned().collect())
    }

    pub fn register_tool(&self, tool: MCPToolDefinition) -> Result<(), String> {
        let mut tools = self.tools.lock().unwrap();
        tools.push(tool);
        Ok(())
    }

    pub fn get_tools(&self) -> Result<Vec<MCPToolDefinition>, String> {
        let tools = self.tools.lock().unwrap();
        Ok(tools.clone())
    }

    pub fn get_tools_by_server(&self, server_name: &str) -> Result<Vec<MCPToolDefinition>, String> {
        let tools = self.tools.lock().unwrap();
        let filtered: Vec<MCPToolDefinition> = tools
            .iter()
            .filter(|t| t.server_name == server_name)
            .cloned()
            .collect();
        Ok(filtered)
    }

    pub fn call_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let tools = self.tools.lock().unwrap();
        let tool = tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| format!("Tool {} not found", tool_name))?;

        // Validate input against schema
        self.validate_tool_input(&args, &tool.input_schema)?;

        // Record execution
        let mut history = self.execution_history.lock().unwrap();
        history.push((tool_name.to_string(), Self::current_timestamp(), true));

        // In a real implementation, this would call the actual MCP server
        // For now, return a placeholder response
        Ok(serde_json::json!({
            "status": "success",
            "tool": tool_name,
            "message": "Tool execution placeholder"
        }))
    }

    fn validate_tool_input(
        &self,
        input: &serde_json::Value,
        schema: &serde_json::Value,
    ) -> Result<(), String> {
        // Basic validation - in production, use jsonschema crate
        if schema.is_object() {
            if let Some(required) = schema.get("required") {
                if let Some(required_fields) = required.as_array() {
                    for field in required_fields {
                        if let Some(field_name) = field.as_str() {
                            if !input.get(field_name).is_some() {
                                return Err(format!("Missing required field: {}", field_name));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_configuration(&self, config: MCPConfiguration) -> Result<(), String> {
        let mut configs = self.configurations.lock().unwrap();
        configs.push(config);
        Ok(())
    }

    pub fn get_configurations(&self, server_name: &str) -> Result<Vec<MCPConfiguration>, String> {
        let configs = self.configurations.lock().unwrap();
        let filtered: Vec<_> = configs
            .iter()
            .filter(|c| c.server_name == server_name)
            .cloned()
            .collect();
        Ok(filtered)
    }

    pub fn validate_configuration(&self, config: &MCPConfiguration) -> Result<bool, String> {
        // Basic validation - check if value matches expected type
        if config.config_value.is_null() {
            return Err("Configuration value cannot be null".to_string());
        }
        Ok(true)
    }

    pub fn get_marketplace(&self) -> Result<Vec<PowerMarketplaceItem>, String> {
        Ok(vec![
            PowerMarketplaceItem {
                id: "filesystem".to_string(),
                name: "Filesystem Power".to_string(),
                description: "Access and manipulate files and directories".to_string(),
                category: "filesystem".to_string(),
                installed: true,
                version: "1.0.0".to_string(),
                author: "Anthropic".to_string(),
                rating: 4.8,
                downloads: 50000,
            },
            PowerMarketplaceItem {
                id: "github".to_string(),
                name: "GitHub Power".to_string(),
                description: "Interact with GitHub repositories and APIs".to_string(),
                category: "vcs".to_string(),
                installed: false,
                version: "1.0.0".to_string(),
                author: "Anthropic".to_string(),
                rating: 4.7,
                downloads: 35000,
            },
            PowerMarketplaceItem {
                id: "puppeteer".to_string(),
                name: "Puppeteer Power".to_string(),
                description: "Browser automation and web scraping".to_string(),
                category: "web".to_string(),
                installed: false,
                version: "1.0.0".to_string(),
                author: "Anthropic".to_string(),
                rating: 4.6,
                downloads: 28000,
            },
            PowerMarketplaceItem {
                id: "database".to_string(),
                name: "Database Power".to_string(),
                description: "Query and manage databases".to_string(),
                category: "database".to_string(),
                installed: false,
                version: "1.0.0".to_string(),
                author: "Anthropic".to_string(),
                rating: 4.5,
                downloads: 22000,
            },
            PowerMarketplaceItem {
                id: "aws".to_string(),
                name: "AWS Power".to_string(),
                description: "Interact with AWS services".to_string(),
                category: "cloud".to_string(),
                installed: false,
                version: "1.0.0".to_string(),
                author: "Anthropic".to_string(),
                rating: 4.4,
                downloads: 18000,
            },
        ])
    }

    pub fn install_power(&self, power_id: &str) -> Result<(), String> {
        // In a real implementation, this would download and install the power
        eprintln!("Installing power: {}", power_id);
        Ok(())
    }

    pub fn uninstall_power(&self, power_id: &str) -> Result<(), String> {
        // In a real implementation, this would uninstall the power
        eprintln!("Uninstalling power: {}", power_id);
        Ok(())
    }

    pub fn get_metrics(&self) -> Result<MCPMetrics, String> {
        let servers = self.servers.lock().unwrap();
        let tools = self.tools.lock().unwrap();
        let status = self.server_status.lock().unwrap();
        let history = self.execution_history.lock().unwrap();

        let total_servers = servers.len();
        let enabled_servers = servers.values().filter(|s| s.enabled.unwrap_or(false)).count();
        let running_servers = status.values().filter(|s| s.running).count();
        let total_tools = tools.len();
        let total_executions = history.len() as u32;
        let successful_executions = history.iter().filter(|(_, _, success)| *success).count() as u32;
        let failed_executions = total_executions - successful_executions;

        let average_execution_time_ms = if !history.is_empty() {
            100.0 // Placeholder
        } else {
            0.0
        };

        Ok(MCPMetrics {
            total_servers,
            enabled_servers,
            running_servers,
            total_tools,
            total_executions,
            successful_executions,
            failed_executions,
            average_execution_time_ms,
        })
    }

    pub fn clear_tools(&self) -> Result<(), String> {
        let mut tools = self.tools.lock().unwrap();
        tools.clear();
        Ok(())
    }

    pub fn build_tool_prompt(&self) -> String {
        let tools = self.get_tools().unwrap_or_default();
        if tools.is_empty() {
            return String::new();
        }

        let tool_list = tools
            .iter()
            .map(|t| format!("- {} ({}): {}", t.name, t.server_name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"
<mcp_tools>
The following additional tools are available via connected MCP servers:
{}

To call an MCP tool, use:
{{"tool": "mcp_call", "toolName": "<tool_name>", "args": {{...}}}}
</mcp_tools>
"#,
            tool_list
        )
    }
}

impl Default for MCPService {
    fn default() -> Self {
        Self::new()
    }
}

// Tauri Commands

#[tauri::command]
pub fn mcp_initialize(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.initialize()
}

#[tauri::command]
pub fn mcp_add_server(
    config: MCPServerConfig,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.add_server(config)
}

#[tauri::command]
pub fn mcp_remove_server(
    name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.remove_server(&name)
}

#[tauri::command]
pub fn mcp_enable_server(
    name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.enable_server(&name)
}

#[tauri::command]
pub fn mcp_disable_server(
    name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.disable_server(&name)
}

#[tauri::command]
pub fn mcp_get_servers(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<MCPServerConfig>, String> {
    let service = state.lock().unwrap();
    service.get_servers()
}

#[tauri::command]
pub fn mcp_get_server_status(
    name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<MCPServerStatus, String> {
    let service = state.lock().unwrap();
    service.get_server_status(&name)
}

#[tauri::command]
pub fn mcp_get_all_server_status(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<MCPServerStatus>, String> {
    let service = state.lock().unwrap();
    service.get_all_server_status()
}

#[tauri::command]
pub fn mcp_register_tool(
    tool: MCPToolDefinition,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.register_tool(tool)
}

#[tauri::command]
pub fn mcp_get_tools(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<MCPToolDefinition>, String> {
    let service = state.lock().unwrap();
    service.get_tools()
}

#[tauri::command]
pub fn mcp_get_tools_by_server(
    server_name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<MCPToolDefinition>, String> {
    let service = state.lock().unwrap();
    service.get_tools_by_server(&server_name)
}

#[tauri::command]
pub fn mcp_call_tool(
    tool_name: String,
    args: serde_json::Value,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<serde_json::Value, String> {
    let service = state.lock().unwrap();
    service.call_tool(&tool_name, args)
}

#[tauri::command]
pub fn mcp_get_marketplace(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<PowerMarketplaceItem>, String> {
    let service = state.lock().unwrap();
    service.get_marketplace()
}

#[tauri::command]
pub fn mcp_install_power(
    power_id: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.install_power(&power_id)
}

#[tauri::command]
pub fn mcp_uninstall_power(
    power_id: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.uninstall_power(&power_id)
}

#[tauri::command]
pub fn mcp_add_configuration(
    config: MCPConfiguration,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.add_configuration(config)
}

#[tauri::command]
pub fn mcp_get_configurations(
    server_name: String,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<Vec<MCPConfiguration>, String> {
    let service = state.lock().unwrap();
    service.get_configurations(&server_name)
}

#[tauri::command]
pub fn mcp_validate_configuration(
    config: MCPConfiguration,
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<bool, String> {
    let service = state.lock().unwrap();
    service.validate_configuration(&config)
}

#[tauri::command]
pub fn mcp_get_metrics(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<MCPMetrics, String> {
    let service = state.lock().unwrap();
    service.get_metrics()
}

#[tauri::command]
pub fn mcp_clear_tools(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.clear_tools()
}

#[tauri::command]
pub fn mcp_build_tool_prompt(
    state: State<'_, Arc<Mutex<MCPService>>>,
) -> String {
    let service = state.lock().unwrap();
    service.build_tool_prompt()
}
