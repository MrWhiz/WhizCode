use serde::{Deserialize, Serialize};
use crate::error::Result;
use std::sync::Arc;
use parking_lot::Mutex;

// Global cancellation token for agent execution
lazy_static::lazy_static! {
    static ref AGENT_CANCEL_TOKEN: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AgentTaskOptions {
    #[serde(default)]
    pub prompt: Option<String>,
    pub task: String,
    pub model: serde_json::Value,
    pub workspace_path: Option<String>,
    pub active_file: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub is_autopilot_mode: Option<bool>,
    pub images: Option<Vec<String>>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct AgentResponse {
    pub response: String,
    pub tool_calls: Vec<ToolCall>,
    pub steps: Vec<String>,
}

#[tauri::command]
pub async fn execute_agent_task(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    _config: Option<serde_json::Value>,
    _is_autopilot_mode: Option<bool>,
    _images: Option<Vec<String>>,
) -> Result<String> {
    // Get model info
    let provider = model.get("provider").and_then(|p| p.as_str()).unwrap_or("ollama");
    let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");
    
    eprintln!("Agent task: provider={}, model={}, task={}", provider, model_name, task);
    
    // Create a system prompt that instructs the model to use tools
    let system_prompt = r#"You are an AI coding assistant. You have access to the following tools:
- read_file: Read the contents of a file
- write_file: Write content to a file
- list_directory: List files in a directory
- search_files: Search for files matching a pattern
- run_command: Run a shell command
- edit_file: Edit a specific part of a file

When you need to perform a task, think step by step and use the appropriate tools.
Format your tool calls as JSON in this format:
{"tool": "tool_name", "args": {"arg1": "value1", "arg2": "value2"}}

Always provide explanations of what you're doing and why."#;

    // Build the full prompt with context
    let full_prompt = format!(
        "{}\n\nUser request: {}\n\nWorkspace: {:?}\nActive file: {:?}",
        system_prompt, task, workspace_path, active_file
    );
    
    match provider {
        "ollama" => {
            match call_ollama_with_tools(&full_prompt, model_name).await {
                Ok(response) => {
                    eprintln!("Ollama response: {}", response);
                    Ok(response)
                }
                Err(e) => {
                    eprintln!("Ollama error: {}", e);
                    Err(e)
                }
            }
        }
        _ => Err(format!("Unsupported model provider: {}", provider).into()),
    }
}

async fn call_ollama_with_tools(prompt: &str, model: &str) -> Result<String> {
    eprintln!("Calling Ollama with model: {}", model);
    
    let client = reqwest::Client::new();
    
    let payload = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "temperature": 0.7,
        "top_p": 0.9,
        "top_k": 40,
    });
    
    eprintln!("Ollama payload: {}", payload);
    
    match client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
    {
        Ok(response) => {
            eprintln!("Ollama response status: {}", response.status());
            if let Ok(data) = response.json::<serde_json::Value>().await {
                eprintln!("Ollama response data: {}", data);
                if let Some(response_text) = data.get("response").and_then(|r| r.as_str()) {
                    // Parse tool calls from the response
                    let tool_calls = extract_tool_calls(response_text);
                    
                    // If there are tool calls, format them in the response
                    if !tool_calls.is_empty() {
                        let mut result = response_text.to_string();
                        result.push_str("\n\n[TOOL_CALLS]\n");
                        for call in tool_calls {
                            result.push_str(&format!("{}\n", serde_json::to_string(&call).unwrap_or_default()));
                        }
                        return Ok(result);
                    }
                    
                    return Ok(response_text.to_string());
                }
            }
            Err("Failed to parse Ollama response".into())
        }
        Err(e) => {
            eprintln!("Ollama connection error: {}", e);
            Err(format!("Failed to connect to Ollama: {}", e).into())
        }
    }
}

fn extract_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();
    
    // Look for JSON objects that look like tool calls
    let lines: Vec<&str> = response.lines().collect();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
            if let Ok(call) = serde_json::from_str::<ToolCall>(trimmed) {
                tool_calls.push(call);
            }
        }
    }
    
    tool_calls
}

#[tauri::command]
pub async fn agent_stop() -> Result<()> {
    let mut cancel = AGENT_CANCEL_TOKEN.lock();
    *cancel = true;
    eprintln!("Agent stop requested");
    Ok(())
}

#[tauri::command]
pub async fn agent_reset() -> Result<()> {
    let mut cancel = AGENT_CANCEL_TOKEN.lock();
    *cancel = false;
    eprintln!("Agent reset");
    Ok(())
}

pub fn is_agent_cancelled() -> bool {
    *AGENT_CANCEL_TOKEN.lock()
}

#[tauri::command]
pub async fn agent_permission_response(_approved: bool, _request_id: Option<String>) -> Result<()> {
    Ok(())
}
