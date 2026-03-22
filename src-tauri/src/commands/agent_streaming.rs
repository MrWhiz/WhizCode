use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use tauri::Emitter;
use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::state::AppState;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamingAgentResponse {
    pub response: String,
    pub steps: Vec<AgentStep>,
    pub tool_calls: Vec<ToolCall>,
    pub total_tokens: u32,
    pub status: String,
}

pub struct StreamingAgentOrchestrator {
    max_iterations: u32,
    #[allow(dead_code)]
    conversation_history: Vec<(String, String)>,
    app_handle: Option<tauri::AppHandle>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct PlannedSubTask {
    pub id: String,
    pub agent: String,
    pub description: String,
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
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    ) -> Result<StreamingAgentResponse> {
        let _provider = model.get("provider").and_then(|p| p.as_str()).unwrap_or("ollama");
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        // Log what workspace path was received for debugging
        eprintln!("[Backend] Received workspace_path: {:?}", workspace_path);

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut all_tool_calls = Vec::new();
        let mut total_tokens = 0u32;
        let mut status = "done".to_string();

        // --- Phase 0: Strategic Planning ---
        let mut persona = "planner".to_string();
        let planner_prompt = crate::commands::prompts::STRATEGIC_PLANNER_PROMPT.to_string();
        let planning_msg = vec![
            ("system".to_string(), planner_prompt),
            ("user".to_string(), format!("Generate a plan for this task in the workspace:\n{}", task)),
        ];
        
        // Emit planning start
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:persona", &persona);
        }

        let (plan_response, tokens) = self.call_llm_streaming(&planning_msg, model_name).await?;
        total_tokens += tokens;
        
        let sub_tasks = self.parse_plan_json(&plan_response);
        eprintln!("[Orchestrator] Generated {} sub-tasks", sub_tasks.len());

        let mut turn_messages = vec![
            ("system".to_string(), self.get_system_prompt(&workspace_path, &active_file)),
            ("user".to_string(), format!("Plan:\n{}\n\nStarting Task: {}", plan_response, task)),
        ];

        // --- Multi-Phase Execution Loop ---
        for sub_task in sub_tasks {
            persona = sub_task.agent.clone();
            eprintln!("[Orchestrator] Switching to Persona: {}", persona);
            
            if let Some(app) = &self.app_handle {
                let _ = app.emit("agent:persona", &persona);
            }

            // Update system prompt based on persona
            let persona_prompt = match persona.as_str() {
                "researcher" => crate::commands::prompts::RESEARCHER_PROMPT.to_string(),
                "executor" => crate::commands::prompts::EXECUTOR_PROMPT.to_string(),
                "reviewer" => crate::commands::prompts::REVIEWER_PROMPT.to_string(),
                _ => crate::commands::prompts::RESEARCHER_PROMPT.to_string(),
            };

            turn_messages[0] = ("system".to_string(), format!("{}\n\nYour current focus: {}\nProject Context:\n{}", 
                persona_prompt, sub_task.description, self.get_system_prompt(&workspace_path, &active_file)));
            
            // Loop for this specific phase
            let mut phase_iterations = 0;
            while phase_iterations < 5 && iteration < self.max_iterations {
                if crate::commands::agent::is_agent_cancelled() { break; }
                
                iteration += 1;
                phase_iterations += 1;
                eprintln!("[Agent] Iteration {}/{} (Phase: {})", iteration, self.max_iterations, persona);

                let (response, tokens) = self.call_llm_streaming(&turn_messages, model_name).await?;
                total_tokens += tokens;

                let tool_calls = extract_tool_calls(&response);

                if tool_calls.is_empty() {
                    steps.push(AgentStep {
                        iteration,
                        tool: "reasoning".to_string(),
                        status: "done".to_string(),
                        summary: format!("[{}] {}", persona.to_uppercase(), "Completed phase reasoning"),
                        result: Some(response.clone()),
                        logs: Some(vec![response.clone()]),
                        persona: Some(persona.clone()),
                        request_id: None,
                    });
                    break;
                }

                for tool_call in &tool_calls {
                    eprintln!("[Agent] Executing tool: {}", tool_call.tool);
                    let tool_result = self.execute_tool(tool_call, &workspace_path, &vector_system, &code_intel).await;

                    let step = AgentStep {
                        iteration,
                        tool: tool_call.tool.clone(),
                        status: if tool_result.is_ok() { "done".to_string() } else { "failed".to_string() },
                        summary: format!("[{}] {}", persona.to_uppercase(), tool_call.tool),
                        result: tool_result.as_ref().ok().map(|s| s.clone()),
                        logs: tool_result.as_ref().ok().map(|s| vec![s.clone()]),
                        persona: Some(persona.clone()),
                        request_id: None,
                    };

                    if let Some(app) = &self.app_handle {
                        let _ = app.emit("agent:step", &step);
                    }

                    steps.push(step);
                    all_tool_calls.push(tool_call.clone());

                    if let Ok(result) = tool_result {
                        turn_messages.push(("assistant".to_string(), response.clone()));
                        turn_messages.push(("user".to_string(), format!("Tool result:\n{}", result)));
                    }

                    if tool_call.tool == "done" {
                        eprintln!("[Agent] Phase complete via done tool");
                        break;
                    }
                }
                
                if tool_calls.iter().any(|c| c.tool == "done") { break; }
            }
            
            // If we reached max iterations across phases, stop everything
            if iteration >= self.max_iterations {
                eprintln!("[Agent] Global max iterations reached ({})", iteration);
                status = "max_iterations_reached".to_string();
                break;
            }
        }

