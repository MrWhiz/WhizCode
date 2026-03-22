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

pub struct StreamingAgentOrchestrator {
    max_iterations: u32,
    app_handle: Option<tauri::AppHandle>,
    suppress_stream: bool,
    file_tree_cache: Arc<RwLock<HashMap<String, (String, u64)>>>,
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
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    ) -> Result<StreamingAgentResponse> {
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        eprintln!("[Backend] Received workspace_path: {:?}", workspace_path);
        eprintln!("[Backend] Prior history turns: {}", prior_history.len());

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut all_tool_calls = Vec::new();
        let mut total_tokens = 0u32;
        let mut status = "done".to_string();

        // Emit start
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "planning",
                "status": "started",
                "description": "Planning task"
            }));
        }

        // ── FIX #3: Planning step at the very start ──────────────────────────
        let plan = crate::commands::planning::PlanningSystem::new()
            .create_plan(&task, &workspace_path);
        let plan_step = AgentStep {
            iteration: 0,
            tool: "planning".to_string(),
            status: "done".to_string(),
            summary: format!("Plan: {} tasks, risk={}", plan.tasks.len(), plan.risk_level),
            result: None,
            logs: None,
            persona: Some("planner".to_string()),
            request_id: None,
            data: Some(serde_json::json!({ "plan": plan })),
        };
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:step", &plan_step);
        }
        steps.push(plan_step);

        // ── FIX #2: Build system prompt with active file CONTENT ─────────────
        let system_prompt = self.get_system_prompt(&workspace_path, &active_file);

        // ── FIX #1: Seed turn_messages with prior conversation history ────────
        let mut turn_messages: Vec<(String, String)> = vec![
            ("system".to_string(), system_prompt),
        ];

        // Inject up to the last 10 prior turns so the LLM has multi-turn context.
        // We skip the very first "Hello! I'm your WhizCode agent" assistant message.
        const MAX_HISTORY_TURNS: usize = 10;
        let history_to_inject = if prior_history.len() > MAX_HISTORY_TURNS {
            &prior_history[prior_history.len() - MAX_HISTORY_TURNS..]
        } else {
            &prior_history[..]
        };
        for turn in history_to_inject {
            turn_messages.push((turn.role.clone(), turn.content.clone()));
        }

        // Current task message
        turn_messages.push((
            "user".to_string(),
            format!("Task: {}\nPlease analyze the task, explore the codebase if needed, execute the necessary changes, and verify the results.", task.clone()),
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

            let (response, tokens) = self.call_llm_streaming(&turn_messages, model_name).await?;
            total_tokens += tokens;

            let mut tool_calls = extract_tool_calls(&response);
            eprintln!("[Agent] LLM response length: {}, extracted {} tool calls", response.len(), tool_calls.len());

            // Natural-language correction retry
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
                    summary: "Completed reasoning".to_string(),
                    result: Some(response.clone()),
                    logs: Some(vec![response.clone()]),
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

            turn_messages.push(("assistant".to_string(), response.clone()));

            let mut tool_results = Vec::new();
            let mut done = false;

            // ── FIX #5: Truly parallel tool execution ─────────────────────────
            let tool_groups = identify_independent_tool_groups(&tool_calls);
            eprintln!("[Agent] {} tool groups from {} tools", tool_groups.len(), tool_calls.len());

            'groups: for (group_idx, group) in tool_groups.iter().enumerate() {
                eprintln!("[Agent] Executing group {} ({} tools) in parallel", group_idx + 1, group.len());
                let group_start = std::time::Instant::now();

                // Check for 'done' in group first
                for &tool_idx in group {
                    if tool_calls[tool_idx].tool == "done" {
                        done = true;
                        break 'groups;
                    }
                    // ── FIX #9: ask_user mid-task clarification ───────────────
                    if tool_calls[tool_idx].tool == "ask_user" {
                        let question = tool_calls[tool_idx].args.get("question")
                            .and_then(|q| q.as_str())
                            .unwrap_or("What would you like me to do next?");
                        if let Some(app) = &self.app_handle {
                            let req_id = format!("ask_{}", iteration);
                            let ask_step = AgentStep {
                                iteration,
                                tool: "ask_user".to_string(),
                                status: "awaiting_permission".to_string(),
                                summary: question.to_string(),
                                result: None,
                                logs: None,
                                persona: Some("agent".to_string()),
                                request_id: Some(req_id.clone()),
                                data: None,
                            };
                            let _ = app.emit("agent:step", &ask_step);
                            // Wait for user response via permission channel
                            let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                            {
                                let mut lock = crate::commands::agent::PERMISSION_TX.lock().unwrap();
                                *lock = Some(tx);
                            }
                            let user_approved = tokio::time::timeout(
                                std::time::Duration::from_secs(120),
                                rx
                            ).await.unwrap_or(Ok(false)).unwrap_or(false);
                            let answer = if user_approved { "User says: proceed / yes" } else { "User says: skip / no" };
                            tool_results.push(format!("[ask_user] {}", answer));
                        }
                        continue;
                    }
                }
                if done { break; }

                // Build futures for parallel execution
                let futures: Vec<_> = group.iter().map(|&tool_idx| {
                    let tc = tool_calls[tool_idx].clone();
                    let wp = workspace_path.clone();
                    let vs = vector_system.clone();
                    let ci = code_intel.clone();
                    let self_handle = self.app_handle.clone();
                    let iter = iteration;
                    async move {
                        let start = std::time::Instant::now();
                        let result = execute_tool_standalone(&tc, &wp, &vs, &ci, self_handle.as_ref()).await;
                        let elapsed = start.elapsed().as_millis();
                        (tc, result, elapsed, iter)
                    }
                }).collect();

                // Run all futures in this group concurrently
                let group_results = futures::future::join_all(futures).await;

                for (tc, tool_result, _elapsed, iter) in group_results {
                    let args_json = serde_json::to_string(&tc.args).unwrap_or_else(|_| "{}".to_string());
                    let step = AgentStep {
                        iteration: iter,
                        tool: tc.tool.clone(),
                        status: if tool_result.is_ok() { "done".to_string() } else { "failed".to_string() },
                        summary: format!("Executed {} with args: {}", tc.tool, args_json),
                        result: tool_result.as_ref().ok().cloned(),
                        logs: tool_result.as_ref().ok().map(|s| vec![s.clone()]),
                        persona: Some("agent".to_string()),
                        request_id: None,
                        data: None,
                    };

                    if let Some(app) = &self.app_handle {
                        let mut ui_step = step.clone();
                        // FIX: Aggressive truncation to prevent Windows IPC PostMessage queue overflow
                        if let Some(res) = &mut ui_step.result {
                            if res.len() > 500 {
                                *res = format!("{}... (truncated for UI)", &res[..500]);
                            }
                        }
                        if let Some(logs) = &mut ui_step.logs {
                            for log in logs.iter_mut() {
                                if log.len() > 500 {
                                    *log = format!("{}... (truncated for UI)", &log[..500]);
                                }
                            }
                        }
                        let _ = app.emit("agent:step", &ui_step);
                    }

                    match &tool_result {
                        Ok(r)  => tool_results.push(format!("[{}] result:\n{}", tc.tool, r)),
                        Err(e) => tool_results.push(format!("[{}] error:\n{}", tc.tool, e)),
                    }
                    all_tool_calls.push(tc.clone());
                    steps.push(step);
                }

                let group_elapsed = group_start.elapsed().as_millis();
                eprintln!("[Agent] Group {} completed in {}ms", group_idx + 1, group_elapsed);
            }

            if done { break; }

            if !tool_results.is_empty() {
                let results_msg = format!(
                    "Tool results:\n{}\n\nContinue with more tool calls or output {{\"tool\": \"done\", \"args\": {{}}}} when finished.",
                    tool_results.join("\n\n")
                );
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
            if char_count + content.len() < 15_000 {
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

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": model,
            "messages": messages_json,
            "stream": true,
            "temperature": 0.1,
            "top_p": 0.9,
            "top_k": 40,
            "options": {
                "num_ctx": 16000,
            }
        });

        let mut response_text = String::new();
        let mut token_count = 0u32;
        let mut token_batch = String::new();
        // FIX: 100-token batches prevent Windows message-queue overflow
        const BATCH_SIZE: usize = 100;

        match client
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

    // ── FIX #2: System prompt now includes active file CONTENT ──────────────
    fn get_system_prompt(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
        let mut prompt = prompts::WHIZCODE_SYSTEM_PROMPT.to_string();

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
        }

        // FIX #2: Inject active file path AND content
        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                prompt.push_str(&format!("\n\nActive file: {}", path));
                if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                    // Truncate large files to avoid bloating the prompt
                    const MAX_CONTENT_CHARS: usize = 8000;
                    if content.len() <= MAX_CONTENT_CHARS {
                        prompt.push_str(&format!("\n\n<active_file_content path=\"{}\">\n{}\n</active_file_content>", path, content));
                    } else {
                        prompt.push_str(&format!(
                            "\n\n<active_file_content path=\"{}\" truncated=\"true\">\n{}\n... (file truncated, use read_file with start_line/end_line for more)\n</active_file_content>",
                            path,
                            &content[..MAX_CONTENT_CHARS]
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
        "read_file", "write_file", "edit_file", "multi_edit_file",
        "list_directory", "search_files", "grep_search",
        "run_command", "git", "npm", "docker",
        "semantic_search", "analyze_workspace", "get_code_intelligence",
        "find_symbols", "search_web", "read_url_content",
        "generate_image", "ask_user", "done",
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
) -> Result<StreamingAgentResponse> {
    let resolved_workspace = {
        let app_state = state.read();
        app_state.get_workspace().map(|p| p.to_string_lossy().to_string())
    }.or(workspace_path);

    eprintln!("[Backend] Resolved workspace_path: {:?}", resolved_workspace);

    let prior_history = conversation_history.unwrap_or_default();

    let mut orchestrator = StreamingAgentOrchestrator::new(Some(app_handle));
    orchestrator.execute_task_streaming(
        task, model, resolved_workspace, active_file,
        prior_history,
        vector_state.inner().clone(), intel_state.inner().clone()
    ).await
}
