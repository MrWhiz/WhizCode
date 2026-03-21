use serde::{Deserialize, Serialize};
use crate::error::Result;

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AgentLoopResponse {
    pub response: String,
    pub steps: Vec<AgentStep>,
    pub tool_calls: Vec<ToolCall>,
}

pub struct AgentOrchestrator {
    max_iterations: u32,
    #[allow(dead_code)]
    conversation_history: Vec<(String, String)>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            max_iterations: 10,
            conversation_history: Vec::new(),
        }
    }

    pub async fn execute_task(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
    ) -> Result<AgentLoopResponse> {
        let _provider = model.get("provider").and_then(|p| p.as_str()).unwrap_or("ollama");
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut current_response = String::new();
        let mut all_tool_calls = Vec::new();

        // System prompt for tool-using agent
        let system_prompt = self.get_system_prompt(&workspace_path, &active_file);

        // Initial user message
        let mut messages = vec![
            ("system".to_string(), system_prompt),
            ("user".to_string(), task.clone()),
        ];

        // Agent loop - iterate until max iterations or agent says it's done
        while iteration < self.max_iterations {
            iteration += 1;
            eprintln!("[Agent] Iteration {}/{}", iteration, self.max_iterations);

            // Call LLM
            let response = self.call_llm(&messages, model_name).await?;
            current_response = response.clone();

            // Extract tool calls
            let tool_calls = extract_tool_calls(&response);

            if tool_calls.is_empty() {
                // No tool calls - agent is done
                eprintln!("[Agent] No tool calls, agent is done");
                steps.push(AgentStep {
                    iteration,
                    tool: "reasoning".to_string(),
                    status: "done".to_string(),
                    summary: "Agent completed reasoning".to_string(),
                    result: Some(response.clone()),
                });
                break;
            }

            // Execute each tool call
            for tool_call in &tool_calls {
                eprintln!("[Agent] Executing tool: {}", tool_call.tool);

                let tool_result = self.execute_tool(tool_call, &workspace_path).await;

                let step = AgentStep {
                    iteration,
                    tool: tool_call.tool.clone(),
                    status: if tool_result.is_ok() { "done".to_string() } else { "failed".to_string() },
                    summary: format!("Executed {} with args: {}", tool_call.tool, tool_call.args),
                    result: tool_result.as_ref().ok().map(|s| s.clone()),
                };

                steps.push(step);
                all_tool_calls.push(tool_call.clone());

                // Add tool result to conversation
                if let Ok(result) = tool_result {
                    messages.push(("assistant".to_string(), response.clone()));
                    messages.push(("user".to_string(), format!("Tool result:\n{}", result)));
                }
            }

            // Check if we should continue
            if iteration >= self.max_iterations {
                eprintln!("[Agent] Max iterations reached");
                break;
            }
        }

        Ok(AgentLoopResponse {
            response: current_response,
            steps,
            tool_calls: all_tool_calls,
        })
    }

    pub async fn execute_tools_parallel(
        &self,
        tool_calls: Vec<ToolCall>,
        workspace_path: &Option<String>,
    ) -> Result<Vec<AgentStep>> {
        let mut steps = Vec::new();
        
        for tool_call in tool_calls {
            let result = self.execute_tool(&tool_call, workspace_path).await;
            
            steps.push(AgentStep {
                iteration: 0,
                tool: tool_call.tool.clone(),
                status: if result.is_ok() { "done".to_string() } else { "failed".to_string() },
                summary: format!("Executed {} with args: {}", tool_call.tool, tool_call.args),
                result: result.ok(),
            });
        }
        
        Ok(steps)
    }

    async fn call_llm(&self, messages: &[(String, String)], model: &str) -> Result<String> {
        // Build prompt from messages
        let mut prompt = String::new();
        for (role, content) in messages {
            prompt.push_str(&format!("{}: {}\n\n", role, content));
        }

        eprintln!("[LLM] Calling {} with prompt length: {}", model, prompt.len());

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "temperature": 0.7,
            "top_p": 0.9,
            "top_k": 40,
        });

        match client
            .post("http://localhost:11434/api/generate")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if let Some(response_text) = data.get("response").and_then(|r| r.as_str()) {
                        return Ok(response_text.to_string());
                    }
                }
                Err("Failed to parse LLM response".into())
            }
            Err(e) => Err(format!("Failed to connect to LLM: {}", e).into()),
        }
    }

    async fn execute_tool(&self, tool_call: &ToolCall, _workspace_path: &Option<String>) -> Result<String> {
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
                
                // Simple file search
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
                
                tokio::fs::write(path, content).await?;
                Ok(format!("Successfully wrote to {}", path))
            }
            "run_command" => {
                let command = tool_call.args.get("command")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing command argument")?;
                
                // Parse command and args
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.is_empty() {
                    return Err("Empty command".into());
                }
                
                let output = tokio::process::Command::new(parts[0])
                    .args(&parts[1..])
                    .output()
                    .await?;
                
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                Ok(format!("Command output:\n{}\n{}", stdout, stderr))
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
                        let output = tokio::process::Command::new("git")
                            .arg("status")
                            .arg("--porcelain")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "add" => {
                        let path = tool_call.args.get("path").and_then(|p| p.as_str()).ok_or("Missing path")?;
                        let output = tokio::process::Command::new("git")
                            .arg("add")
                            .arg(path)
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "commit" => {
                        let message = tool_call.args.get("message").and_then(|m| m.as_str()).ok_or("Missing message")?;
                        let output = tokio::process::Command::new("git")
                            .arg("commit")
                            .arg("-m")
                            .arg(message)
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "push" => {
                        let output = tokio::process::Command::new("git")
                            .arg("push")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "pull" => {
                        let output = tokio::process::Command::new("git")
                            .arg("pull")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "log" => {
                        let output = tokio::process::Command::new("git")
                            .arg("log")
                            .arg("--oneline")
                            .arg("-10")
                            .output()
                            .await?;
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
                        let output = tokio::process::Command::new("npm")
                            .arg("install")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "add" => {
                        let package = tool_call.args.get("package").and_then(|p| p.as_str()).ok_or("Missing package")?;
                        let output = tokio::process::Command::new("npm")
                            .arg("install")
                            .arg(package)
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "list" => {
                        let output = tokio::process::Command::new("npm")
                            .arg("list")
                            .arg("--depth=0")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "run" => {
                        let script = tool_call.args.get("script").and_then(|s| s.as_str()).ok_or("Missing script")?;
                        let output = tokio::process::Command::new("npm")
                            .arg("run")
                            .arg(script)
                            .output()
                            .await?;
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
                        let output = tokio::process::Command::new("docker")
                            .arg("ps")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "images" => {
                        let output = tokio::process::Command::new("docker")
                            .arg("images")
                            .output()
                            .await?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                    "logs" => {
                        let container = tool_call.args.get("container").and_then(|c| c.as_str()).ok_or("Missing container")?;
                        let output = tokio::process::Command::new("docker")
                            .arg("logs")
                            .arg(container)
                            .output()
                            .await?;
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
        let mut prompt = r#"You are an AI coding assistant with access to tools. Your goal is to help the user accomplish their task.

Available tools:
- read_file: Read the contents of a file. Args: {"path": "file_path"}
- write_file: Write content to a file. Args: {"path": "file_path", "content": "file_content"}
- edit_file: Edit specific lines in a file. Args: {"path": "file_path", "start_line": 1, "end_line": 10, "content": "new_content"}
- list_directory: List files in a directory. Args: {"path": "directory_path"}
- search_files: Search for files matching a pattern. Args: {"path": "directory_path", "pattern": "search_pattern"}
- run_command: Run a shell command. Args: {"command": "command_string"}
- git: Git operations. Args: {"operation": "status|add|commit|push|pull|log", "path": "file_path", "message": "commit_message"}
- npm: NPM operations. Args: {"operation": "install|add|list|run", "package": "package_name", "script": "script_name"}
- docker: Docker operations. Args: {"operation": "ps|images|logs|run", "container": "container_name", "image": "image_name"}

When you need to use a tool, output it as JSON on a single line:
{"tool": "tool_name", "args": {"arg1": "value1", "arg2": "value2"}}

You can use multiple tools in one response. After using tools, analyze the results and decide if you need more tools or if you're done.

When you're done with the task, provide a summary of what you did."#.to_string();

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
pub async fn execute_agent_loop(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
) -> Result<AgentLoopResponse> {
    let mut orchestrator = AgentOrchestrator::new();
    orchestrator.execute_task(task, model, workspace_path, active_file).await
}