        Ok(StreamingAgentResponse {
            response: turn_messages.last().map(|m| m.1.clone()).unwrap_or_else(|| "Task processed with no final message.".to_string()),
            steps,
            tool_calls: all_tool_calls,
            total_tokens,
            status,
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

    async fn execute_tool(
        &self, 
        tool_call: &ToolCall, 
        workspace_path: &Option<String>,
        vector_system: &Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: &Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    ) -> Result<String> {
        let ws_root = workspace_path.as_deref().unwrap_or(".");
        let resolve = |p: &str| {
            let path = std::path::Path::new(p);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::path::Path::new(ws_root).join(p)
            }
        };

        match tool_call.tool.as_str() {
            "read_file" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let content = tokio::fs::read_to_string(resolve(path)).await?;
                Ok(format!("File contents:\n{}", content))
            }
            "list_directory" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path argument")?;
                
                let mut entries = Vec::new();
                let mut dir = tokio::fs::read_dir(resolve(path)).await?;
                
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
                if let Ok(mut dir) = tokio::fs::read_dir(resolve(path)).await {
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
                let resolved_path = resolve(path);
                if let Some(parent) = resolved_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }
                
                tokio::fs::write(&resolved_path, content).await?;
                Ok(format!("Successfully wrote to {}", path))
            }
            "run_command" => {
                let command = tool_call.args.get("command")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing command argument")?;
                
                let cwd = workspace_path.as_deref().unwrap_or(".");
                eprintln!("[run_command] Executing: {:?} in {:?}", command, cwd);

                // Always run via system shell so PATH is resolved (npm, npx, git, etc.)
                #[cfg(target_os = "windows")]
                let mut cmd = {
                    let mut c = tokio::process::Command::new("cmd");
                    c.args(["/C", command]);
                    c
                };

                #[cfg(not(target_os = "windows"))]
                let mut cmd = {
                    let mut c = tokio::process::Command::new("sh");
                    c.args(["-c", command]);
                    c
                };

                cmd.current_dir(cwd);
                cmd.env("FORCE_COLOR", "0");
                // Set CI=true so npm/npx/yarn never prompt for user input
                cmd.env("CI", "true");
                cmd.env("NPM_CONFIG_YES", "true");
                // Pipe stdin so any remaining prompts receive 'y'
                cmd.stdin(std::process::Stdio::piped());
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());

                // 5 minute timeout for long-running commands like npx create-react-app
                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => return Err(format!("Failed to spawn command '{}': {}", command, e).into()),
                };

