use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use tauri::Emitter;
use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::state::AppState;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
    suppress_stream: bool,
    file_tree_cache: Arc<RwLock<HashMap<String, (String, u64)>>>, // (workspace_path) -> (tree, timestamp)
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
            suppress_stream: false,
            file_tree_cache: Arc::new(RwLock::new(HashMap::new())),
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

        // --- Phase 0: Fast rule-based planning (no LLM call) ---
        let sub_tasks = self.build_plan_fast(&task);
        eprintln!("[Orchestrator] Fast plan: {} sub-tasks", sub_tasks.len());

        // Emit planning done immediately so UI shows activity
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "planning",
                "status": "completed",
                "description": format!("Plan ready: {} phases", sub_tasks.len())
            }));
            let _ = app.emit("agent:step", AgentStep {
                iteration: 0,
                tool: "planning".to_string(),
                status: "done".to_string(),
                summary: format!("Plan: {}", sub_tasks.iter().map(|t| t.agent.as_str()).collect::<Vec<_>>().join(" → ")),
                result: None,
                logs: None,
                persona: Some("planner".to_string()),
                request_id: None,
            });
        }

        let mut turn_messages = vec![
            ("system".to_string(), self.get_system_prompt(&workspace_path, &active_file)),
            ("user".to_string(), task.clone()),
        ];

        // --- Multi-Phase Execution Loop ---
        for sub_task in sub_tasks {
            let persona = sub_task.agent.clone();
            eprintln!("[Orchestrator] Switching to Persona: {}", persona);
            eprintln!("[Orchestrator] Task description: {}", sub_task.description);
            
            if let Some(app) = &self.app_handle {
                let _ = app.emit("agent:persona", &persona);
                let _ = app.emit("agent:phase", &serde_json::json!({
                    "phase": persona.clone(),
                    "status": "started",
                    "description": sub_task.description.clone()
                }));
            }

            // Update system prompt based on persona
            let persona_prompt = match persona.as_str() {
                "researcher" => crate::commands::prompts::RESEARCHER_PROMPT.to_string(),
                "executor" => crate::commands::prompts::EXECUTOR_PROMPT.to_string(),
                "reviewer" => crate::commands::prompts::REVIEWER_PROMPT.to_string(),
                _ => crate::commands::prompts::RESEARCHER_PROMPT.to_string(),
            };

            turn_messages[0] = ("system".to_string(), format!("{}\n\nYour current focus: {}", 
                persona_prompt, sub_task.description));
            eprintln!("[Orchestrator] System prompt updated for {} persona", persona);
            
            // Loop for this specific phase
            let mut phase_iterations = 0;
            while phase_iterations < 5 && iteration < self.max_iterations {
                if crate::commands::agent::is_agent_cancelled() { break; }
                
                iteration += 1;
                phase_iterations += 1;
                eprintln!("[Agent] === Iteration {}/{} (Phase: {}, Phase iteration: {}/{}) ===", iteration, self.max_iterations, persona, phase_iterations, 5);

                let (response, tokens) = self.call_llm_streaming(&turn_messages, model_name).await?;
                total_tokens += tokens;

                let mut tool_calls = extract_tool_calls(&response);
                eprintln!("[Agent] LLM response length: {}, extracted {} tool calls", response.len(), tool_calls.len());
                if tool_calls.is_empty() {
                    eprintln!("[Agent] LLM response (first 1000 chars): {}", &response[..response.len().min(1000)]);
                    eprintln!("[Agent] Full response: {}", response);
                }

                // If LLM gave natural language instead of tool calls, retry once with a hard correction
                if tool_calls.is_empty() && looks_like_natural_language(&response) {
                    eprintln!("[Agent] LLM gave natural language, retrying with correction...");
                    let mut correction_msgs = turn_messages.clone();
                    correction_msgs.push(("assistant".to_string(), response.clone()));
                    correction_msgs.push(("user".to_string(),
                        "ERROR: You output text instead of JSON tool calls.\n\
                         You MUST output ONLY raw JSON objects, one per line.\n\
                         Do not output any text, explanations, or markdown.\n\
                         Example of CORRECT output:\n\
                         {\"tool\": \"read_file\", \"args\": {\"path\": \"/workspace/file.txt\"}}\n\
                         {\"tool\": \"done\", \"args\": {}}\n\
                         Now output your tool calls (JSON only):".to_string()
                    ));
                    self.suppress_stream = true;
                    let (retry_response, retry_tokens) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    total_tokens += retry_tokens;
                    tool_calls = extract_tool_calls(&retry_response);
                    if tool_calls.is_empty() {
                        eprintln!("[Agent] Retry also gave no tool calls, treating as done");
                    }
                }

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
                    
                    // Emit phase completion
                    if let Some(app) = &self.app_handle {
                        let _ = app.emit("agent:phase", &serde_json::json!({
                            "phase": persona.clone(),
                            "status": "completed",
                            "description": format!("{} phase completed", persona)
                        }));
                    }
                    break;
                }

                // Push assistant response ONCE for all tool calls in this turn
                turn_messages.push(("assistant".to_string(), response.clone()));

                let mut tool_results = Vec::new();
                let mut done = false;

                // Identify independent tool groups for parallel execution
                let tool_groups = identify_independent_tool_groups(&tool_calls);
                eprintln!("[Agent] Tool groups for parallel execution: {} groups from {} tools", tool_groups.len(), tool_calls.len());

                // Execute each group (currently sequential, but structured for future parallelization)
                for (group_idx, group) in tool_groups.iter().enumerate() {
                    eprintln!("[Agent] Executing group {} with {} tools in parallel", group_idx + 1, group.len());
                    let group_start = std::time::Instant::now();

                    // Execute all tools in this group
                    for &tool_idx in group {
                        let tool_call = &tool_calls[tool_idx];
                        eprintln!("[Agent] Executing tool: {}", tool_call.tool);

                        if tool_call.tool == "done" {
                            eprintln!("[Agent] Phase complete via done tool");
                            done = true;
                            break;
                        }

                        let start_time = std::time::Instant::now();
                        let tool_result = self.execute_tool(tool_call, &workspace_path, &vector_system, &code_intel).await;
                        let elapsed = start_time.elapsed().as_millis();

                        let step = AgentStep {
                            iteration,
                            tool: tool_call.tool.clone(),
                            status: if tool_result.is_ok() { "done".to_string() } else { "failed".to_string() },
                            summary: format!("[{}] {} ({}ms)", persona.to_uppercase(), tool_call.tool, elapsed),
                            result: tool_result.as_ref().ok().cloned(),
                            logs: tool_result.as_ref().ok().map(|s| vec![s.clone()]),
                            persona: Some(persona.clone()),
                            request_id: None,
                        };

                        if let Some(app) = &self.app_handle {
                            let _ = app.emit("agent:step", &step);
                        }

                        steps.push(step);
                        all_tool_calls.push(tool_call.clone());

                        match tool_result {
                            Ok(result) => tool_results.push(format!("[{}] result:\n{}", tool_call.tool, result)),
                            Err(e)     => tool_results.push(format!("[{}] error:\n{}", tool_call.tool, e)),
                        }
                    }

                    let group_elapsed = group_start.elapsed().as_millis();
                    eprintln!("[Agent] Group {} completed in {}ms", group_idx + 1, group_elapsed);

                    if done {
                        break;
                    }
                }

                // Push all tool results as a single user message with explicit reminder
                if !tool_results.is_empty() {
                    let results_msg = format!(
                        "Tool results:\n{}\n\nContinue with more tool calls or output {{\"tool\": \"done\", \"args\": {{}}}} when finished.",
                        tool_results.join("\n\n")
                    );
                    turn_messages.push(("user".to_string(), results_msg));
                }

                if done { break; }
            }
            
            // If we reached max iterations across phases, stop everything
            if iteration >= self.max_iterations {
                eprintln!("[Agent] Global max iterations reached ({})", iteration);
                status = "max_iterations_reached".to_string();
                break;
            }
        }

        Ok(StreamingAgentResponse {
            response: {
                // Build a clean summary from completed steps instead of raw message content
                let done_steps: Vec<String> = steps.iter()
                    .filter(|s| s.status == "done" && s.tool != "planning" && s.tool != "reasoning")
                    .map(|s| format!("- {}", s.summary))
                    .collect();
                if done_steps.is_empty() {
                    "Task completed.".to_string()
                } else {
                    format!("Completed:\n{}", done_steps.join("\n"))
                }
            },
            steps,
            tool_calls: all_tool_calls,
            total_tokens,
            status,
        })
    }

    async fn call_llm_streaming(&self, messages: &[(String, String)], model: &str) -> Result<(String, u32)> {
        // Build prompt: system first, then conversation in order
        let mut prompt = String::new();
        
        for (role, content) in messages {
            match role.as_str() {
                "system" => prompt.push_str(&format!("{}\n\n", content)),
                "user"   => prompt.push_str(&format!("[USER]\n{}\n\n", content)),
                "assistant" => prompt.push_str(&format!("[ASSISTANT]\n{}\n\n", content)),
                _ => {}
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
            "num_predict": 2048,
        });

        let mut response_text = String::new();
        let mut token_count = 0u32;
        let mut token_batch = String::new();
        const BATCH_SIZE: usize = 10; // Emit every 10 tokens instead of every token

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
                                    token_batch.push_str(token);
                                    token_count += 1;

                                    // Emit batched tokens every BATCH_SIZE tokens
                                    if token_count % BATCH_SIZE as u32 == 0 {
                                        if let Some(app) = &self.app_handle {
                                            if !self.suppress_stream {
                                                let _ = app.emit("agent:stream", StreamToken {
                                                    token: token_batch.clone(),
                                                    iteration: 0,
                                                });
                                                token_batch.clear();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Emit any remaining tokens
                        if !token_batch.is_empty() {
                            if let Some(app) = &self.app_handle {
                                if !self.suppress_stream {
                                    let _ = app.emit("agent:stream", StreamToken {
                                        token: token_batch,
                                        iteration: 0,
                                    });
                                }
                            }
                        }
                        
                        eprintln!("[LLM] Response received: {} tokens", token_count);
                        Ok((response_text, token_count))
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to read response: {}", e);
                        eprintln!("[LLM] {}", err_msg);
                        if let Some(app) = &self.app_handle {
                            let _ = app.emit("agent:error", &serde_json::json!({
                                "error": err_msg,
                                "phase": "llm_response"
                            }));
                        }
                        Err(err_msg.into())
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to connect to LLM at http://localhost:11434: {}. Is Ollama running?", e);
                eprintln!("[LLM] {}", err_msg);
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:error", &serde_json::json!({
                        "error": err_msg,
                        "phase": "llm_connection"
                    }));
                }
                Err(err_msg.into())
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
        let resolve = |p: &str| -> std::path::PathBuf {
            // Replace LLM placeholder paths like /workspace/ with the real workspace root
            let normalized = if p.starts_with("/workspace/") {
                p.replacen("/workspace/", "", 1)
            } else if p == "/workspace" {
                String::new()
            } else {
                p.to_string()
            };
            let path = std::path::Path::new(&normalized);
            if path.is_absolute() {
                path.to_path_buf()
            } else if normalized.is_empty() {
                std::path::Path::new(ws_root).to_path_buf()
            } else {
                std::path::Path::new(ws_root).join(&normalized)
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
            prompt.push_str(&format!("\n\nCurrent workspace path: {}\nIMPORTANT: Use this EXACT path in all file operations. Do NOT use /workspace/ as a placeholder.", ws));

            // Get file tree with caching (5 minute TTL)
            let file_tree = {
                let cache = self.file_tree_cache.read();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                if let Some((tree, timestamp)) = cache.get(ws) {
                    if now - timestamp < 300 { // 5 minute cache
                        eprintln!("[Cache] File tree hit for {}", ws);
                        tree.clone()
                    } else {
                        drop(cache);
                        let new_tree = crate::commands::vector_search::VectorSearchSystem::build_file_tree(ws, 200);
                        let mut cache_mut = self.file_tree_cache.write();
                        cache_mut.insert(ws.clone(), (new_tree.clone(), now));
                        new_tree
                    }
                } else {
                    drop(cache);
                    let new_tree = crate::commands::vector_search::VectorSearchSystem::build_file_tree(ws, 200);
                    let mut cache_mut = self.file_tree_cache.write();
                    cache_mut.insert(ws.clone(), (new_tree.clone(), now));
                    new_tree
                }
            };
            
            prompt.push_str(&format!("\n\n<workspace_structure>\n{}\n</workspace_structure>", file_tree));

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

    fn build_plan_fast(&self, task: &str) -> Vec<PlannedSubTask> {
        let lower = task.to_lowercase();
        eprintln!("[Planner] Analyzing task: {}", task);
        eprintln!("[Planner] Lowercase: {}", lower);
        
        // Read/search/explain tasks → just executor
        let is_read_only = lower.contains("explain") || lower.contains("what is") || lower.contains("show me") || lower.contains("list");
        eprintln!("[Planner] is_read_only: {}", is_read_only);
        
        // Write/create/fix tasks → researcher + executor + reviewer
        let needs_review = lower.contains("create") || lower.contains("build") || lower.contains("implement") || lower.contains("fix") || lower.contains("refactor");
        eprintln!("[Planner] needs_review: {}", needs_review);

        if is_read_only {
            eprintln!("[Planner] Plan: executor only (read-only task)");
            vec![PlannedSubTask {
                id: "task_1".to_string(),
                agent: "executor".to_string(),
                description: task.to_string(),
            }]
        } else if needs_review {
            eprintln!("[Planner] Plan: researcher → executor → reviewer (complex task)");
            vec![
                PlannedSubTask { id: "task_1".to_string(), agent: "researcher".to_string(), description: format!("Explore workspace structure relevant to: {}", task) },
                PlannedSubTask { id: "task_2".to_string(), agent: "executor".to_string(), description: task.to_string() },
                PlannedSubTask { id: "task_3".to_string(), agent: "reviewer".to_string(), description: "Verify the implementation is correct and complete".to_string() },
            ]
        } else {
            eprintln!("[Planner] Plan: researcher → executor (standard task)");
            vec![
                PlannedSubTask { id: "task_1".to_string(), agent: "researcher".to_string(), description: format!("Gather context for: {}", task) },
                PlannedSubTask { id: "task_2".to_string(), agent: "executor".to_string(), description: task.to_string() },
            ]
        }
    }
}

/// Identifies which tools can be executed in parallel (no shared dependencies)
fn identify_independent_tool_groups(tool_calls: &[ToolCall]) -> Vec<Vec<usize>> {
    if tool_calls.is_empty() {
        return vec![];
    }

    // Tools that read from the same file or write to the same file have dependencies
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut used_indices = std::collections::HashSet::new();

    for (i, tool_i) in tool_calls.iter().enumerate() {
        if used_indices.contains(&i) {
            continue;
        }

        let mut group = vec![i];
        used_indices.insert(i);

        // Find all tools that can run in parallel with tool_i
        for (j, tool_j) in tool_calls.iter().enumerate().skip(i + 1) {
            if used_indices.contains(&j) {
                continue;
            }

            // Check if tools have conflicting file operations
            if !tools_have_conflict(tool_i, tool_j) {
                group.push(j);
                used_indices.insert(j);
            }
        }

        groups.push(group);
    }

    groups
}

/// Checks if two tools have conflicting file operations
fn tools_have_conflict(tool_a: &ToolCall, tool_b: &ToolCall) -> bool {
    // Tools that don't access files can always run in parallel
    let file_tools = ["read_file", "write_file", "edit_file", "delete_file", "list_directory"];
    
    if !file_tools.contains(&tool_a.tool.as_str()) || !file_tools.contains(&tool_b.tool.as_str()) {
        return false;
    }

    // Get file paths from both tools
    let path_a = tool_a.args.get("path").and_then(|p| p.as_str());
    let path_b = tool_b.args.get("path").and_then(|p| p.as_str());

    match (path_a, path_b) {
        (Some(a), Some(b)) => {
            // Same file = conflict
            if a == b {
                return true;
            }
            // One is parent of other = conflict
            if a.starts_with(b) || b.starts_with(a) {
                return true;
            }
            false
        }
        _ => false,
    }
}

fn looks_like_natural_language(response: &str) -> bool {
    let trimmed = response.trim();
    // If it starts with a JSON object brace, it's probably tool calls
    if trimmed.starts_with('{') { return false; }
    // If it contains common natural language patterns, it's prose
    let prose_signals = ["I will", "I'll", "Let me", "First,", "To ", "Step ", "Here ", "Sure", "Okay", "The ", "This "];
    prose_signals.iter().any(|s| trimmed.contains(s))
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
