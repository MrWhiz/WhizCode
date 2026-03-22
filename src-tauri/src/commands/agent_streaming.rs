use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use tauri::Emitter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamToken {
    pub token: String,
    pub iteration: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentStep {
    pub iteration: u32,
    pub tool: String,
    pub status: String,
    pub summary: String,
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamingAgentResponse {
    pub response: String,
    pub steps: Vec<AgentStep>,
    pub tool_calls: Vec<ToolCall>,
    pub total_tokens: u32,
}

pub struct StreamingAgentOrchestrator {
    max_iterations: u32,
    #[allow(dead_code)]
    conversation_history: Vec<(String, String)>,
    app_handle: Option<tauri::AppHandle>,
}

impl StreamingAgentOrchestrator {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            max_iterations: 10,
            conversation_history: Vec::new(),
            app_handle,
        }
    }

    pub async fn execute_task_streaming(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
    ) -> Result<StreamingAgentResponse> {
        let _provider = model.get("provider").and_then(|p| p.as_str()).unwrap_or("ollama");
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut current_response = String::new();
        let mut all_tool_calls = Vec::new();
        let mut total_tokens = 0u32;

        let system_prompt = self.get_system_prompt(&workspace_path, &active_file);
        let mut messages = vec![
            ("system".to_string(), system_prompt),
            ("user".to_string(), task.clone()),
        ];

        while iteration < self.max_iterations {
            // Check if agent was cancelled
            if crate::commands::agent::is_agent_cancelled() {
                eprintln!("[Agent] Execution cancelled by user");
                steps.push(AgentStep {
                    iteration,
                    tool: "cancelled".to_string(),
                    status: "cancelled".to_string(),
                    summary: "Agent execution was cancelled by user".to_string(),
                    result: None,
                    logs: None,
                });
                break;
            }

            iteration += 1;
            eprintln!("[Agent] Iteration {}/{}", iteration, self.max_iterations);

            let (response, tokens) = self.call_llm_streaming(&messages, model_name).await?;
            current_response = response.clone();
            total_tokens += tokens;

            let tool_calls = extract_tool_calls(&response);

            if tool_calls.is_empty() {
                eprintln!("[Agent] No tool calls, agent is done");
                steps.push(AgentStep {
                    iteration,
                    tool: "reasoning".to_string(),
                    status: "done".to_string(),
                    summary: "Agent completed reasoning".to_string(),
                    result: Some(response.clone()),
                    logs: Some(vec![response.clone()]),
                });
                break;
            }

            for tool_call in &tool_calls {
                eprintln!("[Agent] Executing tool: {}", tool_call.tool);

                let tool_result = self.execute_tool(tool_call, &workspace_path).await;

                let step = AgentStep {
                    iteration,
                    tool: tool_call.tool.clone(),
                    status: if tool_result.is_ok() { "done".to_string() } else { "failed".to_string() },
                    summary: format!("Executed {} with args: {}", tool_call.tool, tool_call.args),
                    result: tool_result.as_ref().ok().map(|s| s.clone()),
                    logs: tool_result.as_ref().ok().map(|s| vec![s.clone()]),
                };

                // Emit tool step event for real-time UI update
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:step", &step);
                }

                steps.push(step);
                all_tool_calls.push(tool_call.clone());

                if let Ok(result) = tool_result {
                    messages.push(("assistant".to_string(), response.clone()));
                    messages.push(("user".to_string(), format!("Tool result:\n{}", result)));
                }
            }

            if iteration >= self.max_iterations {
                eprintln!("[Agent] Max iterations reached");
                break;
            }
        }

        Ok(StreamingAgentResponse {
            response: current_response,
            steps,
            tool_calls: all_tool_calls,
            total_tokens,
        })
    }

    async fn call_llm_streaming(&self, messages: &[(String, String)], model: &str) -> Result<(String, u32)> {
        // Build the prompt from messages, ensuring system prompt is first
        let mut prompt = String::new();
        
        for (role, content) in messages {
            if role == "system" {
                prompt.push_str(&format!("{}\n\n", content));
            }
        }
        
        for (role, content) in messages {
            if role != "system" {
                prompt.push_str(&format!("[{}]\n{}\n\n", role.to_uppercase(), content));
            }
        }
        
        eprintln!("[LLM] Calling {} with prompt length: {}", model, prompt.len());

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": true,
            "temperature": 0.1,
            "top_p": 0.9,
            "top_k": 40,
        });

        let mut response_text = String::new();
        let mut token_count = 0u32;

        match client
            .post("http://localhost:11434/api/generate")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(response) => {
                match response.text().await {
                    Ok(text) => {
                        for line in text.lines() {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(token) = data.get("response").and_then(|r| r.as_str()) {
                                    response_text.push_str(token);
                                    token_count += 1;

                                    if let Some(app) = &self.app_handle {
                                        let _ = app.emit("agent:stream", StreamToken {
                                            token: token.to_string(),
                                            iteration: 0,
                                        });
                                    }
                                }
                            }
                        }
                        Ok((response_text, token_count))
                    }
                    Err(e) => Err(format!("Failed to read response: {}", e).into()),
                }
            }
            Err(e) => {
                eprintln!("Ollama connection error: {}", e);
                Err(format!("Failed to connect to LLM: {}", e).into())
            }
        }
    }

    async fn execute_tool(&self, tool_call: &ToolCall, workspace_path: &Option<String>) -> Result<String> {
        match tool_call.tool.as_str() {
            "read_file" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let content = tokio::fs::read_to_string(path).await?;
                Ok(format!("File contents:\n{}", content))
            }
            "list_directory" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let mut entries = Vec::new();
                let mut dir = tokio::fs::read_dir(path).await?;
                
                while let Some(entry) = dir.next_entry().await? {
                    let name = entry.file_name();
                    let is_dir = entry.metadata().await?.is_dir();
                    entries.push(format!("{}{}", name.to_string_lossy(), if is_dir { "/" } else { "" }));
                }
                
                entries.sort();
                Ok(format!("Directory contents:\n{}", entries.join("\n")))
            }
            "search_files" => {
                let pattern = tool_call.args.get("pattern")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing pattern argument")?;
                
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or(".");
                
                let mut results = Vec::new();
                if let Ok(mut dir) = tokio::fs::read_dir(path).await {
                    while let Ok(Some(entry)) = dir.next_entry().await {
                        let name = entry.file_name();
                        if name.to_string_lossy().contains(pattern) {
                            results.push(entry.path().to_string_lossy().to_string());
                        }
                    }
                }
                
                Ok(format!("Found {} files matching '{}'\n{}", results.len(), pattern, results.join("\n")))
            }
            "write_file" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let content = tool_call.args.get("content")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing content argument")?;
                
                // Create parent directories if they don't exist
                let file_path = std::path::Path::new(path);
                if let Some(parent) = file_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }
                
                tokio::fs::write(path, content).await?;
                Ok(format!("Successfully wrote to {}", path))
            }
            "run_command" => {
                let command = tool_call.args.get("command")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing command argument")?;
                
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.is_empty() {
                    return Err("Empty command".into());
                }
                
                let mut cmd = tokio::process::Command::new(parts[0]);
                cmd.args(&parts[1..]);
                
                // Set working directory to workspace if available
                if let Some(ws) = workspace_path {
                    cmd.current_dir(&ws);
                }
                
                // Add timeout for command execution
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    cmd.output()
                ).await {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let status = output.status;
                        
                        if !status.success() {
                            Err(format!("Command failed with status {}:\nStdout: {}\nStderr: {}", status, stdout, stderr).into())
                        } else if stdout.is_empty() && stderr.is_empty() {
                            Ok(format!("Command executed successfully"))
                        } else {
                            let mut result = String::new();
                            if !stdout.is_empty() {
                                result.push_str(&format!("Output:\n{}\n", stdout));
                            }
                            if !stderr.is_empty() {
                                result.push_str(&format!("Warnings/Info:\n{}", stderr));
                            }
                            Ok(result)
                        }
                    }
                    Ok(Err(e)) => Err(format!("Command execution failed: {}", e).into()),
                    Err(_) => Err("Command execution timed out after 30 seconds".into()),
                }
            }
            "edit_file" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let content = tool_call.args.get("content")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing content argument")?;
                
                let start_line = tool_call.args.get("start_line").and_then(|s| s.as_u64()).map(|s| s as u32);
                let end_line = tool_call.args.get("end_line").and_then(|e| e.as_u64()).map(|e| e as u32);
                
                let file_content = tokio::fs::read_to_string(path).await?;
                let lines: Vec<&str> = file_content.lines().collect();
                
                let start = start_line.unwrap_or(1) as usize;
                let end = end_line.unwrap_or(lines.len() as u32) as usize;
                
                let mut new_lines = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    let line_num = i + 1;
                    if line_num >= start && line_num <= end {
                        if line_num == start {
                            new_lines.push(content.to_string());
                        }
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                
                let new_content = new_lines.join("\n");
                tokio::fs::write(path, &new_content).await?;
                Ok(format!("Successfully edited {} (lines {}-{})", path, start, end))
            }
            "git" => {
                let operation = tool_call.args.get("operation")
                    .and_then(|o| o.as_str())
                    .ok_or("Missing operation argument")?;
                
                let output = match operation {
                    "status" => {
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("status").arg("--porcelain");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "add" => {
                        let path = tool_call.args.get("path").and_then(|p| p.as_str()).ok_or("Missing path")?;
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("add").arg(path);
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "commit" => {
                        let message = tool_call.args.get("message").and_then(|m| m.as_str()).ok_or("Missing message")?;
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("commit").arg("-m").arg(message);
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "push" => {
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("push");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "pull" => {
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("pull");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "log" => {
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.arg("log").arg("--oneline").arg("-10");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    _ => return Err(format!("Unknown git operation: {}", operation).into()),
                };
                Ok(output)
            }
            "npm" => {
                let operation = tool_call.args.get("operation")
                    .and_then(|o| o.as_str())
                    .ok_or("Missing operation argument")?;
                
                let output = match operation {
                    "install" => {
                        let mut cmd = tokio::process::Command::new("npm");
                        cmd.arg("install");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "add" => {
                        let package = tool_call.args.get("package").and_then(|p| p.as_str()).ok_or("Missing package")?;
                        let mut cmd = tokio::process::Command::new("npm");
                        cmd.arg("install").arg(package);
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "list" => {
                        let mut cmd = tokio::process::Command::new("npm");
                        cmd.arg("list").arg("--depth=0");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "run" => {
                        let script = tool_call.args.get("script").and_then(|s| s.as_str()).ok_or("Missing script")?;
                        let mut cmd = tokio::process::Command::new("npm");
                        cmd.arg("run").arg(script);
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    _ => return Err(format!("Unknown npm operation: {}", operation).into()),
                };
                Ok(output)
            }
            "docker" => {
                let operation = tool_call.args.get("operation")
                    .and_then(|o| o.as_str())
                    .ok_or("Missing operation argument")?;
                
                let output = match operation {
                    "ps" => {
                        let mut cmd = tokio::process::Command::new("docker");
                        cmd.arg("ps");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "images" => {
                        let mut cmd = tokio::process::Command::new("docker");
                        cmd.arg("images");
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "logs" => {
                        let container = tool_call.args.get("container").and_then(|c| c.as_str()).ok_or("Missing container")?;
                        let mut cmd = tokio::process::Command::new("docker");
                        cmd.arg("logs").arg(container);
                        if let Some(ws) = workspace_path {
                            cmd.current_dir(&ws);
                        }
                        let output = cmd.output().await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    _ => return Err(format!("Unknown docker operation: {}", operation).into()),
                };
                Ok(output)
            }
            _ => Err(format!("Unknown tool: {}", tool_call.tool).into()),
        }
    }

    fn get_system_prompt(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
        let mut prompt = prompts::KIRO_SYSTEM_PROMPT.to_string();

        if let Some(ws) = workspace_path {
            prompt.push_str(&format!("\n\nCurrent workspace: {}", ws));
        }

        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                prompt.push_str(&format!("\nActive file: {}", path));
            }
        }

        prompt
    }
}

fn extract_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();
    
    eprintln!("[EXTRACT] Response length: {}", response.len());
    eprintln!("[EXTRACT] Response preview: {}", &response[..std::cmp::min(200, response.len())]);
    
    let lines: Vec<&str> = response.lines().collect();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
            eprintln!("[EXTRACT] Found potential tool call: {}", trimmed);
            if let Ok(call) = serde_json::from_str::<ToolCall>(trimmed) {
                eprintln!("[EXTRACT] Successfully parsed tool call: {}", call.tool);
                tool_calls.push(call);
            } else {
                eprintln!("[EXTRACT] Failed to parse as JSON");
            }
        }
    }
    
    eprintln!("[EXTRACT] Total tool calls found: {}", tool_calls.len());
    tool_calls
}

#[tauri::command]
pub async fn execute_agent_loop_streaming(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    app_handle: tauri::AppHandle,
) -> Result<StreamingAgentResponse> {
    let mut orchestrator = StreamingAgentOrchestrator::new(Some(app_handle));
    orchestrator.execute_task_streaming(task, model, workspace_path, active_file).await
}