                // Write 'y\n' to stdin to answer any confirmation prompts
                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    let _ = stdin.write_all(b"y\n").await;
                }

                match tokio::time::timeout(
                    std::time::Duration::from_secs(300),
                    child.wait_with_output()
                ).await {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let success = output.status.success();

                        eprintln!("[run_command] Exit: {}, stdout len: {}, stderr len: {}", output.status, stdout.len(), stderr.len());

                        let mut result = format!("Command: {}\nCWD: {}\n", command, cwd);
                        if !stdout.is_empty() {
                            result.push_str(&format!("Output:\n{}\n", stdout));
                        }
                        if !stderr.is_empty() {
                            result.push_str(&format!("Stderr:\n{}\n", stderr));
                        }
                        if stdout.is_empty() && stderr.is_empty() {
                            result.push_str("Command completed with no output.\n");
                        }

                        if success {
                            Ok(result)
                        } else {
                            Err(format!("Command failed (exit: {}):\n{}", output.status, result).into())
                        }
                    }
                    Ok(Err(e)) => Err(format!("Failed to spawn command '{}': {}", command, e).into()),
                    Err(_) => Err(format!("Command '{}' timed out after 5 minutes", command).into()),
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
                
                let resolved_path = resolve(path);
                let file_content = tokio::fs::read_to_string(&resolved_path).await?;
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
                tokio::fs::write(&resolved_path, &new_content).await?;
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
            "semantic_search" => {
                let query = tool_call.args.get("query")
                    .and_then(|q| q.as_str())
                    .ok_or("Missing query argument")?;
                
                let ws_root = workspace_path.as_deref().unwrap_or(".");
                let search_query = crate::commands::vector_search::SemanticQuery {
                    query: query.to_string(),
                    file_path: None,
                    limit: Some(5),
                };
                
                let results = {
                    let system = vector_system.lock().unwrap();
                    // Ensure it's indexed (basic lazy indexing)
                    let stats = system.get_index_stats().unwrap();
                    if stats.total_chunks == 0 {
                        let _ = system.index_workspace(ws_root);
                    }
                    system.semantic_search(&search_query).map_err(|e| format!("Search failed: {}", e))?
                };
                
                let mut out = format!("Found {} relevant code blocks for '{}':\n", results.len(), query);
                for res in results {
                    out.push_str(&format!("\n--- {} (relevance: {:.2}) ---\n{}\n", res.chunk.file_path, res.relevance_score, res.chunk.content));
                }
                Ok(out)
            }
            "find_symbols" => {
                let query = tool_call.args.get("query")
                    .and_then(|q| q.as_str())
                    .ok_or("Missing query argument")?;
                
                let ws_root = workspace_path.as_deref().unwrap_or(".");
                let intel = code_intel.lock().unwrap();
                
                // Lazy analyze if needed
                let symbols = intel.get_all_symbols(ws_root);
                if symbols.is_empty() {
                    // Try to analyze
                    let _ = intel.analyze_workspace(ws_root.to_string());
                }
                
                let results: Vec<_> = intel.get_all_symbols(ws_root).into_iter()
                    .filter(|s| s.name.contains(query))
                    .collect();
                
                let mut out = format!("Found {} symbols matching '{}':\n", results.len(), query);
                for s in results {
                    out.push_str(&format!("- {} ({}): {} line {}\n", s.name, s.symbol_type, s.file_path, s.line_number));
                }
                Ok(out)
            }
            "get_code_intelligence" => {
                let path = tool_call.args.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                
                let ws_root = workspace_path.as_deref().unwrap_or(".");
                let intel = code_intel.lock().unwrap();
                
                let metrics = intel.get_code_metrics(ws_root);
                let mut out = format!("Code Intelligence for {}\n", ws_root);
                if let Some(m) = metrics {
                    out.push_str(&format!("Metrics: Complexity={:.2}, Debt={:.2}, Files={}, Symbols={}\n", 
                        m.average_complexity, m.technical_debt, m.total_files, m.total_symbols));
                }
                
                if !path.is_empty() {
                    let suggestions = intel.suggest_refactoring(ws_root, path);
                    out.push_str(&format!("\nRefactoring suggestions for {}:\n", path));
                    for s in suggestions {
                        out.push_str(&format!("- [{}] {}: {}\n", s.priority, s.recommendation, s.impact));
                    }
                }
                
                Ok(out)
            }
            "done" => {
                Ok("Task completed successfully.".to_string())
            }
            "search_web" => {
                let query = tool_call.args.get("query")
                    .and_then(|q| q.as_str())
                    .ok_or("Missing query argument")?;
                
                let results = crate::commands::web_search::search_web(query.to_string()).await?;
                let mut out = format!("Search results for '{}':\n", query);
                for (i, r) in results.iter().enumerate() {
                    out.push_str(&format!("{}. {} ({})\n   {}\n", i+1, r.title, r.url, r.snippet));
                }
                Ok(out)
            }
            "read_url_content" => {
                let url = tool_call.args.get("url")
                    .and_then(|u| u.as_str())
                    .ok_or("Missing url argument")?;
                
                let content = crate::commands::web_search::read_url_content(url.to_string()).await?;
                Ok(format!("Content from {}:\n\n{}", url, content))
            }
            "generate_image" => {
                let prompt = tool_call.args.get("prompt")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing prompt argument")?;
                
                let ws_root = workspace_path.as_deref().unwrap_or(".");
                let result = crate::commands::assets::generate_image(
                    crate::commands::assets::ImageRequest {
                        prompt: prompt.to_string(),
                        width: 1024,
                        height: 1024
                    },
                    ws_root.to_string()
                ).await?;
                
                Ok(format!("Generated image saved to {}. URL: {}", result.asset_path, result.url))
            }
            _ => Err(format!("Unknown tool: {}", tool_call.tool).into()),
        }
    }

    fn get_system_prompt(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
        let mut prompt = prompts::KIRO_SYSTEM_PROMPT.to_string();

        if let Some(ws) = workspace_path {
            prompt.push_str(&format!("\n\nCurrent workspace: {}", ws));
            
            // Inject Knowledge Items (The 'Brain')
            if let Ok(lore) = crate::commands::distillation::load_relevant_knowledge(std::path::Path::new(ws)) {
                if !lore.is_empty() {
                    prompt.push_str(&lore);
                }
            }
            
            // Inject Structured Workflows and Skills
            let workflows_context = crate::commands::workflows::get_workflows_context(std::path::Path::new(ws));
            if !workflows_context.is_empty() {
                prompt.push_str(&workflows_context);
            }
        }

        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                prompt.push_str(&format!("\nActive file: {}", path));
            }
        }

        prompt
    }

    #[allow(dead_code)]
    async fn ask_to_continue(&self, iteration: u32) -> bool {
        if let Some(app) = &self.app_handle {
            let (tx, rx) = tokio::sync::oneshot::channel();
            {
                let mut lock = crate::commands::agent::PERMISSION_TX.lock().unwrap();
                *lock = Some(tx);
            }

            let step = AgentStep {
                iteration,
                tool: "continue_iterations".to_string(),
                status: "awaiting_permission".to_string(),
                summary: format!("Maximum iterations ({}) reached. Continue for 10 more steps?", iteration),
                result: None,
                logs: None,
                persona: Some("orchestrator".to_string()),
                request_id: Some("iteration_limit".to_string()),
            };

            let _ = app.emit("agent:step", &step);

            rx.await.unwrap_or(false)
        } else {
            false
        }
    }

    fn parse_plan_json(&self, response: &str) -> Vec<PlannedSubTask> {
        // Extract JSON array from response
        if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                let json_slice = &response[start..=end];
                if let Ok(tasks) = serde_json::from_str::<Vec<PlannedSubTask>>(json_slice) {
                    return tasks;
                }
            }
        }
        
        // Fallback: Default single task if parsing fails
        vec![PlannedSubTask {
            id: "task_1".to_string(),
            agent: "executor".to_string(),
            description: "Complete the original task".to_string(),
        }]
    }
}

