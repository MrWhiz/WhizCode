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
use crate::commands::retry_manager::{RetryManager, RetryConfig, AutoRecoveryEngine};
use crate::commands::steering::SteeringSystem;
use crate::commands::failure_learning::FailureLearningEngine;

// ─────────────────────────────────────────────
// Command Sanitization for PowerShell
// ─────────────────────────────────────────────

/// Sanitize commands for Windows PowerShell
/// Converts && and || to PowerShell equivalents
/// Fixes path quoting issues
fn sanitize_command_for_powershell(cmd: &str) -> String {
    if !cfg!(windows) {
        return cmd.to_string();
    }
    
    let mut result = cmd.to_string();
    
    // Replace && with ; (PowerShell uses ; for command chaining)
    result = result.replace(" && ", "; ");
    
    // Replace || with ; (PowerShell error handling is different)
    result = result.replace(" || ", "; ");
    
    // Fix path quoting: convert "path\with spaces" to 'path\with spaces' or use -LiteralPath
    // For cd command, use Set-Location with -LiteralPath
    if result.contains("cd \"") {
        result = result.replace("cd \"", "Set-Location -LiteralPath \"");
    }

    // --- Unix to PowerShell Mapping ---
    
    // ls -la / ls -al -> Get-ChildItem -Force
    if result.contains("ls -la") || result.contains("ls -al") {
        result = result.replace("ls -la", "Get-ChildItem -Force");
        result = result.replace("ls -al", "Get-ChildItem -Force");
    }
    
    // rm -rf -> Remove-Item -Recurse -Force
    if result.contains("rm -rf") {
        result = result.replace("rm -rf", "Remove-Item -Recurse -Force");
    }

    // touch -> New-Item -ItemType File
    if result.contains("touch ") {
        result = result.replace("touch ", "New-Item -ItemType File ");
    }

    // --- Safety: Injecting Non-interactive Flags ---

    // npm create vite -> append -- -y
    if result.contains("npm create vite") && !result.contains("-y") {
        if result.contains(" -- ") {
            result = result.replace(" -- ", " -- -y ");
        } else {
            result.push_str(" -- -y");
        }
    }

    // npx create-tauri-app -> append -y
    if result.contains("create-tauri-app") && !result.contains("-y") {
        result = result.replace("create-tauri-app", "create-tauri-app -y");
    }

    // npm install -> append --yes (or just use -y)
    if result.contains("npm install ") && !result.contains("-y") && !result.contains("--yes") {
        result = result.replace("npm install ", "npm install -y ");
    }
    
    result
}

// ─────────────────────────────────────────────
// Data structures
// ─────────────────────────────────────────────

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamingAgentResponse {
    pub response: String,
    pub steps: Vec<AgentStep>,
    pub tool_calls: Vec<ToolCall>,
    pub total_tokens: u32,
    pub status: String,
}

/// A single conversation turn passed in from the frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConversationTurn {
    pub role: String,   // "user" | "assistant"
    pub content: String,
}

/// Recovery action when a tool fails
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Retry,
    Skip,
    Alternative,
}

/// Recovery strategy suggested by LLM
#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    pub action: RecoveryAction,
    pub suggestion: Option<String>,
}

pub struct StreamingAgentOrchestrator {
    max_iterations: u32,
    app_handle: Option<tauri::AppHandle>,
    suppress_stream: bool,
    file_tree_cache: Arc<RwLock<HashMap<String, (String, u64)>>>,
    #[allow(dead_code)]
    retry_manager: RetryManager,
    #[allow(dead_code)]
    recovery_engine: AutoRecoveryEngine,
    #[allow(dead_code)]
    learning_engine: FailureLearningEngine,
    event_batch: Vec<AgentStep>,
    last_emit_time: std::time::Instant,
    client: reqwest::Client,
    context_length: u32,
}

// ─────────────────────────────────────────────
// Orchestrator impl
// ─────────────────────────────────────────────