fn extract_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();
    
    eprintln!("[EXTRACT] Response length: {}", response.len());
    
    // Attempt to find JSON objects in the response
    // We look for anything that starts with { and ends with }
    let mut start_indices = Vec::new();
    for (i, c) in response.char_indices() {
        if c == '{' {
            start_indices.push(i);
        }
    }
    
    // For each start index, try to find a valid JSON object starting there
    for start in start_indices {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escaped = false;
        
        for (i, c) in response[start..].char_indices() {
            let actual_idx = start + i;
            
            if escaped {
                escaped = false;
                continue;
            }
            
            if c == '\\' {
                escaped = true;
                continue;
            }
            
            if c == '"' {
                in_string = !in_string;
                continue;
            }
            
            if !in_string {
                if c == '{' {
                    brace_count += 1;
                } else if c == '}' {
                    brace_count -= 1;
                    
                    if brace_count == 0 {
                        // Found a potential JSON block
                        let potential_json = &response[start..=actual_idx];
                        if potential_json.contains("\"tool\"") {
                            if let Ok(call) = serde_json::from_str::<ToolCall>(potential_json) {
                                // Filter: only add if the tool is one we support (or 'done')
                                let supported = [
                                    "read_file", "write_file", "edit_file", "list_directory", 
                                    "search_files", "run_command", "git", "npm", "docker", 
                                    "semantic_search", "analyze_workspace", "get_code_intelligence",
                                    "find_symbols", "search_web", "read_url_content", 
                                    "generate_image", "done"
                                ];
                                
                                if supported.contains(&call.tool.as_str()) {
                                    eprintln!("[EXTRACT] Successfully parsed tool call: {}", call.tool);
                                    tool_calls.push(call);
                                } else {
                                    eprintln!("[EXTRACT] Skipping unsupported tool: {}", call.tool);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    
    // Deduplicate tool calls by comparing their serialized form or just their content
    let mut unique_calls = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for call in tool_calls {
        let serialized = serde_json::to_string(&call).unwrap_or_default();
        if !seen.contains(&serialized) {
            seen.insert(serialized);
            unique_calls.push(call);
        }
    }
    
    eprintln!("[EXTRACT] Total tool calls found: {}", unique_calls.len());
    unique_calls
}

#[tauri::command]
pub async fn execute_agent_loop_streaming(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    vector_state: State<'_, Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>>,
    intel_state: State<'_, Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>>,
) -> Result<StreamingAgentResponse> {
    // Read workspace from AppState first (most reliable source)
    // Fall back to JS-provided path if AppState has none
    let resolved_workspace = {
        let app_state = state.read();
        app_state.get_workspace().map(|p| p.to_string_lossy().to_string())
    }.or(workspace_path);

    eprintln!("[Backend] Resolved workspace_path: {:?}", resolved_workspace);

    let mut orchestrator = StreamingAgentOrchestrator::new(Some(app_handle));
    orchestrator.execute_task_streaming(task, model, resolved_workspace, active_file, vector_state.inner().clone(), intel_state.inner().clone()).await
}