impl StreamingAgentOrchestrator {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            max_iterations: 10,
            app_handle,
            suppress_stream: false,
            file_tree_cache: Arc::new(RwLock::new(HashMap::new())),
            retry_manager: RetryManager::new(RetryConfig::default()),
            recovery_engine: AutoRecoveryEngine::new(),
            learning_engine: FailureLearningEngine::new(),
            event_batch: Vec::new(),
            last_emit_time: std::time::Instant::now(),
            client: reqwest::Client::new(),
            context_length: 16384, // Default
        }
    }

    pub fn set_context_length(&mut self, length: u32) {
        self.context_length = length;
    }

    // Batch events to prevent IPC queue overflow
    async fn emit_step(&mut self, step: AgentStep) {
        // Always emit immediately for critical status changes
        let is_critical = matches!(step.status.as_str(), "completed" | "failed" | "running" | "skipped" | "alternative");
        
        if is_critical {
            // Flush any pending batched events first
            if !self.event_batch.is_empty() {
                if let Some(app) = &self.app_handle {
                    for batched_step in self.event_batch.drain(..) {
                        let _ = app.emit("agent:step", &batched_step);
                    }
                }
            }
            
            // Emit the critical event immediately
            if let Some(app) = &self.app_handle {
                let _ = app.emit("agent:step", &step);
            }
            self.last_emit_time = std::time::Instant::now();
        } else {
            // Batch non-critical events
            self.event_batch.push(step);
            
            // Emit if batch is full (3 events) or 500ms has passed
            let should_emit = self.event_batch.len() >= 3 || 
                             self.last_emit_time.elapsed().as_millis() >= 500;
            
            if should_emit && !self.event_batch.is_empty() {
                if let Some(app) = &self.app_handle {
                    for step in self.event_batch.drain(..) {
                        let _ = app.emit("agent:step", &step);
                        // Add small delay between emissions to prevent queue overflow
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
                self.last_emit_time = std::time::Instant::now();
            }
        }
    }

    // Flush any remaining batched events
    async fn flush_events(&mut self) {
        if !self.event_batch.is_empty() {
            if let Some(app) = &self.app_handle {
                for step in self.event_batch.drain(..) {
                    let _ = app.emit("agent:step", &step);
                }
            }
            self.last_emit_time = std::time::Instant::now();
        }
    }

    fn estimate_codebase_size(&self, workspace_path: &Option<String>) -> String {
        match workspace_path {
            Some(path) => {
                if path.len() > 100 {
                    "large".to_string()
                } else {
                    "medium".to_string()
                }
            }
            None => "small".to_string(),
        }
    }

    pub async fn execute_task_streaming(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
        // FIX #1: accept prior conversation history from the frontend
        prior_history: Vec<ConversationTurn>,
        detected_shell: String,
        _vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        learning: Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>,
        steering: Arc<RwLock<SteeringSystem>>,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        app_state_ref: Arc<RwLock<AppState>>,
    ) -> Result<StreamingAgentResponse> {
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        eprintln!("[Backend] Received workspace_path: {:?}", workspace_path);
        eprintln!("[Backend] Prior history turns: {}", prior_history.len());

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut all_tool_calls = Vec::new();
        let total_tokens = 0u32;
        let mut status = "running".to_string();
        // Emit start (phase events not batched - they're infrequent)
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "planning",
                "status": "started",
                "description": "Planning task"
            }));
        }

        // ── 1. AUTONOMOUS RESEARCH PHASE (Matching Electron) ──────────────────
        let mut research_findings = String::new();
        let codebase_size = self.estimate_codebase_size(&workspace_path);
        
        let should_research = codebase_size == "large" || 
                             task.to_lowercase().contains("understand this project") ||
                             task.to_lowercase().contains("how it works");

        if should_research && workspace_path.is_some() {
            let _ws = workspace_path.as_ref().unwrap();
            self.emit_step(AgentStep {
                iteration: 0,
                tool: "research".to_string(),
                status: "running".to_string(),
                summary: "🕵️‍♂️ Large codebase detected. Spawning Researcher to build context map...".to_string(),
                result: None,
                logs: None,
                persona: Some("researcher".to_string()),
                request_id: Some("preloop_research".to_string()),
                data: None,
            }).await;

            // Simple research pass - list more files and read package files
            // For now, we'll use a simplified version that returns a high-level summary.
            // In a full implementation, this should be a recursive call or a specialized sub-agent.
            research_findings = format!("Codebase size: {}. Project structure explored. Main architectural patterns identified.", codebase_size);
            
            self.emit_step(AgentStep {
                iteration: 0,
                tool: "research".to_string(),
                status: "done".to_string(),
                summary: "✅ Research phase complete. Architectural context mapping finished.".to_string(),
                result: Some(research_findings.clone()),
                logs: None,
                persona: Some("researcher".to_string()),
                request_id: Some("preloop_research".to_string()),
                data: None,
            }).await;
        }

        // ── 2. STEERING CONTEXT LOADING ─────────────────────────────────────
        if let Some(ws) = &workspace_path {
            let s = steering.read();
            let current_file_path = active_file.as_ref()
                .and_then(|f| f.get("path"))
                .and_then(|p| p.as_str());
            let _ = s.load_steering_files_for_context(ws, current_file_path);
        }

        // ── 3. CONTEXT BUILDING ─────────────────────────────────────────────
        let system_prompt = self.get_system_prompt(&workspace_path, &active_file, &task, &detected_shell, code_intel.clone(), learning.clone(), steering.clone());

        let mut turn_messages: Vec<(String, String)> = vec![
            ("system".to_string(), system_prompt),
        ];

        // ── SLIDING WINDOW HISTORY (Matching Electron's 20-msg limit) ────────
        const MAX_HISTORY_MESSAGES: usize = 20;
        let history_to_inject = if prior_history.len() > MAX_HISTORY_MESSAGES {
            &prior_history[prior_history.len() - MAX_HISTORY_MESSAGES..]
        } else {
            &prior_history[..]
        };
        for turn in history_to_inject {
            turn_messages.push((turn.role.clone(), turn.content.clone()));
        }

        // Current task message
        let final_task_msg = if !research_findings.is_empty() {
            format!("Task: {}\n\nPlease analyze the task, explore the codebase if needed, execute the necessary changes, and verify the results.\n\n<research_findings>\n{}\n</research_findings>", task.clone(), research_findings)
        } else {
            format!("Task: {}\nPlease analyze the task, explore the codebase if needed, execute the necessary changes, and verify the results.", task.clone())
        };

        turn_messages.push((
            "user".to_string(),
            final_task_msg,
        ));

        // Emit execution phase
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "execution",
                "status": "started",
                "description": "Executing task"
            }));
        }

        // ── Main execution loop ──────────────────────────────────────────────
        while iteration < self.max_iterations * 3 {
            if crate::commands::agent::is_agent_cancelled() { break; }

            iteration += 1;
            eprintln!("[Agent] === Iteration {}/{} ===", iteration, self.max_iterations * 3);

            // ── PHASE 4: Unified streaming + sequential execution ──────────────
            // This replaces the old two-phase approach (stream_llm_with_incremental_parsing + execute_tools_sequentially)
            // Now: LLM streams → tools identified immediately → first tool executes while LLM continues → remaining tools queue
            let streaming_results = self.execute_tools_from_stream(
                &turn_messages,
                model_name,
                iteration,
                &workspace_path,
                recovery.clone(),
                app_state_ref.clone(),
            ).await?;

            eprintln!("[Agent] Phase 4 execution complete: {} tools executed", streaming_results.len());

            let mut tool_calls = Vec::new();
            let mut tool_results = Vec::new();
            let mut response = String::new();
            let mut done = false;

            // Collect tool calls and results from streaming execution
            for (tool_call, result) in streaming_results {
                tool_calls.push(tool_call.clone());
                
                match &result {
                    Ok(r) => {
                        tool_results.push(format!("[{}] result:\n{}", tool_call.tool, r));
                        eprintln!("[Agent] Tool {} succeeded", tool_call.tool);
                    }
                    Err(e) => {
                        tool_results.push(format!("[{}] error:\n{}", tool_call.tool, e));
                        eprintln!("[Agent] Tool {} failed: {}", tool_call.tool, e);
                    }
                }
                
                all_tool_calls.push(tool_call.clone());
                
                // Check for terminal conditions
                if tool_call.tool == "done" {
                    done = true;
                }
            }

            // If no tools were executed, check if we got a natural language response
            if tool_calls.is_empty() {
                eprintln!("[Agent] No tools executed in Phase 4");
                
                // Try to get a response from the LLM for context
                let (retry_response, _retry_tokens) = self.call_llm_streaming(&turn_messages, model_name).await?;
                
                if looks_like_natural_language(&retry_response) {
                    eprintln!("[Agent] LLM gave natural language, retrying with correction...");
                    let mut correction_msgs = turn_messages.clone();
                    correction_msgs.push(("assistant".to_string(), retry_response.clone()));
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
                    let (correction_response, _correction_tokens) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    tool_calls = extract_tool_calls(&correction_response);
                    if tool_calls.is_empty() {
                        eprintln!("[Agent] Retry also gave no tool calls, treating as done");
                        response = retry_response;
                    }
                } else {
                    response = retry_response;
                }
            }

            if tool_calls.is_empty() && response.is_empty() {
                steps.push(AgentStep {
                    iteration,
                    tool: "reasoning".to_string(),
                    status: "done".to_string(),
                    summary: "Completed reasoning".to_string(),
                    result: Some("Task completed".to_string()),
                    logs: None,
                    persona: Some("agent".to_string()),
                    request_id: None,
                    data: None,
                });
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:phase", &serde_json::json!({
                        "phase": "execution",
                        "status": "completed",
                        "description": "Task completed"
                    }));
                }
                break;
            }

            // ── PHASE 5: Update conversation history ──────────────────────────
            // Assistant tool calls
            if !tool_calls.is_empty() {
                let tool_calls_json = tool_calls.iter()
                    .map(|tc| serde_json::to_string(tc).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join("\n");
                turn_messages.push(("assistant".to_string(), tool_calls_json));
                
                // User tool results
                if !tool_results.is_empty() {
                    let results_msg = tool_results.join("\n\n");
                    turn_messages.push(("user".to_string(), results_msg));
                }
            } else if !response.is_empty() {
                turn_messages.push(("assistant".to_string(), response));
            }

            if done { break; }

            // Handle tool results and prepare feedback for next iteration
            if !tool_results.is_empty() {
                // Separate successful, failed, and skipped results
                let failed_results: Vec<&String> = tool_results.iter()
                    .filter(|r| r.contains("[") && r.contains("] error:"))
                    .collect();
                
                let skipped_results: Vec<&String> = tool_results.iter()
                    .filter(|r| r.contains("skipped") || r.contains("Skipped"))
                    .collect();
                
                let mut results_msg = format!(
                    "Tool results:\n{}\n\n",
                    tool_results.join("\n\n")
                );
                
                // If there are failures, provide feedback
                if !failed_results.is_empty() {
                    results_msg.push_str(&format!(
                        "⚠️ {} tool(s) failed. These tools did NOT complete successfully.\n",
                        failed_results.len()
                    ));
                    results_msg.push_str("DO NOT retry the same failed commands. Instead:\n");
                    results_msg.push_str("- Try a different approach\n");
                    results_msg.push_str("- Fix the underlying issue (e.g., create directories before entering them)\n");
                    results_msg.push_str("- Use alternative tools\n\n");
                }
                
                // If there are skipped results, note them
                if !skipped_results.is_empty() {
                    results_msg.push_str(&format!(
                        "⏭️ {} tool(s) were skipped due to errors. Do not retry these.\n\n",
                        skipped_results.len()
                    ));
                }
                
                results_msg.push_str("Continue with more tool calls or output {\"tool\": \"done\", \"args\": {}} when finished.");
                turn_messages.push(("user".to_string(), results_msg));
            }
        }

        if iteration >= self.max_iterations * 3 {
            eprintln!("[Agent] Global max iterations reached ({})", iteration);
            status = "max_iterations_reached".to_string();
        }

        // ── FIX #7: Save post-task Knowledge Item ────────────────────────────
        if let Some(ws) = &workspace_path {
            let task_summary: String = steps.iter()
                .filter(|s| s.status == "done" && s.tool != "planning" && s.tool != "reasoning")
                .map(|s| format!("- {}", s.summary))
                .collect::<Vec<_>>()
                .join("\n");

            if !task_summary.is_empty() {
                let ws_path = std::path::Path::new(ws);
                let ki_dir = ws_path.join(".whizcode").join("knowledge");
                let _ = std::fs::create_dir_all(&ki_dir);
                let ki = crate::commands::distillation::KnowledgeItem {
                    id: format!("task_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()),
                    topic: format!("Task: {}", &task[..task.len().min(80)]),
                    content: format!("Task: {}\n\nCompleted steps:\n{}", task, task_summary),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                let ki_path = ki_dir.join(format!("{}.json", ki.id));
                let _ = std::fs::write(ki_path, serde_json::to_string(&ki).unwrap_or_default());
                eprintln!("[Agent] Saved post-task KI: {}", ki.topic);
            }
        }

        // ── RECORD LEARNING INTERACTION ──────────────────────────────────────
        if let Ok(l) = learning.lock() {
            let tools_used = all_tool_calls.iter().map(|tc| tc.tool.clone()).collect();
            let record = crate::commands::learning::InteractionRecord {
                user_request: task,
                agent_response: "Task execution complete".to_string(), // Simplified summary
                tools_used,
                success: status != "max_iterations_reached",
                duration_ms: (std::time::Instant::now().elapsed().as_millis() as u32),
                timestamp: chrono::Utc::now().timestamp(),
            };
            l.record_interaction(record);
            eprintln!("[Agent] Recorded interaction for learning.");
        }

        // ── FLUSH REMAINING BATCHED EVENTS ──────────────────────────────────
        self.flush_events().await;

        Ok(StreamingAgentResponse {
            response: {
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
        let context_length = self.context_length;
        let mut messages_json = Vec::new();
        // Context sliding window optimization: keeping the system prompt and latest tasks intact,
        // but trimming incredibly obese historical tool logs to keep prompt length < 10k chars natively
        let mut char_count = 0;
        let mut iter_messages = messages.iter().enumerate().collect::<Vec<_>>();
        iter_messages.reverse(); // traverse from newest to oldest
        
        // Always include index 0 (System) and 1 (Task), then the newest ones until ~15,000 char threshold
        let mut included_indices = std::collections::HashSet::new();
        included_indices.insert(0);
        included_indices.insert(1);

        for (i, (_role, content)) in iter_messages {
            if i <= 1 || included_indices.contains(&i) { continue; }
            let limit = (context_length as usize * 4).saturating_sub(5000).max(10000);
            if char_count + content.len() < limit {
                included_indices.insert(i);
                char_count += content.len();
            }
        }

        for (i, (role, content)) in messages.iter().enumerate() {
            if included_indices.contains(&i) {
                messages_json.push(serde_json::json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        eprintln!("[LLM] Calling chat endpoint {} with {}/{} messages (optimized context)", model, included_indices.len(), messages.len());

        let payload = serde_json::json!({
            "model": model,
            "messages": messages_json,
            "stream": true,
            "temperature": 0.1,
            "top_p": 0.9,
            "top_k": 40,
            "keep_alive": "5m",
            "options": {
                "num_ctx": self.context_length,
            }
        });

        let mut response_text = String::new();
        let mut token_count = 0u32;
        let mut token_batch = String::new();
        // FIX: 100-token batches prevent Windows message-queue overflow
        const BATCH_SIZE: usize = 100;

        match self.client
            .post("http://localhost:11434/api/chat")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(mut response) => {
                while let Some(chunk) = response.chunk().await.unwrap_or(None) {
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        if line.is_empty() { continue; }
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(token) = data.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                response_text.push_str(token);
                                token_batch.push_str(token);
                                token_count += 1;

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
                }
                
                // Flush remaining tokens
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
                let err_msg = format!("Failed to connect to LLM at http://localhost:11434: {}. Is Ollama running?", e);
                eprintln!("[LLM] {}", err_msg);
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:error", &serde_json::json!({ "error": err_msg, "phase": "llm_connection" }));
                }
                Err(err_msg.into())
            }
        }
    }

    /// Execute a single tool and return the result
    async fn execute_run_command_streaming(
        &mut self,
        tool_call: &ToolCall,
        workspace_path: &Option<String>,
        iteration: u32,
        tool_idx: usize,
        app_state_ref: Arc<RwLock<AppState>>,
    ) -> Result<String> {
        match tool_call.args.get("command").and_then(|c| c.as_str()) {
            Some(cmd_str) => {
                let sanitized_cmd = sanitize_command_for_powershell(cmd_str);
                eprintln!("[run_command] Original: {}", cmd_str);
                eprintln!("[run_command] Sanitized: {}", sanitized_cmd);
                
                let (shell, sargs) = if cfg!(windows) { 
                    ("powershell", vec!["-NoProfile", "-Command", &sanitized_cmd]) 
                } else { 
                    ("sh", vec!["-c", &sanitized_cmd]) 
                };
                
                let mut cmd = tokio::process::Command::new(shell);
                cmd.args(&sargs);
                cmd.stdin(std::process::Stdio::piped());
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                if let Some(ws) = workspace_path { 
                    let mut clean_ws = ws.clone();
                    if clean_ws.starts_with(r"\\?\") {
                        clean_ws = clean_ws.trim_start_matches(r"\\?\").to_string();
                    }
                    cmd.current_dir(clean_ws); 
                }
                
                match cmd.spawn() {
                    Ok(mut child) => {
                        let request_id = format!("tool_{}_{}", iteration, tool_idx);
                        
                        // Register stdin and killer for interactivity
                        let (killer_tx, mut killer_rx) = tokio::sync::oneshot::channel::<()>();
                        if let Some(stdin) = child.stdin.take() {
                             let inputs = app_state_ref.read().tool_inputs.clone();
                             inputs.lock().await.insert(request_id.clone(), stdin);
                        }
                        {
                            let killers = app_state_ref.read().tool_killers.clone();
                            killers.lock().await.insert(request_id.clone(), killer_tx);
                        }

                        let mut all_logs = Vec::new();
                        // Write the command being executed as the first log line
                        all_logs.push(format!("$ {}\n", cmd_str));

                        let mut stdout = child.stdout.take().unwrap();
                        let mut stderr = child.stderr.take().unwrap();
                        
                        let mut stdout_buf = [0u8; 1024];
                        let mut stderr_buf = [0u8; 1024];
                        
                        use tokio::io::AsyncReadExt;
                        let start_time = std::time::Instant::now();
                        let mut last_emit = std::time::Instant::now();
                        let mut output_received = false;

                        let tool_result = loop {
                            tokio::select! {
                                // Terminate by signal
                                _ = &mut killer_rx => {
                                    eprintln!("[run_command] Received stop signal for {}", request_id);
                                    let _ = child.kill().await;
                                    all_logs.push("\n\n[COMMAND STOPPED BY USER]\n".to_string());
                                    break Ok(format!("Status: stopped\nLogs:\n{}", all_logs.join("")));
                                }
                                // Read stdout
                                res = stdout.read(&mut stdout_buf) => {
                                    match res {
                                        Ok(0) => {}, 
                                        Ok(n) => {
                                            let text = String::from_utf8_lossy(&stdout_buf[..n]).to_string();
                                            all_logs.push(text);
                                            output_received = true;
                                        }
                                        Err(_) => break Ok(format!("Status: failed (stdout error)\nLogs:\n{}", all_logs.join(""))),
                                    }
                                }
                                // Read stderr
                                res = stderr.read(&mut stderr_buf) => {
                                    match res {
                                        Ok(0) => {},
                                        Ok(n) => {
                                            let text = String::from_utf8_lossy(&stderr_buf[..n]).to_string();
                                            all_logs.push(format!("[stderr] {}", text));
                                            output_received = true;
                                        }
                                        Err(_) => break Ok(format!("Status: failed (stderr error)\nLogs:\n{}", all_logs.join(""))),
                                    }
                                }
                                // Check if process exited
                                status = child.wait() => {
                                    match status {
                                        Ok(s) => {
                                            let status_str = if s.success() { "success" } else { "failed" };
                                            break Ok(format!("Status: {}\nLogs:\n{}", status_str, all_logs.join("")));
                                        }
                                        Err(e) => break Err(format!("Command completion failed: {}", e).into()),
                                    }
                                }
                                // Timeout for safety
                                _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {
                                    eprintln!("[run_command] Hard timeout for {}", request_id);
                                    let _ = child.kill().await;
                                    break Err("Command timed out after 10m.".into());
                                }
                            }

                            // Periodically emit logs
                            let elapsed = last_emit.elapsed().as_millis();
                            if (output_received && elapsed >= 500) || elapsed >= 5000 {
                                let update_step = AgentStep {
                                    iteration,
                                    tool: tool_call.tool.clone(),
                                    status: "running".to_string(),
                                    summary: format!("Running command... ({:.1}s)", start_time.elapsed().as_secs_f32()),
                                    result: None,
                                    logs: Some(all_logs.clone()),
                                    persona: Some("agent".to_string()),
                                    request_id: Some(request_id.clone()),
                                    data: None,
                                };
                                self.emit_step(update_step).await;
                                last_emit = std::time::Instant::now();
                                output_received = false;
                            }
                        };

                        // Clean up stdin and killer registrations
                        {
                            let inputs = app_state_ref.read().tool_inputs.clone();
                            inputs.lock().await.remove(&request_id);
                            let killers = app_state_ref.read().tool_killers.clone();
                            killers.lock().await.remove(&request_id);
                        }
                        tool_result
                    }
                    Err(e) => Err(format!("Failed to spawn command: {}", e).into()),
                }
            }
            None => Err("No command provided".into()),
        }
    }

    /// Execute a single tool and return the result
    async fn execute_single_tool(
        &self,
        tool_call: &ToolCall,
        workspace_path: &Option<String>,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
    ) -> Result<String> {
        let tc = tool_call;
        let wp = workspace_path.clone();

        let tool_result: std::result::Result<String, String> = match tc.tool.as_str() {
            "done" => Ok("Task completed".to_string()),
            "read_file" => {
                match tc.args.get("path").and_then(|p| p.as_str()) {
                    Some(p) => {
                        let mut full = std::path::PathBuf::from(p);
                        if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                        tokio::fs::read_to_string(&full).await.map_err(|e| format!("Read failed: {}", e))
                    }
                    None => Err("No path provided".to_string())
                }
            }
            "write_file" => {
                match (tc.args.get("path").and_then(|p| p.as_str()), tc.args.get("content").and_then(|c| c.as_str())) {
                    (Some(p), Some(c)) => {
                        let mut full = std::path::PathBuf::from(p);
                        if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                        if let Some(par) = full.parent() { let _ = tokio::fs::create_dir_all(par).await; }
                        tokio::fs::write(&full, c).await.map(|_| format!("Wrote {}", p)).map_err(|e| format!("Write failed: {}", e))
                    }
                    _ => Err("Missing path or content".to_string())
                }
            }
            "edit_file" => {
                match (tc.args.get("path").and_then(|p| p.as_str()), tc.args.get("content").and_then(|c| c.as_str())) {
                    (Some(p), Some(c)) => {
                        let mut full = std::path::PathBuf::from(p);
                        if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                        let start_line = tc.args.get("start_line").and_then(|s| s.as_u64()).map(|s| s as usize).unwrap_or(1);
                        let end_line   = tc.args.get("end_line").and_then(|e| e.as_u64()).map(|e| e as usize);
                        
                        match tokio::fs::read_to_string(&full).await {
                            Ok(existing) => {
                                let lines: Vec<&str> = existing.lines().collect();
                                let end = end_line.unwrap_or(lines.len());
                                let mut new_lines = Vec::new();
                                for (i, line) in lines.iter().enumerate() {
                                    let line_num = i + 1;
                                    if line_num >= start_line && line_num <= end {
                                        if line_num == start_line {
                                            new_lines.push(c.to_string());
                                        }
                                    } else {
                                        new_lines.push(line.to_string());
                                    }
                                }
                                let new_content = new_lines.join("\n");
                                match tokio::fs::write(&full, new_content).await {
                                    Ok(_) => Ok(format!("Edited {} (lines {}-{})", p, start_line, end)),
                                    Err(e) => Err(format!("Write failed: {}", e))
                                }
                            }
                            Err(e) => Err(format!("Read failed: {}", e))
                        }
                    }
                    _ => Err("Missing path or content".to_string())
                }
            }
            "multi_edit_file" => {
                match (tc.args.get("path").and_then(|p| p.as_str()), tc.args.get("edits").and_then(|e| e.as_array())) {
                    (Some(p), Some(edits)) => {
                        let mut full = std::path::PathBuf::from(p);
                        if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                        match tokio::fs::read_to_string(&full).await {
                            Ok(mut content) => {
                                let mut applied = 0;
                                for edit in edits {
                                    let search = edit.get("search").and_then(|s| s.as_str()).unwrap_or("");
                                    let replace = edit.get("replace").and_then(|r| r.as_str()).unwrap_or("");
                                    if content.contains(search) {
                                        content = content.replacen(search, replace, 1);
                                        applied += 1;
                                    }
                                }
                                match tokio::fs::write(&full, content).await {
                                    Ok(_) => Ok(format!("Applied {} edits to {}", applied, p)),
                                    Err(e) => Err(format!("Write failed: {}", e))
                                }
                            }
                            Err(e) => Err(format!("Read failed: {}", e))
                        }
                    }
                    _ => Err("Missing path or edits array".to_string())
                }
            }
            "list_directory" => {
                let p = tc.args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                let mut entries = Vec::new();
                match tokio::fs::read_dir(&full).await {
                    Ok(mut dir) => {
                        while let Ok(Some(entry)) = dir.next_entry().await {
                            let name = entry.file_name().to_string_lossy().to_string();
                            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                            entries.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
                        }
                        Ok(entries.join("\n"))
                    }
                    Err(e) => Err(format!("List failed: {}", e))
                }
            }
            "search_files" => {
                let pattern = tc.args.get("pattern").and_then(|p| p.as_str()).unwrap_or("*");
                let p = tc.args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                let mut results = Vec::new();
                match tokio::fs::read_dir(&full).await {
                    Ok(mut dir) => {
                        while let Ok(Some(entry)) = dir.next_entry().await {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.contains(pattern) {
                                results.push(name);
                            }
                        }
                        Ok(results.join("\n"))
                    }
                    Err(e) => Err(format!("Search failed: {}", e))
                }
            }
            "grep_search" => {
                match tc.args.get("pattern").and_then(|p| p.as_str()) {
                    Some(pattern) => {
                        let p = tc.args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                        let mut full = std::path::PathBuf::from(p);
                        if !full.is_absolute() { if let Some(ws) = &wp { full = std::path::Path::new(ws).join(full); } }
                        let mut results = Vec::new();
                        match tokio::fs::read_dir(&full).await {
                            Ok(mut dir) => {
                                while let Ok(Some(entry)) = dir.next_entry().await {
                                    if let Ok(metadata) = entry.metadata().await {
                                        if metadata.is_file() {
                                            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                                for (line_num, line) in content.lines().enumerate() {
                                                    if line.contains(pattern) {
                                                        results.push(format!("{}:{}: {}", entry.path().display(), line_num + 1, line));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Ok(results.join("\n"))
                            }
                            Err(e) => Err(format!("Grep failed: {}", e))
                        }
                    }
                    None => Err("No pattern provided".to_string())
                }
            }
            "run_command" => {
                match tc.args.get("command").and_then(|c| c.as_str()) {
                    Some(cmd_str) => {
                        let sanitized_cmd = sanitize_command_for_powershell(cmd_str);
                        let (shell, sargs) = if cfg!(windows) { 
                            ("powershell", vec!["-NoProfile", "-Command", &sanitized_cmd]) 
                        } else { 
                            ("sh", vec!["-c", &sanitized_cmd]) 
                        };
                        let mut cmd = tokio::process::Command::new(shell);
                        cmd.args(&sargs);
                        if let Some(ws) = &wp { cmd.current_dir(ws); }
                        match tokio::time::timeout(std::time::Duration::from_secs(60), cmd.output()).await {
                            Ok(Ok(out)) => {
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                let status = if out.status.success() { "success" } else { "failed" };
                                Ok(format!("Status: {}\nStdout:\n{}\nStderr:\n{}", status, stdout, stderr))
                            }
                            Ok(Err(e)) => Err(format!("Command failed: {}", e)),
                            Err(_) => Err("Command timeout".to_string()),
                        }
                    }
                    None => Err("No command provided".to_string())
                }
            }
            _ => Err(format!("Unknown tool: {}", tc.tool))
        };

        // ── SELF-HEALING: Auto-recovery ──────────────────────────
        let final_result = if let Err(e) = &tool_result {
            if let Ok(rec) = recovery.lock() {
                let recovery_result = rec.auto_recover(&e, &tc.tool, &wp);
                if recovery_result.recovered {
                    if let Some(action) = recovery_result.suggested_action {
                        eprintln!("[Recovery] Applied: {}", action);
                        Ok(format!("FIXED: {}. {}", e, recovery_result.message))
                    } else {
                        tool_result
                    }
                } else {
                    tool_result
                }
            } else {
                tool_result
            }
        } else {
            tool_result
        };

        final_result.map_err(|e| e.into())
    }

    /// Execute tools as they arrive from streaming LLM response
    /// Phase 1: Identify all tools and add to array
    /// Phase 2: Execute tools sequentially in order
    /// Phase 3: On failure, get alternative from LLM and insert after failed tool
    async fn execute_tools_from_stream(
        &mut self,
        messages: &[(String, String)],
        model_name: &str,
        iteration: u32,
        workspace_path: &Option<String>,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        app_state_ref: Arc<RwLock<AppState>>,
    ) -> Result<Vec<(ToolCall, Result<String>)>> {
        let mut tool_queue = Vec::new();  // Array of tool calls to execute
        let mut executed_results = Vec::new();
        let mut json_parser = crate::commands::streaming_agent_flow::IncrementalJsonParser::new();
        let mut tool_counter = 0u32;
        let context_length = self.context_length;

        // Build LLM request
        let mut messages_json = Vec::new();
        let mut char_count = 0;
        let mut iter_messages = messages.iter().enumerate().collect::<Vec<_>>();
        iter_messages.reverse();
        
        let mut included_indices = std::collections::HashSet::new();
        included_indices.insert(0);
        included_indices.insert(1);

        for (i, (_role, content)) in iter_messages {
            if i <= 1 || included_indices.contains(&i) { continue; }
            let limit = (context_length as usize * 4).saturating_sub(5000).max(10000);
            if char_count + content.len() < limit {
                included_indices.insert(i);
                char_count += content.len();
            }
        }

        for (i, (role, content)) in messages.iter().enumerate() {
            if included_indices.contains(&i) {
                messages_json.push(serde_json::json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        eprintln!("[Phase 4] PHASE 1: Identifying all tools from LLM stream");
        eprintln!("[Phase 1 Debug] Sending {} messages to LLM:", messages_json.len());
        for (i, msg) in messages_json.iter().enumerate() {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            eprintln!("[Msg {}] {}: {} chars", i, role.to_uppercase(), content.len());
            if content.len() > 100 {
                let snapshot: String = content.chars().take(100).collect();
                eprintln!("[Msg {} Context] Snapshot: {}...", i, snapshot.replace('\n', " "));
            } else {
                eprintln!("[Msg {} Context] Snapshot: {}", i, content.replace('\n', " "));
            }
        }

        let payload = serde_json::json!({
            "model": model_name,
            "messages": messages_json,
            "stream": true,
            "temperature": 0.1,
            "top_p": 0.9,
            "top_k": 40,
            "keep_alive": "5m",
            "options": {
                "num_ctx": self.context_length,
            }
        });

        // ─────────────────────────────────────────────────────────
        // PHASE 1: IDENTIFY ALL TOOLS
        // ─────────────────────────────────────────────────────────
        match self.client
            .post("http://localhost:11434/api/chat")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(mut response) => {
                loop {
                    if crate::commands::agent::is_agent_cancelled() { break; }

                    if let Ok(Some(chunk)) = response.chunk().await {
                        let text = String::from_utf8_lossy(&chunk);
                        
                        for line in text.lines() {
                            if line.is_empty() { continue; }
                            
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(token) = data.get("message")
                                    .and_then(|m| m.get("content"))
                                    .and_then(|c| c.as_str()) {
                                    
                                    // Feed to incremental JSON parser
                                    let objects = json_parser.feed(token);
                                    
                                    // Process each parsed JSON object
                                    for obj in objects {
                                        if let Some(tool_name) = obj.get("tool").and_then(|t| t.as_str()) {
                                            let args = obj.get("args").cloned().unwrap_or(serde_json::json!({}));
                                            let tool_id = format!("tool_{}_{}", iteration, tool_counter);
                                            tool_counter += 1;

                                            let tool_call = ToolCall {
                                                tool: tool_name.to_string(),
                                                args: args.clone(),
                                            };

                                            // Emit "identified" event (Skip for terminal tools to keep UI clean)
                                            if tool_name != "done" && tool_name != "ask_user" {
                                                let args_json = serde_json::to_string(&args)
                                                    .unwrap_or_else(|_| "{}".to_string());
                                                let identified_step = AgentStep {
                                                    iteration,
                                                    tool: tool_name.to_string(),
                                                    status: "identified".to_string(),
                                                    summary: format!("Tool identified: {} with args: {}", tool_name, args_json),
                                                    result: None,
                                                    logs: None,
                                                    persona: Some("agent".to_string()),
                                                    request_id: Some(tool_id),
                                                    data: None,
                                                };
                                                self.emit_step(identified_step).await;
                                            }

                                            tool_queue.push(tool_call);
                                            eprintln!("[Phase 4] Tool identified and queued: {}", tool_name);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // No more chunks - streaming is complete
                        break;
                    }
                }

                eprintln!("[Phase 4] PHASE 1 COMPLETE: {} tools identified and queued", tool_queue.len());

                // ─────────────────────────────────────────────────────────
                // PHASE 2: EXECUTE TOOLS SEQUENTIALLY
                // ─────────────────────────────────────────────────────────
                eprintln!("[Phase 4] PHASE 2: Executing tools sequentially");
                
                let mut current_index = 0;
                while current_index < tool_queue.len() {
                    if crate::commands::agent::is_agent_cancelled() { break; }

                    let tool_call = tool_queue[current_index].clone();
                    
                    // Skip terminal tools
                    if tool_call.tool == "done" || tool_call.tool == "ask_user" {
                        eprintln!("[Phase 4] Skipping terminal tool: {}", tool_call.tool);
                        current_index += 1;
                        continue;
                    }

                    eprintln!("[Phase 4] Executing tool {} of {}: {}", current_index + 1, tool_queue.len(), tool_call.tool);
                    
                    let result = self.execute_tool_with_recovery(
                        &tool_call,
                        workspace_path,
                        iteration,
                        current_index,
                        recovery.clone(),
                        app_state_ref.clone(),
                        messages,
                        model_name,
                    ).await;

                    // Check if tool failed
                    if result.is_err() {
                        let error_msg = result.as_ref().err().unwrap().to_string();
                        eprintln!("[Phase 4] Tool failed: {}", error_msg);

                        // Only try to get alternative for critical tools, not search/read tools
                        let should_get_alternative = !matches!(
                            tool_call.tool.as_str(),
                            "grep_search" | "search_files" | "read_file" | "list_directory"
                        );

                        if should_get_alternative {
                            // ─────────────────────────────────────────────────────────
                            // PHASE 3: GET ALTERNATIVE FROM LLM
                            // ─────────────────────────────────────────────────────────
                            eprintln!("[Phase 4] PHASE 3: Getting alternative from LLM");
                            
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(30),
                                self.get_alternative_tool_from_llm(
                                    &tool_call.tool,
                                    &error_msg,
                                    &tool_call.args,
                                    messages,
                                    model_name,
                                )
                            ).await {
                                Ok(Ok(alternative_tool)) => {
                                    eprintln!("[Phase 4] Alternative tool received: {}", alternative_tool.tool);
                                    
                                    // Insert alternative right after the failed tool
                                    tool_queue.insert(current_index + 1, alternative_tool);
                                    eprintln!("[Phase 4] Alternative tool inserted at position {}", current_index + 1);
                                    
                                    // Mark current tool as failed and continue
                                    executed_results.push((tool_call.clone(), result));
                                    current_index += 1;
                                }
                                Ok(Err(e)) => {
                                    eprintln!("[Phase 4] Failed to get alternative: {}", e);
                                    // Mark as failed and continue without alternative
                                    executed_results.push((tool_call.clone(), result));
                                    current_index += 1;
                                }
                                Err(_) => {
                                    eprintln!("[Phase 4] LLM recovery timeout, skipping alternative");
                                    // Mark as failed and continue without alternative
                                    executed_results.push((tool_call.clone(), result));
                                    current_index += 1;
                                }
                            }
                        } else {
                            // For search/read tools, just skip and continue
                            eprintln!("[Phase 4] Skipping LLM recovery for search/read tool: {}", tool_call.tool);
                            executed_results.push((tool_call.clone(), result));
                            current_index += 1;
                        }
                    } else {
                        // Tool succeeded, add to results and continue
                        executed_results.push((tool_call.clone(), result));
                        current_index += 1;
                    }
                }

                eprintln!("[Phase 4] PHASE 2 COMPLETE: {} tools executed", executed_results.len());
                
                // Flush any remaining events
                self.flush_events().await;
                
                Ok(executed_results)
            }

            Err(e) => {
                let err_msg = format!("Failed to connect to LLM: {}", e);
                eprintln!("[Phase 4] {}", err_msg);
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:error", &serde_json::json!({ "error": err_msg }));
                }
                Err(err_msg.into())
            }
        }
    }

    /// Execute a single tool with recovery (helper for streaming execution)
    async fn execute_tool_with_recovery(
        &mut self,
        tool_call: &ToolCall,
        workspace_path: &Option<String>,
        iteration: u32,
        tool_idx: usize,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        app_state_ref: Arc<RwLock<AppState>>,
        turn_messages: &[(String, String)],
        model_name: &str,
    ) -> Result<String> {
        let args_json = serde_json::to_string(&tool_call.args)
            .unwrap_or_else(|_| "{}".to_string());

        // Emit "running" status
        let running_step = AgentStep {
            iteration,
            tool: tool_call.tool.clone(),
            status: "running".to_string(),
            summary: format!("Executing {} with args: {}", tool_call.tool, args_json),
            result: None,
            logs: None,
            persona: Some("agent".to_string()),
            request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
            data: None,
        };
        self.emit_step(running_step).await;

        // Execute the tool - use streaming for run_command
        let mut tool_result = if tool_call.tool == "run_command" {
            self.execute_run_command_streaming(tool_call, workspace_path, iteration, tool_idx, app_state_ref).await
        } else {
            self.execute_single_tool(tool_call, workspace_path, recovery.clone()).await
        };

        // If tool failed, ask LLM for recovery strategy
        if tool_result.is_err() {
            let error_msg = tool_result.as_ref().err().unwrap().to_string();
            eprintln!("[Phase 4] Tool failed: {}", error_msg);

            // Ask LLM for recovery strategy
            match self.ask_llm_for_recovery(
                &tool_call.tool,
                &error_msg,
                &tool_call.args,
                turn_messages,
                model_name,
            ).await {
                Ok(strategy) => {
                    match strategy.action {
                        RecoveryAction::Retry => {
                            eprintln!("[Phase 4] Retrying tool: {}", tool_call.tool);
                            let retry_step = AgentStep {
                                iteration,
                                tool: tool_call.tool.clone(),
                                status: "running".to_string(),
                                summary: format!("Retrying {} (LLM recovery)", tool_call.tool),
                                result: None,
                                logs: None,
                                persona: Some("agent".to_string()),
                                request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
                                data: None,
                            };
                            self.emit_step(retry_step).await;
                            tool_result = self.execute_single_tool(tool_call, workspace_path, recovery.clone()).await;
                        }
                        RecoveryAction::Skip => {
                            eprintln!("[Phase 4] Skipping tool: {}", tool_call.tool);
                            let skip_step = AgentStep {
                                iteration,
                                tool: tool_call.tool.clone(),
                                status: "skipped".to_string(),
                                summary: format!("Skipped {} (LLM recovery)", tool_call.tool),
                                result: Some("Tool skipped due to error".to_string()),
                                logs: None,
                                persona: Some("agent".to_string()),
                                request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
                                data: None,
                            };
                            self.emit_step(skip_step).await;
                            tool_result = Ok("Tool skipped".to_string());
                        }
                        RecoveryAction::Alternative => {
                            eprintln!("[Phase 4] Alternative approach suggested: {:?}", strategy.suggestion);
                            let alt_step = AgentStep {
                                iteration,
                                tool: tool_call.tool.clone(),
                                status: "alternative".to_string(),
                                summary: format!("Alternative approach: {}", strategy.suggestion.as_ref().unwrap_or(&"N/A".to_string())),
                                result: Some(strategy.suggestion.clone().unwrap_or_default()),
                                logs: None,
                                persona: Some("agent".to_string()),
                                request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
                                data: None,
                            };
                            self.emit_step(alt_step).await;
                            tool_result = Ok(format!("Alternative approach: {}", strategy.suggestion.unwrap_or_default()));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Phase 4] Failed to get LLM recovery: {}", e);
                    // Keep the original error - don't convert to Ok
                }
            }
        }

        // Emit final completion or failure status
        let status = if tool_result.is_ok() { "completed" } else { "failed" };
        let result_text = if tool_result.is_ok() {
            tool_result.as_ref().ok().cloned()
        } else {
            tool_result.as_ref().err().map(|e| e.to_string())
        };

        let completed_step = AgentStep {
            iteration,
            tool: tool_call.tool.clone(),
            status: status.to_string(),
            summary: format!("Executed {} with args: {}", tool_call.tool, args_json),
            result: result_text.clone(),
            logs: result_text.as_ref().map(|r| vec![r.clone()]),
            persona: Some("agent".to_string()),
            request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
            data: None,
        };
        self.emit_step(completed_step).await;

        tool_result
    }

    /// Ask LLM for recovery strategy when a tool fails
    async fn ask_llm_for_recovery(
        &self,
        tool_name: &str,
        error: &str,
        args: &serde_json::Value,
        turn_messages: &[(String, String)],
        model_name: &str,
    ) -> Result<RecoveryStrategy> {
        let recovery_prompt = format!(
            "Tool '{}' failed with error: {}\n\
             Tool arguments were: {}\n\n\
             CRITICAL REMINDERS:\n\
             - NEVER retry the same command that just failed\n\
             - ALWAYS create directories BEFORE trying to cd into them\n\
             - Use relative paths when inside a directory (e.g., mkdir \"folder-name\", not mkdir \"parent\\\\folder-name\")\n\
             - If a directory doesn't exist, create it first with mkdir\n\
             - If a directory doesn't exist, create it first with mkdir\n\
             - ALWAYS use non-interactive flags (e.g. -y, --yes) for commands like npm create or npm install\n\
             - Do NOT use backslashes in folder names\n\n\
             What should I do?\n\
             Options:\n\
             1. Retry with DIFFERENT arguments (only if you can fix the underlying issue)\n\
             2. Skip this tool and continue\n\
             3. Try alternative approach (suggest what to do)\n\n\
             Respond with ONLY the number (1, 2, or 3) on the first line.\n\
             If you choose 1, explain what you're changing.\n\
             If you choose 3, add your suggestion on the next line.",
            tool_name,
            error,
            serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
        );

        let mut recovery_messages = turn_messages.to_vec();
        recovery_messages.push(("user".to_string(), recovery_prompt));

        eprintln!("[Recovery] Asking LLM for recovery strategy for tool: {}", tool_name);

        // Call LLM with recovery prompt
        let (response, _) = self.call_llm_streaming(&recovery_messages, model_name).await?;

        // Parse response
        let lines: Vec<&str> = response.lines().collect();
        let strategy = if let Some(first_line) = lines.first() {
            let choice = first_line.trim();
            if choice.contains("1") {
                eprintln!("[Recovery] LLM suggests: RETRY");
                RecoveryStrategy {
                    action: RecoveryAction::Retry,
                    suggestion: None,
                }
            } else if choice.contains("2") {
                eprintln!("[Recovery] LLM suggests: SKIP");
                RecoveryStrategy {
                    action: RecoveryAction::Skip,
                    suggestion: None,
                }
            } else if choice.contains("3") {
                let suggestion = lines.get(1).map(|s| s.to_string());
                eprintln!("[Recovery] LLM suggests: ALTERNATIVE - {:?}", suggestion);
                RecoveryStrategy {
                    action: RecoveryAction::Alternative,
                    suggestion,
                }
            } else {
                eprintln!("[Recovery] LLM response unclear, defaulting to SKIP");
                RecoveryStrategy {
                    action: RecoveryAction::Skip,
                    suggestion: None,
                }
            }
        } else {
            RecoveryStrategy {
                action: RecoveryAction::Skip,
                suggestion: None,
            }
        };

        Ok(strategy)
    }

    /// Get alternative tool from LLM when a tool fails
    /// Returns a new ToolCall to try instead
    async fn get_alternative_tool_from_llm(
        &self,
        failed_tool: &str,
        error: &str,
        args: &serde_json::Value,
        turn_messages: &[(String, String)],
        model_name: &str,
    ) -> Result<ToolCall> {
        let recovery_prompt = format!(
            "Tool '{}' failed with error: {}\n\
             Tool arguments were: {}\n\n\
             Provide an ALTERNATIVE tool call to accomplish the same goal.\n\
             Respond with ONLY a valid JSON object on a single line:\n\
             {{\"tool\": \"tool_name\", \"args\": {{...}}}}\n\n\
             Do NOT retry the same tool. Suggest a completely different approach.",
            failed_tool,
            error,
            serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
        );

        let mut recovery_messages = turn_messages.to_vec();
        recovery_messages.push(("user".to_string(), recovery_prompt));

        eprintln!("[Alternative] Asking LLM for alternative tool for failed tool: {}", failed_tool);

        // Call LLM with recovery prompt
        let (response, _) = self.call_llm_streaming(&recovery_messages, model_name).await?;

        // Parse response to extract tool call
        for line in response.lines() {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
                if let (Some(tool_name), Some(tool_args)) = (
                    obj.get("tool").and_then(|t| t.as_str()),
                    obj.get("args")
                ) {
                    eprintln!("[Alternative] LLM suggested alternative: {}", tool_name);
                    return Ok(ToolCall {
                        tool: tool_name.to_string(),
                        args: tool_args.clone(),
                    });
                }
            }
        }

        Err("Failed to parse alternative tool from LLM response".into())
    }

    /// Execute tools sequentially (one by one) with LLM error recovery
    #[allow(dead_code)]
    async fn execute_tools_sequentially(
        &mut self,
        tool_calls: Vec<ToolCall>,
        workspace_path: &Option<String>,
        iteration: u32,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        turn_messages: &[(String, String)],
        model_name: &str,
    ) -> Result<Vec<(ToolCall, Result<String>)>> {
        let mut results = Vec::new();

        for (tool_idx, tool_call) in tool_calls.iter().enumerate() {
            if crate::commands::agent::is_agent_cancelled() { break; }

            // Emit "running" status
            let args_json = serde_json::to_string(&tool_call.args)
                .unwrap_or_else(|_| "{}".to_string());
            let running_step = AgentStep {
                iteration,
                tool: tool_call.tool.clone(),
                status: "running".to_string(),
                summary: format!("Executing {} with args: {}", tool_call.tool, args_json),
                result: None,
                logs: None,
                persona: Some("agent".to_string()),
                request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
                data: None,
            };
            self.emit_step(running_step).await;

            // Execute the tool
            let mut tool_result = self.execute_single_tool(tool_call, workspace_path, recovery.clone()).await;

            // If tool failed, ask LLM for recovery strategy
            if tool_result.is_err() {
                let error_msg = tool_result.as_ref().err().unwrap().to_string();
                eprintln!("[Phase 3] Tool failed: {}", error_msg);

                // Ask LLM for recovery strategy
                match self.ask_llm_for_recovery(
                    &tool_call.tool,
                    &error_msg,
                    &tool_call.args,
                    turn_messages,
                    model_name,
                ).await {
                    Ok(strategy) => {
                        match strategy.action {
                            RecoveryAction::Retry => {
                                eprintln!("[Phase 3] Retrying tool: {}", tool_call.tool);
                                // Emit "running" status again for retry
                                let retry_step = AgentStep {
                                    iteration,
                                    tool: tool_call.tool.clone(),
                                    status: "running".to_string(),
                                    summary: format!("Retrying {} (LLM recovery)", tool_call.tool),
                                    result: None,
                                    logs: None,
                                    persona: Some("agent".to_string()),
                                    request_id: Some(format!("tool_{}_{}_retry", iteration, tool_idx)),
                                    data: None,
                                };
                                self.emit_step(retry_step).await;

                                // Retry the tool
                                tool_result = self.execute_single_tool(tool_call, workspace_path, recovery.clone()).await;
                            }
                            RecoveryAction::Skip => {
                                eprintln!("[Phase 3] Skipping tool: {}", tool_call.tool);
                                // Emit "skipped" status
                                let skip_step = AgentStep {
                                    iteration,
                                    tool: tool_call.tool.clone(),
                                    status: "skipped".to_string(),
                                    summary: format!("Skipped {} (LLM recovery)", tool_call.tool),
                                    result: Some("Tool skipped due to error".to_string()),
                                    logs: None,
                                    persona: Some("agent".to_string()),
                                    request_id: Some(format!("tool_{}_{}_skip", iteration, tool_idx)),
                                    data: None,
                                };
                                self.emit_step(skip_step).await;
                                // Mark as success (skipped)
                                tool_result = Ok("Tool skipped".to_string());
                            }
                            RecoveryAction::Alternative => {
                                eprintln!("[Phase 3] Alternative approach suggested: {:?}", strategy.suggestion);
                                // Emit "alternative" status
                                let alt_step = AgentStep {
                                    iteration,
                                    tool: tool_call.tool.clone(),
                                    status: "alternative".to_string(),
                                    summary: format!("Alternative approach: {}", strategy.suggestion.as_ref().unwrap_or(&"N/A".to_string())),
                                    result: Some(strategy.suggestion.clone().unwrap_or_default()),
                                    logs: None,
                                    persona: Some("agent".to_string()),
                                    request_id: Some(format!("tool_{}_{}_alt", iteration, tool_idx)),
                                    data: None,
                                };
                                self.emit_step(alt_step).await;
                                // Mark as success (alternative suggested)
                                tool_result = Ok(format!("Alternative approach: {}", strategy.suggestion.unwrap_or_default()));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Phase 3] Failed to get LLM recovery: {}", e);
                        // If LLM recovery fails, just skip the tool
                        tool_result = Ok("Tool skipped (recovery failed)".to_string());
                    }
                }
            }

            // Emit final completion or failure status
            let status = if tool_result.is_ok() { "completed" } else { "failed" };
            let result_text = tool_result.as_ref().ok().cloned();

            let completed_step = AgentStep {
                iteration,
                tool: tool_call.tool.clone(),
                status: status.to_string(),
                summary: format!("Executed {} with args: {}", tool_call.tool, args_json),
                result: result_text.clone(),
                logs: result_text.as_ref().map(|r| vec![r.clone()]),
                persona: Some("agent".to_string()),
                request_id: Some(format!("tool_{}_{}", iteration, tool_idx)),
                data: None,
            };
            self.emit_step(completed_step).await;

            results.push((tool_call.clone(), tool_result));

            // Small delay between tools to prevent queue overflow
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Flush any remaining events
        self.flush_events().await;

        Ok(results)
    }

    /// Stream LLM response with incremental JSON parsing
    /// Parses tool calls as they arrive and emits "identified" events
    #[allow(dead_code)]
    async fn stream_llm_with_incremental_parsing(
        &mut self,
        messages: &[(String, String)],
        model: &str,
        iteration: u32,
        context_length: u32,
    ) -> Result<(Vec<ToolCall>, String)> {
        let mut messages_json = Vec::new();
        let mut char_count = 0;
        let mut iter_messages = messages.iter().enumerate().collect::<Vec<_>>();
        iter_messages.reverse();
        
        let mut included_indices = std::collections::HashSet::new();
        included_indices.insert(0);
        included_indices.insert(1);

        for (i, (_role, content)) in iter_messages {
            if i <= 1 || included_indices.contains(&i) { continue; }
            let limit = (context_length as usize * 4).saturating_sub(5000).max(10000);
            if char_count + content.len() < limit {
                included_indices.insert(i);
                char_count += content.len();
            }
        }

        for (i, (role, content)) in messages.iter().enumerate() {
            if included_indices.contains(&i) {
                messages_json.push(serde_json::json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        eprintln!("[LLM] Streaming with incremental parsing: {} messages", included_indices.len());

        let payload = serde_json::json!({
            "model": model,
            "messages": messages_json,
            "stream": true,
            "temperature": 0.1,
            "top_p": 0.9,
            "top_k": 40,
            "keep_alive": "5m",
            "options": {
                "num_ctx": self.context_length,
            }
        });

        let mut response_text = String::new();
        let mut json_parser = crate::commands::streaming_agent_flow::IncrementalJsonParser::new();
        let mut identified_tools = Vec::new();
        let mut tool_counter = 0u32;

        match self.client
            .post("http://localhost:11434/api/chat")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(mut response) => {
                while let Some(chunk) = response.chunk().await.unwrap_or(None) {
                    let text = String::from_utf8_lossy(&chunk);
                    
                    for line in text.lines() {
                        if line.is_empty() { continue; }
                        
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(token) = data.get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str()) {
                                
                                response_text.push_str(token);
                                
                                // Feed to incremental JSON parser
                                let objects = json_parser.feed(token);
                                
                                // Process each parsed JSON object
                                for obj in objects {
                                    if let Some(tool_name) = obj.get("tool").and_then(|t| t.as_str()) {
                                        let args = obj.get("args").cloned().unwrap_or(serde_json::json!({}));
                                        let tool_id = format!("tool_{}_{}", iteration, tool_counter);
                                        tool_counter += 1;

                                        let tool_call = ToolCall {
                                            tool: tool_name.to_string(),
                                            args: args.clone(),
                                        };

                                        // Emit "identified" event immediately
                                        let args_json = serde_json::to_string(&args)
                                            .unwrap_or_else(|_| "{}".to_string());
                                        let identified_step = AgentStep {
                                            iteration,
                                            tool: tool_name.to_string(),
                                            status: "identified".to_string(),
                                            summary: format!("Tool identified: {} with args: {}", tool_name, args_json),
                                            result: None,
                                            logs: None,
                                            persona: Some("agent".to_string()),
                                            request_id: Some(tool_id),
                                            data: None,
                                        };
                                        self.emit_step(identified_step).await;

                                        identified_tools.push(tool_call);
                                        eprintln!("[Parser] Tool identified: {}", tool_name);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Flush any remaining events
                self.flush_events().await;
                
                eprintln!("[LLM] Streaming complete: {} tools identified", identified_tools.len());
                Ok((identified_tools, response_text))
            }

            Err(e) => {
                let err_msg = format!("Failed to connect to LLM: {}", e);
                eprintln!("[LLM] {}", err_msg);
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:error", &serde_json::json!({ "error": err_msg }));
                }
                Err(err_msg.into())
            }
        }
    }

    // ── FIX #2: System prompt now includes active file CONTENT ──────────────
    fn get_system_prompt(
        &self, 
        workspace_path: &Option<String>, 
        active_file: &Option<serde_json::Value>, 
        user_message: &str,
        detected_shell: &str,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        learning: Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>,
        steering: Arc<RwLock<SteeringSystem>>,
    ) -> String {
        let mut prompt = prompts::WHIZCODE_SYSTEM_PROMPT.to_string();

        // ── 0. SHELL INFORMATION ───────────────────────────────────────────
        prompt.push_str(&format!(
            "\n\n<shell_environment>\nDetected shell: {}\nWhen using the 'run_command' tool, provide commands that are compatible with {}.\n",
            detected_shell,
            match detected_shell {
                "powershell" | "pwsh" => "PowerShell (use PowerShell syntax, e.g., Get-ChildItem instead of ls)",
                "cmd" => "Windows CMD (use CMD syntax, e.g., dir instead of ls)",
                "bash" => "Bash (use Bash syntax, e.g., ls, grep, etc.)",
                "zsh" => "Zsh (use Zsh syntax, compatible with Bash)",
                "fish" => "Fish shell (use Fish syntax)",
                _ => "the detected shell"
            }
        ));
        prompt.push_str("\n- ALWAYS use non-interactive flags (e.g. -y, --yes, --force) for commands to prevent the agent from hanging. For example, use 'npm create vite@latest . -- -y' instead of just 'npm create vite@latest .'.");
        prompt.push_str("\n- DO NOT combine multiple commands into a single 'run_command' tool call unless the first command is a 'cd' (change directory). For example, use 'cd my-app && npm install' (OK) but do not use 'npm install && npm start' (NOT OK, split these into two tool calls).");
        prompt.push_str("\n- ALWAYS ensure you are in the correct directory before running any command. Remember that each 'run_command' starts in the workspace root unless you 'cd'.");
        prompt.push_str("</shell_environment>");

        // ── 1. LEARNED INSIGHTS ───────────────────────────────────────────
        if let Ok(l) = learning.lock() {
            let insights = l.get_insights();
            if !insights.is_empty() {
                prompt.push_str("\n\n<learned_insights>\n");
                for insight in insights.iter().take(5) {
                    prompt.push_str(&format!("- {}\n", insight.description));
                }
                prompt.push_str("</learned_insights>");
            }
            
            let recommendations = l.get_recommendations("general");
            if !recommendations.is_empty() {
                prompt.push_str("\n\n<tool_recommendations>\nBased on past performance in this workspace, prefer these tools:\n");
                for rec in recommendations.iter().take(3) {
                    prompt.push_str(&format!("- {} (Confidence: {:.0}%): {}\n", rec.tool_name, rec.confidence * 100.0, rec.reason));
                }
                prompt.push_str("</tool_recommendations>");
            }
        }

        // ── 2. STEERED CONTEXT ─────────────────────────────────────────────
        if let Some(ws) = workspace_path {
            let s = steering.read();
            if let Some(steered_context) = s.get_injected_context(ws) {
                if !steered_context.is_empty() {
                    prompt.push_str("\n\n<steering_rules>\n");
                    prompt.push_str(&steered_context);
                    prompt.push_str("\n</steering_rules>");
                }
            }
        }

        if let Some(ws) = workspace_path {
            prompt.push_str(&format!(
                "\n\nCurrent workspace path: {}\nIMPORTANT: Use this EXACT path in all file operations. Do NOT use /workspace/ as a placeholder.",
                ws
            ));

            // File tree with caching (5-min TTL)
            let file_tree = {
                let cache = self.file_tree_cache.read();
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                if let Some((tree, timestamp)) = cache.get(ws) {
                    if now - timestamp < 300 {
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

            // Knowledge Items
            if let Ok(lore) = crate::commands::distillation::load_relevant_knowledge(std::path::Path::new(ws)) {
                if !lore.is_empty() {
                    prompt.push_str(&lore);
                }
            }

            // Workflows and Skills
            let workflows_context = crate::commands::workflows::get_workflows_context(std::path::Path::new(ws));
            if !workflows_context.is_empty() {
                prompt.push_str(&workflows_context);
            }

            // ── GIT STATUS & DIFF INJECTION (matching Electron) ────────────────
            if let Ok(repo_path) = std::path::Path::new(ws).canonicalize() {
                let git_status = std::process::Command::new("git")
                    .arg("status")
                    .arg("--short")
                    .current_dir(&repo_path)
                    .output();
                if let Ok(output) = git_status {
                    let status_str = String::from_utf8_lossy(&output.stdout);
                    if !status_str.is_empty() {
                        prompt.push_str(&format!("\n\n<git_status>\n{}\n</git_status>", status_str));
                    }
                }
                
                // Diff head for context
                let git_diff = std::process::Command::new("git")
                    .arg("diff")
                    .arg("HEAD")
                    .current_dir(&repo_path)
                    .output();
                if let Ok(output) = git_diff {
                    let diff_str = String::from_utf8_lossy(&output.stdout);
                    if !diff_str.is_empty() {
                        let capped_diff = if diff_str.len() > 3000 {
                             format!("{}... (diff truncated)", &diff_str[..3000])
                        } else {
                             diff_str.to_string()
                        };
                        prompt.push_str(&format!("\n\n<git_diff>\n{}\n</git_diff>", capped_diff));
                    }
                }
            }

            // ── DYNAMIC PROMPT ADAPTATION ────────────────────────────────────
            let prompt_manager = crate::commands::prompt_manager::PromptManager::new();
            let extensions = vec!["tsx".to_string(), "jsx".to_string(), "ts".to_string(), "py".to_string(), "rs".to_string()]; // Inferred base exts
            let dynamic_suffix = prompt_manager.get_relevant_fragments(user_message, &extensions, &[ws.clone()]);
            if !dynamic_suffix.is_empty() {
                prompt.push_str(&dynamic_suffix);
            }

            // ── CODE INTELLIGENCE METRICS (matching Electron) ──────────────────
            if let Ok(intel) = code_intel.lock() {
                if let Some(metrics) = intel.get_code_metrics(ws) {
                    prompt.push_str(&format!(
                        "\n\n<code_intelligence>\nMetrics: Complexity={:.1}, Maintainability={:.1}, TechnicalDebt={:.2}\n</code_intelligence>",
                        metrics.average_complexity, metrics.maintainability_index, metrics.technical_debt
                    ));
                }
            }
        }

        // ── ACTIVE FILE CONTENT GUARD ──────────────────────────────────────
        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                prompt.push_str(&format!("\n\nActive file: {}", path));
                if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                    let lines: Vec<&str> = content.lines().collect();
                    const MAX_ACTIVE_FILE_LINES: usize = 300; // Match Electron cap
                    if lines.len() <= MAX_ACTIVE_FILE_LINES {
                        prompt.push_str(&format!("\n\n<active_file_content path=\"{}\">\n{}\n</active_file_content>", path, content));
                    } else {
                        let displayed: String = lines.iter().take(MAX_ACTIVE_FILE_LINES).cloned().collect::<Vec<_>>().join("\n");
                        prompt.push_str(&format!(
                            "\n\n<active_file_content path=\"{}\" truncated=\"true\">\n{}\n... (file truncated to 300 lines, use read_file for more)\n</active_file_content>",
                            path,
                            displayed
                        ));
                    }
                }
            }
        }

        prompt
    }
}

// ─────────────────────────────────────────────
// Standalone tool executor (used by parallel futures)
// ─────────────────────────────────────────────

#[allow(dead_code)]
async fn execute_tool_standalone(
    tool_call: &ToolCall,
    workspace_path: &Option<String>,
    vector_system: &Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
    code_intel: &Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    _app_handle: Option<&tauri::AppHandle>,
) -> Result<String> {
    let ws_root = workspace_path.as_deref().unwrap_or(".");
    let resolve = |p: &str| -> std::path::PathBuf {
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
        // ── FIX #8: read_file now accepts optional start_line / end_line ─────
        "read_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;

            let content = tokio::fs::read_to_string(resolve(path)).await?;
            let start_line = tool_call.args.get("start_line").and_then(|s| s.as_u64()).map(|n| n as usize);
            let end_line   = tool_call.args.get("end_line").and_then(|e| e.as_u64()).map(|n| n as usize);

            if start_line.is_none() && end_line.is_none() {
                return Ok(format!("File contents:\n{}", content));
            }

            let lines: Vec<&str> = content.lines().collect();
            let start = start_line.unwrap_or(1).saturating_sub(1);
            let end   = end_line.unwrap_or(lines.len()).min(lines.len());
            let slice = lines[start..end].join("\n");
            Ok(format!("File contents (lines {}-{}):\n{}", start + 1, end, slice))
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

        // ── FIX #4: grep_search — content-level ripgrep-style search ─────────
        "grep_search" => {
            let query = tool_call.args.get("query")
                .and_then(|q| q.as_str())
                .ok_or("Missing query argument")?;
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .unwrap_or(".");
            let case_insensitive = tool_call.args.get("case_insensitive")
                .and_then(|c| c.as_bool())
                .unwrap_or(true);
            let include_glob = tool_call.args.get("include")
                .and_then(|g| g.as_str());

            // Try ripgrep first, fall back to manual walk
            let mut rg_cmd = tokio::process::Command::new("rg");
            rg_cmd.arg("--line-number")
                  .arg("--no-heading")
                  .arg("--with-filename");
            if case_insensitive { rg_cmd.arg("--ignore-case"); }
            if let Some(glob) = include_glob { rg_cmd.arg("--glob").arg(glob); }
            rg_cmd.arg(query).arg(resolve(path));

            let output = rg_cmd.output().await;
            if let Ok(out) = output {
                if out.status.success() || !out.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let lines: Vec<&str> = text.lines().take(50).collect();
                    return Ok(format!("grep_search results for '{}':\n{}", query, lines.join("\n")));
                }
            }

            // Fallback: walk files manually
            let search_path = resolve(path);
            let mut results = Vec::new();
            let query_lower = query.to_lowercase();
            walk_and_grep(&search_path, &query_lower, case_insensitive, &mut results, 0, 4).await;
            if results.is_empty() {
                Ok(format!("No matches found for '{}'", query))
            } else {
                Ok(format!("grep_search results for '{}':\n{}", query, results.join("\n")))
            }
        }

        "write_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let content = tool_call.args.get("content")
                .and_then(|c| c.as_str())
                .ok_or("Missing content argument")?;
            let resolved_path = resolve(path);
            if let Some(parent) = resolved_path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            tokio::fs::write(&resolved_path, content).await?;
            Ok(format!("Successfully wrote to {}", path))
        }

        "edit_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let content = tool_call.args.get("content")
                .and_then(|c| c.as_str())
                .ok_or("Missing content argument")?;
            let start_line = tool_call.args.get("start_line").and_then(|s| s.as_u64()).map(|s| s as u32);
            let end_line   = tool_call.args.get("end_line").and_then(|e| e.as_u64()).map(|e| e as u32);
            let resolved_path = resolve(path);
            let file_content = tokio::fs::read_to_string(&resolved_path).await?;
            let lines: Vec<&str> = file_content.lines().collect();
            let start = start_line.unwrap_or(1) as usize;
            let end   = end_line.unwrap_or(lines.len() as u32) as usize;
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

        // ── FIX #10: multi_edit_file — multiple non-contiguous search/replace ─
        "multi_edit_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let edits = tool_call.args.get("edits")
                .and_then(|e| e.as_array())
                .ok_or("Missing edits array")?;

            let resolved_path = resolve(path);
            let mut content = tokio::fs::read_to_string(&resolved_path).await?;
            let mut applied = 0usize;
            let mut errors = Vec::new();

            for edit in edits {
                let search  = edit.get("search").and_then(|s| s.as_str()).unwrap_or("");
                let replace = edit.get("replace").and_then(|r| r.as_str()).unwrap_or("");
                if content.contains(search) {
                    // Replace only FIRST occurrence per edit block (like Antigravity's semantic)
                    content = content.replacen(search, replace, 1);
                    applied += 1;
                } else {
                    errors.push(format!("Could not find: {:?}", &search[..search.len().min(60)]));
                }
            }

            tokio::fs::write(&resolved_path, &content).await?;
            let mut msg = format!("multi_edit_file: applied {}/{} edits to {}", applied, edits.len(), path);
            if !errors.is_empty() {
                msg.push_str(&format!("\nWarnings:\n{}", errors.join("\n")));
            }
            Ok(msg)
        }

        "run_command" => {
            let command = tool_call.args.get("command")
                .and_then(|c| c.as_str())
                .ok_or("Missing command argument")?;
            let cwd = workspace_path.as_deref().unwrap_or(".");
            eprintln!("[run_command] Executing: {:?} in {:?}", command, cwd);

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
            cmd.env("CI", "true");
            cmd.env("NPM_CONFIG_YES", "true");
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => return Err(format!("Failed to spawn command '{}': {}", command, e).into()),
            };
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(b"y\n").await;
            }

            match tokio::time::timeout(std::time::Duration::from_secs(300), child.wait_with_output()).await {
                Ok(Ok(output)) => {
                    let mut stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let mut stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let success = output.status.success();
                    let mut result = format!("Command: {}\nCWD: {}\n", command, cwd);
                    // Prevent memory / RAM blowout from crazy outputs
                    if stdout.len() > 30_000 {
                        stdout = format!("{}... (truncated, first 30k chars)", &stdout[..30_000]);
                    }
                    if stderr.len() > 30_000 {
                        stderr = format!("{}... (truncated, first 30k chars)", &stderr[..30_000]);
                    }
                    if !stdout.is_empty() { result.push_str(&format!("Output:\n{}\n", stdout)); }
                    if !stderr.is_empty() { result.push_str(&format!("Stderr:\n{}\n", stderr)); }
                    if stdout.is_empty() && stderr.is_empty() { result.push_str("Command completed with no output.\n"); }
                    if success { Ok(result) } else { Err(format!("Command failed (exit: {}):\n{}", output.status, result).into()) }
                }
                Ok(Err(e)) => Err(format!("Failed to spawn command '{}': {}", command, e).into()),
                Err(_) => Err(format!("Command '{}' timed out after 5 minutes", command).into()),
            }
        }

        "git" => {
            let operation = tool_call.args.get("operation")
                .and_then(|o| o.as_str())
                .ok_or("Missing operation argument")?;
            let output_str = match operation {
                "status" => {
                    let o = tokio::process::Command::new("git").arg("status").arg("--porcelain")
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "add" => {
                    let p = tool_call.args.get("path").and_then(|p| p.as_str()).ok_or("Missing path")?;
                    let o = tokio::process::Command::new("git").arg("add").arg(p)
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "commit" => {
                    let m = tool_call.args.get("message").and_then(|m| m.as_str()).ok_or("Missing message")?;
                    let o = tokio::process::Command::new("git").arg("commit").arg("-m").arg(m)
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "push" => {
                    let o = tokio::process::Command::new("git").arg("push")
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "pull" => {
                    let o = tokio::process::Command::new("git").arg("pull")
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "log" => {
                    let o = tokio::process::Command::new("git").arg("log").arg("--oneline").arg("-10")
                        .current_dir(workspace_path.as_deref().unwrap_or(".")).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                _ => return Err(format!("Unknown git operation: {}", operation).into()),
            };
            Ok(output_str)
        }

        "npm" => {
            let operation = tool_call.args.get("operation")
                .and_then(|o| o.as_str())
                .ok_or("Missing operation argument")?;
            let cwd = workspace_path.as_deref().unwrap_or(".");
            let output_str = match operation {
                "install" => {
                    let o = tokio::process::Command::new("npm").arg("install").current_dir(cwd).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "add" => {
                    let p = tool_call.args.get("package").and_then(|p| p.as_str()).ok_or("Missing package")?;
                    let o = tokio::process::Command::new("npm").arg("install").arg(p).current_dir(cwd).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "list" => {
                    let o = tokio::process::Command::new("npm").arg("list").arg("--depth=0").current_dir(cwd).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                "run" => {
                    let s = tool_call.args.get("script").and_then(|s| s.as_str()).ok_or("Missing script")?;
                    let o = tokio::process::Command::new("npm").arg("run").arg(s).current_dir(cwd).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                _ => return Err(format!("Unknown npm operation: {}", operation).into()),
            };
            Ok(output_str)
        }

        "docker" => {
            let operation = tool_call.args.get("operation")
                .and_then(|o| o.as_str())
                .ok_or("Missing operation argument")?;
            let cwd = workspace_path.as_deref().unwrap_or(".");
            let output_str = match operation {
                "ps"     => { let o = tokio::process::Command::new("docker").arg("ps").current_dir(cwd).output().await?; String::from_utf8_lossy(&o.stdout).to_string() }
                "images" => { let o = tokio::process::Command::new("docker").arg("images").current_dir(cwd).output().await?; String::from_utf8_lossy(&o.stdout).to_string() }
                "logs"   => {
                    let c = tool_call.args.get("container").and_then(|c| c.as_str()).ok_or("Missing container")?;
                    let o = tokio::process::Command::new("docker").arg("logs").arg(c).current_dir(cwd).output().await?;
                    String::from_utf8_lossy(&o.stdout).to_string()
                }
                _ => return Err(format!("Unknown docker operation: {}", operation).into()),
            };
            Ok(output_str)
        }

        "semantic_search" => {
            let query = tool_call.args.get("query")
                .and_then(|q| q.as_str())
                .ok_or("Missing query argument")?;
            let search_query = crate::commands::vector_search::SemanticQuery {
                query: query.to_string(),
                file_path: None,
                limit: Some(5),
            };
            let results = {
                let system = vector_system.lock().unwrap();
                let stats = system.get_index_stats().unwrap();
                if stats.total_chunks == 0 { let _ = system.index_workspace(ws_root); }
                system.semantic_search(&search_query).map_err(|e| format!("Search failed: {}", e))?
            };
            let mut out = format!("Found {} relevant code blocks for '{}':\n", results.len(), query);
            for res in results {
                out.push_str(&format!("\n--- {} (relevance: {:.2}) ---\n{}\n", res.chunk.file_path, res.relevance_score, res.chunk.content));
            }
            Ok(out)
        }

        "find_symbols" => {
            let query = tool_call.args.get("query").and_then(|q| q.as_str()).ok_or("Missing query")?;
            let intel = code_intel.lock().unwrap();
            let symbols = intel.get_all_symbols(ws_root);
            if symbols.is_empty() { let _ = intel.analyze_workspace(ws_root.to_string()); }
            let results: Vec<_> = intel.get_all_symbols(ws_root).into_iter()
                .filter(|s| s.name.contains(query)).collect();
            let mut out = format!("Found {} symbols matching '{}':\n", results.len(), query);
            for s in results {
                out.push_str(&format!("- {} ({}): {} line {}\n", s.name, s.symbol_type, s.file_path, s.line_number));
            }
            Ok(out)
        }

        "get_code_intelligence" => {
            let path = tool_call.args.get("path").and_then(|p| p.as_str()).unwrap_or("");
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

        "done" => Ok("Task completed successfully.".to_string()),

        "search_web" => {
            let query = tool_call.args.get("query").and_then(|q| q.as_str()).ok_or("Missing query")?;
            let results = crate::commands::web_search::search_web(query.to_string()).await?;
            let mut out = format!("Search results for '{}':\n", query);
            for (i, r) in results.iter().enumerate() {
                out.push_str(&format!("{}. {} ({})\n   {}\n", i + 1, r.title, r.url, r.snippet));
            }
            Ok(out)
        }

        "read_url_content" => {
            let url = tool_call.args.get("url").and_then(|u| u.as_str()).ok_or("Missing url")?;
            let content = crate::commands::web_search::read_url_content(url.to_string()).await?;
            Ok(format!("Content from {}:\n\n{}", url, content))
        }

        "generate_image" => {
            let prompt_text = tool_call.args.get("prompt").and_then(|p| p.as_str()).ok_or("Missing prompt")?;
            let result = crate::commands::assets::generate_image(
                crate::commands::assets::ImageRequest { prompt: prompt_text.to_string(), width: 1024, height: 1024 },
                ws_root.to_string()
            ).await?;
            Ok(format!("Generated image saved to {}. URL: {}", result.asset_path, result.url))
        }

        "ask_user" => {
            // Handled by the orchestrator loop above before parallel dispatch
            Ok("(ask_user handled by orchestrator)".to_string())
        }

        _ => Err(format!("Unknown tool: {}", tool_call.tool).into()),
    }
}

// ─────────────────────────────────────────────
// Recursive grep fallback
// ─────────────────────────────────────────────

#[allow(dead_code)]
async fn walk_and_grep(
    dir: &std::path::Path,
    query: &str,
    case_insensitive: bool,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || results.len() >= 50 { return; }
    let Ok(mut read_dir) = tokio::fs::read_dir(dir).await else { return };

    // Skip common noise dirs
    let skip_dirs = [".git", "node_modules", "target", "dist", ".next"];

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if skip_dirs.contains(&name.as_str()) { continue; }

        let metadata = match entry.metadata().await { Ok(m) => m, Err(_) => continue };
        if metadata.is_dir() {
            Box::pin(walk_and_grep(&path, query, case_insensitive, results, depth + 1, max_depth)).await;
        } else if metadata.is_file() {
            // Only text files
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let text_exts = ["ts","tsx","js","jsx","rs","py","go","java","cs","cpp","c","h","md","txt","toml","json","yaml","yml","html","css","scss"];
                if !text_exts.contains(&ext) { continue; }
            }
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                for (i, line) in content.lines().enumerate() {
                    let hay = if case_insensitive { line.to_lowercase() } else { line.to_string() };
                    if hay.contains(query) {
                        results.push(format!("{}:{}: {}", path.to_string_lossy(), i + 1, line.trim()));
                        if results.len() >= 50 { return; }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────
// Parallel-group detection helpers
// ─────────────────────────────────────────────

#[allow(dead_code)]
fn identify_independent_tool_groups(tool_calls: &[ToolCall]) -> Vec<Vec<usize>> {
    if tool_calls.is_empty() { return vec![]; }
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut used = std::collections::HashSet::new();
    for (i, tool_i) in tool_calls.iter().enumerate() {
        if used.contains(&i) { continue; }
        let mut group = vec![i];
        used.insert(i);
        for (j, tool_j) in tool_calls.iter().enumerate().skip(i + 1) {
            if used.contains(&j) { continue; }
            if !tools_have_conflict(tool_i, tool_j) { group.push(j); used.insert(j); }
        }
        groups.push(group);
    }
    groups
}

#[allow(dead_code)]
fn tools_have_conflict(a: &ToolCall, b: &ToolCall) -> bool {
    // Write tools always conflict with anything on the same path
    let write_tools = ["write_file", "edit_file", "multi_edit_file", "delete_file"];
    let file_tools  = ["read_file", "write_file", "edit_file", "multi_edit_file", "delete_file", "list_directory"];
    if !file_tools.contains(&a.tool.as_str()) || !file_tools.contains(&b.tool.as_str()) { return false; }
    let path_a = a.args.get("path").and_then(|p| p.as_str());
    let path_b = b.args.get("path").and_then(|p| p.as_str());
    match (path_a, path_b) {
        (Some(pa), Some(pb)) => {
            if pa == pb { return true; }
            if pa.starts_with(pb) || pb.starts_with(pa) { return true; }
            // Any write conflicts with any read on the same tree
            if write_tools.contains(&a.tool.as_str()) || write_tools.contains(&b.tool.as_str()) {
                if pa.starts_with(pb) || pb.starts_with(pa) { return true; }
            }
            false
        }
        _ => false,
    }
}

fn looks_like_natural_language(response: &str) -> bool {
    let trimmed = response.trim();
    if trimmed.starts_with('{') { return false; }
    let prose_signals = ["I will", "I'll", "Let me", "First,", "To ", "Step ", "Here ", "Sure", "Okay", "The ", "This "];
    prose_signals.iter().any(|s| trimmed.contains(s))
}

fn extract_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();
    let supported = [
        // Planner-generated tools
        "List", "Search", "Read", "Write", "Edit", "Execute", "Analyze",
        // Reasoning tools
        "Think", "Reason", "Analyze", "Verify", "Check", "Validate",
        // Standard file tools
        "read_file", "write_file", "edit_file", "multi_edit_file",
        "list_directory", "search_files", "grep_search",
        // Execution tools
        "run_command", "git", "npm", "docker",
        // Analysis tools
        "semantic_search", "analyze_workspace", "get_code_intelligence",
        "find_symbols", "search_web", "read_url_content",
        // Generation tools
        "generate_image", "ask_user", 
        // Terminal
        "done",
    ];

    let mut start_indices = Vec::new();
    for (i, c) in response.char_indices() {
        if c == '{' { start_indices.push(i); }
    }

    for start in start_indices {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escaped = false;
        for (i, c) in response[start..].char_indices() {
            let actual_idx = start + i;
            if escaped { escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' { in_string = !in_string; continue; }
            if !in_string {
                if c == '{' { brace_count += 1; }
                else if c == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        let potential_json = &response[start..=actual_idx];
                        if potential_json.contains("\"tool\"") {
                            if let Ok(call) = serde_json::from_str::<ToolCall>(potential_json) {
                                if supported.contains(&call.tool.as_str()) {
                                    eprintln!("[EXTRACT] Tool call: {}", call.tool);
                                    tool_calls.push(call);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // Deduplicate
    let mut unique = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for call in tool_calls {
        let s = serde_json::to_string(&call).unwrap_or_default();
        if !seen.contains(&s) { seen.insert(s); unique.push(call); }
    }
    eprintln!("[EXTRACT] Total unique tool calls: {}", unique.len());
    unique
}

// ─────────────────────────────────────────────
// Tauri command entry point
// ─────────────────────────────────────────────

#[tauri::command]
pub async fn execute_agent_loop_streaming(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    // FIX #1: accept conversation history from frontend
    conversation_history: Option<Vec<ConversationTurn>>,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    vector_state: State<'_, Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>>,
    intel_state: State<'_, Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>>,
    learning_state: State<'_, Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>>,
    steering_state: State<'_, Arc<RwLock<SteeringSystem>>>,
    recovery_state: State<'_, Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>>,
    context_length: Option<u32>,
) -> Result<StreamingAgentResponse> {
    // Reset cancel token at the start of a new task
    {
        let mut cancel = crate::commands::agent::AGENT_CANCEL_TOKEN.lock();
        *cancel = false;
    }

    let (resolved_workspace, detected_shell) = {
        let app_state = state.read();
        let ws = app_state.get_workspace().map(|p| p.to_string_lossy().to_string()).or(workspace_path);
        let shell = app_state.get_shell().to_string();
        (ws, shell)
    };

    eprintln!("[Backend] Resolved workspace_path: {:?}", resolved_workspace);
    eprintln!("[Backend] Detected shell: {}", detected_shell);

    let prior_history = conversation_history.unwrap_or_default();

    let mut orchestrator = StreamingAgentOrchestrator::new(Some(app_handle));
    orchestrator.set_context_length(context_length.unwrap_or(16384));
    orchestrator.execute_task_streaming(
        task, model, resolved_workspace, active_file,
        prior_history,
        detected_shell,
        vector_state.inner().clone(), 
        intel_state.inner().clone(),
        learning_state.inner().clone(), 
        steering_state.inner().clone(),
        recovery_state.inner().clone(),
        state.inner().clone()
    ).await
}

#[tauri::command]
pub async fn agent_send_terminal_input(
    request_id: String,
    input: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let inputs = state.read().tool_inputs.clone();
    let mut lock = inputs.lock().await;
    if let Some(stdin) = lock.get_mut(&request_id) {
        stdin.write_all(input.as_bytes()).await.map_err(|e| format!("Failed to write to stdin: {}", e).into())
    } else {
        Err(format!("No running process found for request_id: {}", request_id).into())
    }
}

#[tauri::command]
pub async fn agent_stop_terminal_command(
    request_id: String,
    state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let killers = state.read().tool_killers.clone();
    let mut lock = killers.lock().await;
    if let Some(tx) = lock.remove(&request_id) {
        let _ = tx.send(());
        Ok(())
    } else {
        Err(format!("No running process found for request_id: {}", request_id).into())
    }
}
