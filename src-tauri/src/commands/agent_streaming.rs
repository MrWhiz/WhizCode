use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use crate::commands::problem_identifier::ProblemIdentifier;
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
use regex::Regex;

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

fn extract_completion_logs(tool_name: &str, result_text: Option<&String>) -> Option<Vec<String>> {
    match result_text {
        Some(result) if tool_name == "run_command" => {
            if let Some((_, logs)) = result.split_once("\nLogs:\n") {
                Some(vec![logs.to_string()])
            } else {
                Some(vec![result.clone()])
            }
        }
        Some(result) => Some(vec![result.clone()]),
        None => None,
    }
}

fn is_high_risk_command(command: &str) -> bool {
    let normalized = command.to_lowercase();
    let high_risk_patterns = [
        "rm -rf",
        "remove-item -recurse -force",
        "git reset --hard",
        "git checkout --",
        "del /f",
        "rmdir /s",
        "format ",
        "drop database",
        "truncate table",
    ];

    high_risk_patterns
        .iter()
        .any(|pattern| normalized.contains(pattern))
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
        let is_critical = matches!(
            step.status.as_str(),
            "completed" | "failed" | "running" | "skipped" | "alternative" | "awaiting_permission"
        );
        
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

    async fn request_iteration_continuation(&mut self, iteration: u32) -> Result<bool> {
        let request_id = format!("iteration_limit_{}", iteration);
        let question = format!(
            "WhizCode has reached {} iterations without finishing this task. Do you want it to continue for 30 more iterations?",
            iteration
        );

        let step = AgentStep {
            iteration,
            tool: "reasoning".to_string(),
            status: "awaiting_permission".to_string(),
            summary: question.clone(),
            result: None,
            logs: Some(vec![
                "Iteration safety limit reached.".to_string(),
                "Approve to continue the current task for another 30 iterations.".to_string(),
            ]),
            persona: Some("agent".to_string()),
            request_id: Some(request_id),
            data: Some(serde_json::json!({
                "reason": "iteration_limit",
                "iteration": iteration,
                "extension": 30,
            })),
        };
        self.emit_step(step).await;

        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        {
            let mut permission_tx = crate::commands::agent::PERMISSION_TX.lock().unwrap();
            *permission_tx = Some(tx);
        }

        match rx.await {
            Ok(approved) => Ok(approved),
            Err(_) => Err("Failed waiting for user response at iteration limit.".into()),
        }
    }

    // estimate_codebase_size removed — the stub research phase that used it was
    // injecting fabricated text into every prompt. Real research is a future feature.

    pub async fn execute_task_streaming(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
        // FIX #1: accept prior conversation history from the frontend
        prior_history: Vec<ConversationTurn>,
        detected_shell: String,
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        learning: Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>,
        steering: Arc<RwLock<SteeringSystem>>,
        recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        app_state_ref: Arc<RwLock<AppState>>,
        context_memory: Arc<std::sync::Mutex<crate::commands::context_memory::ContextMemory>>,
        hooks: Arc<std::sync::Mutex<crate::commands::hooks::HooksManager>>,
        graph: Arc<std::sync::Mutex<crate::commands::graph::GraphService>>,
    ) -> Result<StreamingAgentResponse> {
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");

        eprintln!("[Backend] Received workspace_path: {:?}", workspace_path);
        eprintln!("[Backend] Prior history turns: {}", prior_history.len());

        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut all_tool_calls = Vec::new();
        let total_tokens = 0u32;
        let mut status = "running".to_string();

        // Initialize infrastructure components
        let mut streaming_feedback = crate::commands::streaming_feedback::StreamingFeedback::new();
        let mut task_manager_instance: Option<crate::commands::task_manager::TaskFile> = None;
        // Emit start (phase events not batched - they're infrequent)
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "planning",
                "status": "started",
                "description": "Planning task"
            }));
        }

        // Initialize task tracking if workspace is available
        if let Some(ws) = &workspace_path {
            streaming_feedback.start_phase("planning");
            let project_name = std::path::Path::new(ws)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project")
                .to_string();
            task_manager_instance = Some(crate::commands::task_manager::TaskFile::new(
                project_name.clone(),
                task.clone(),
            ));
            eprintln!("[Agent] Initialized task tracking for workspace: {}", ws);

            // Record project context in context memory
            if let Ok(ctx_mem) = context_memory.lock() {
                ctx_mem.record_project_context(crate::commands::context_memory::ProjectContext {
                    workspace_path: ws.clone(),
                    project_type: "unknown".to_string(),
                    languages: vec![],
                    frameworks: vec![],
                    common_files: vec![],
                    last_analyzed: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }
        }

        // TIER 2: Trigger agent_start hooks
        if let Ok(hooks_mgr) = hooks.lock() {
            let triggered = hooks_mgr.trigger_event("agent_start");
            if !triggered.is_empty() {
                eprintln!("[Hooks] Triggered {} hook(s) on agent_start", triggered.len());
                for h in &triggered {
                    hooks_mgr.record_execution(h.id.clone(), "agent_start".to_string(), true, None);
                }
            }
        }

        // ── 1. RESEARCH & PLANNING PHASE (Context Gatherer) ────────────────
        // Real autonomous research using the Context Gatherer sub-agent.
        // This identifies relevant files and context BEFORE the main loop starts.
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "research",
                "status": "started",
                "description": "Gathering codebase context..."
            }));
        }

        streaming_feedback.start_phase("research");

        let research_findings = if let Some(ws) = &workspace_path {
            match self.run_research_phase(
                &task,
                ws,
                &active_file,
                model_name,
                vector_system.clone(),
                code_intel.clone(),
                app_state_ref.clone(),
            ).await {
                Ok(findings) => findings,
                Err(e) => {
                    eprintln!("[Research] Phase failed: {}. Proceeding without research.", e);
                    String::new()
                }
            }
        } else {
            String::new()
        };

        streaming_feedback.complete_phase("research");

        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:phase", &serde_json::json!({
                "phase": "research",
                "status": "completed",
                "description": "Research complete."
            }));
        }

        // TIER 2: Build dependency graph from code intelligence
        if let Some(ws) = &workspace_path {
            if let Ok(graph_svc) = graph.lock() {
                if let Ok(intel) = code_intel.lock() {
                    let symbols = intel.get_all_symbols(ws);
                    let nodes: Vec<crate::commands::graph::GraphNode> = symbols.iter().map(|s| {
                        crate::commands::graph::GraphNode {
                            id: s.name.clone(),
                            label: s.name.clone(),
                            node_type: s.symbol_type.clone(),
                            metadata: std::collections::HashMap::new(),
                        }
                    }).collect();
                    match graph_svc.build_dependency_graph(ws.clone(), nodes, vec![]) {
                        Ok(g) => {
                            let cycles = graph_svc.find_circular_dependencies(ws);
                            eprintln!("[Graph] Built graph: {} nodes, {} circular deps", g.nodes.len(), cycles.len());
                            if !cycles.is_empty() {
                                eprintln!("[Graph] ⚠️ Circular dependencies detected: {:?}", cycles.iter().map(|c| &c.cycle).collect::<Vec<_>>());
                            }
                        }
                        Err(e) => eprintln!("[Graph] Failed to build graph: {}", e),
                    }
                }
            }
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
        // Lean system prompt: only static rules + shell info + learned insights
        let system_prompt = self.get_system_prompt(&detected_shell, learning.clone());

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

        // ── WORKSPACE CONTEXT PRIMING (injected ONCE, not on every iteration) ─
        // This is the key optimisation: file tree, git diff, KIs, active file,
        // steering and code-intel are sent as a single user/assistant primer pair
        // instead of being re-embedded in the system prompt on every LLM call.
        let mut workspace_context = self.build_workspace_context(
            &workspace_path, &active_file, &task,
            code_intel.clone(), steering.clone(),
        );

        // Optimize context using ContextOptimizer if workspace is available
        if let Some(ws) = &workspace_path {
            streaming_feedback.start_phase("context_optimization");
            let mut optimizer = crate::commands::context_optimizer::ContextOptimizer::new(Some(8000));
            
            // Extract files from workspace context for optimization
            let files_for_optimization = vec![(ws.clone(), workspace_context.clone())];
            let pruned = optimizer.prune_context(files_for_optimization, &task, ws);
            
            eprintln!("[Agent] Context optimization: {} tokens (max: 8000)", pruned.estimated_tokens);
            eprintln!("[Agent] Pruned context summary: {}", pruned.summary);
            
            // Apply pruned context to workspace_context
            if pruned.estimated_tokens < workspace_context.len() as u32 / 4 {
                eprintln!("[Agent] Applying pruned context ({} files, {} tokens)", pruned.files.len(), pruned.estimated_tokens);
                // Use the summary as the workspace context since files were pruned
                workspace_context = pruned.summary;
            }
            
            streaming_feedback.complete_phase("context_optimization");
        }

        if !workspace_context.is_empty() {
            turn_messages.push(("user".to_string(), workspace_context));
            turn_messages.push((
                "assistant".to_string(),
                "Understood. I have reviewed the workspace context and am ready to assist.".to_string(),
            ));
        }

        // Current task message
        let final_task_msg = if !research_findings.is_empty() {
            format!("Task: {}\n\nPlease analyze the task, explore the codebase if needed, execute the necessary changes, and verify the results.\n\n<research_findings>\n{}\n</research_findings>", task.clone(), research_findings)
        } else {
            format!("Task: {}\nPlease analyze the task, explore the codebase if needed, execute the necessary changes, and verify the results.", task.clone())
        };

        // ── INTELLIGENT PROBLEM ANALYSIS ────────────────────────────────────
        // Analyze the problem to provide targeted investigation guidance
        let problem_analysis = Self::analyze_problem_intelligently(&task);
        let final_task_msg = format!("{}{}", final_task_msg, problem_analysis);

        // ── CONTEXT MEMORY: Inject prior patterns into task message ──────────
        // Query for relevant strategies and error patterns from past executions
        let context_memory_hint = if let Ok(ctx_mem) = context_memory.lock() {
            let stats = ctx_mem.get_statistics();
            let mut hint = String::new();

            // Inject best strategies for tool_execution tasks
            if stats.total_strategies > 0 {
                let best = ctx_mem.get_best_strategies("tool_execution");
                if !best.is_empty() {
                    hint.push_str("\n\n<prior_successful_strategies>\n");
                    for s in best.iter().take(3) {
                        hint.push_str(&format!(
                            "- {} (used {} times, effectiveness: {:.0}%)\n",
                            s.strategy, s.success_count, s.effectiveness_score * 100.0
                        ));
                    }
                    hint.push_str("</prior_successful_strategies>");
                }
            }

            // Inject known error patterns to help the LLM avoid them
            if stats.total_error_patterns > 0 {
                let errors = ctx_mem.get_all_error_patterns();
                let frequent: Vec<_> = {
                    let mut e = errors;
                    e.sort_by(|a, b| b.success_count.cmp(&a.success_count));
                    e.into_iter().take(3).collect()
                };
                if !frequent.is_empty() {
                    hint.push_str("\n\n<known_error_patterns>\n");
                    for e in &frequent {
                        hint.push_str(&format!(
                            "- {} (seen {} times, success rate: {:.0}%)\n",
                            e.error_type, e.success_count, e.success_rate * 100.0
                        ));
                    }
                    hint.push_str("</known_error_patterns>");
                }
            }

            hint
        } else {
            String::new()
        };

        let final_task_msg = format!("{}{}", final_task_msg, context_memory_hint);

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

        streaming_feedback.start_phase("execution");

        // ── Main execution loop ──────────────────────────────────────────────
        // Loop-detection state (mirrors Electron's repeatCount + ping-pong guard)
        let mut previous_tool_sig = String::new();
        let mut repeat_count = 0u32;
        let mut tool_sig_history: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(4);
        let mut validation_error_count = 0u32; // Track consecutive validation errors

        let iteration_extension = self.max_iterations * 3;
        let mut iteration_limit = self.max_iterations * 3;
        while iteration < iteration_limit {
            if crate::commands::agent::is_agent_cancelled() { break; }

            iteration += 1;
            eprintln!("[Agent] === Iteration {}/{} ===", iteration, self.max_iterations * 3);

            // ── Trim turn_messages growth (keep first 4 pinned + last 20 turns) ──
            // Each iteration adds 2 messages; without a cap the Vec grows to 4+(n*2).
            // We keep the 4 pinned anchor messages and the most recent 20 to bound memory.
            const MAX_HISTORY_TURNS: usize = 20;
            if turn_messages.len() > 4 + MAX_HISTORY_TURNS * 2 {
                let tail = turn_messages.split_off(4);
                let keep_from = tail.len().saturating_sub(MAX_HISTORY_TURNS * 2);
                turn_messages.extend_from_slice(&tail[keep_from..]);
            }

            // ── PER-ITERATION CONTEXT OPTIMIZATION ───────────────────────────
            // Every 5 iterations, check if the accumulated tool results are bloating
            // the context and prune the oldest non-pinned messages if over token budget.
            // Also trigger immediately if any single message is oversized.
            let total_context_chars: usize = turn_messages.iter().map(|(_, c)| c.len()).sum();
            let needs_trim = (iteration % 5 == 0 && total_context_chars / 4 > 6000)
                || turn_messages.iter().skip(4).any(|(_, c)| c.len() > 8000);
            if needs_trim && turn_messages.len() > 6 {
                let estimated_tokens = total_context_chars / 4;
                eprintln!("[ContextOptimizer] Iteration {}: context ~{} tokens — trimming to last 10 turns",
                    iteration, estimated_tokens);
                if turn_messages.len() > 4 + 10 * 2 {
                    let tail = turn_messages.split_off(4);
                    let keep_from = tail.len().saturating_sub(10 * 2);
                    turn_messages.extend_from_slice(&tail[keep_from..]);
                    eprintln!("[ContextOptimizer] Pruned to {} messages", turn_messages.len());
                }
            }

            // ── VALIDATION ERROR TRACKING ───────────────────────────────────
            // Track consecutive validation errors to detect stuck loops

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
                vector_system.clone(),
                code_intel.clone(),
            ).await?;

            eprintln!("[Agent] Phase 4 execution complete: {} tools executed", streaming_results.len());

            let mut tool_calls = Vec::new();
            let mut tool_results = Vec::new();
            let mut response = String::new();
            let mut done = false;
            let mut rejected_tools = Vec::new(); // Track rejected tool calls

            // Collect tool calls, results, and the raw LLM text (no extra Ollama call needed)
            let mut raw_llm_text = String::new();
            for (tool_call_or_text, result) in streaming_results {
                // The last entry with tool=="__response__" carries the full response text
                if tool_call_or_text.tool == "__response__" {
                    raw_llm_text = result.unwrap_or_default();
                    continue;
                }
                
                // Extract rejected tools from the special marker entry
                if tool_call_or_text.tool == "__rejected_tools__" {
                    if let Ok(Ok(data)) = serde_json::to_value(&tool_call_or_text.args).map(|v| v.get("tools").cloned().ok_or(())) {
                        if let Ok(tools_array) = serde_json::from_value::<Vec<String>>(data) {
                            rejected_tools.extend(tools_array);
                        }
                    }
                    continue;
                }
                
                tool_calls.push(tool_call_or_text.clone());
                match &result {
                    Ok(r) => {
                        tool_results.push(format!("[{}] result:\n{}", tool_call_or_text.tool, r));
                        eprintln!("[Agent] Tool {} succeeded", tool_call_or_text.tool);
                        
                        // Update task status if task manager is available
                        if let Some(ref mut task_file) = task_manager_instance {
                            let task_id = format!("tool_{}", tool_call_or_text.tool);
                            task_file.update_task_status(
                                &task_id,
                                crate::commands::task_manager::TaskStatus::Completed,
                                Some(r.clone()),
                            );
                        }

                        // TIER 1.1: Record successful tool execution in learning system
                        if let Ok(learning_sys) = learning.lock() {
                            let record = crate::commands::learning::InteractionRecord {
                                user_request: format!("Tool: {}", tool_call_or_text.tool),
                                agent_response: r.clone(),
                                tools_used: vec![tool_call_or_text.tool.clone()],
                                success: true,
                                duration_ms: 0,
                                timestamp: chrono::Utc::now().timestamp(),
                            };
                            learning_sys.record_interaction(record);
                        }

                        // TIER 1.2: Record successful strategy in context memory
                        if let Ok(context_mem) = context_memory.lock() {
                            context_mem.record_successful_strategy(
                                "tool_execution".to_string(),
                                tool_call_or_text.tool.clone(),
                                vec![tool_call_or_text.tool.clone()],
                                0.0,
                            );
                        }

                        // TIER 2: Trigger tool_success hooks
                        if let Ok(hooks_mgr) = hooks.lock() {
                            let triggered = hooks_mgr.trigger_tool_event("tool_success", &tool_call_or_text.tool);
                            for h in &triggered {
                                hooks_mgr.record_execution(h.id.clone(), "tool_success".to_string(), true, None);
                            }
                        }
                    }
                    Err(e) => {
                        tool_results.push(format!("[{}] error:\n{}", tool_call_or_text.tool, e));
                        eprintln!("[Agent] Tool {} failed: {}", tool_call_or_text.tool, e);
                        
                        // Update task status if task manager is available
                        if let Some(ref mut task_file) = task_manager_instance {
                            let task_id = format!("tool_{}", tool_call_or_text.tool);
                            task_file.update_task_status(
                                &task_id,
                                crate::commands::task_manager::TaskStatus::Failed,
                                Some(e.to_string()),
                            );
                        }

                        // TIER 1.1: Record failed tool execution in learning system
                        if let Ok(learning_sys) = learning.lock() {
                            let record = crate::commands::learning::InteractionRecord {
                                user_request: format!("Tool: {}", tool_call_or_text.tool),
                                agent_response: format!("Error: {}", e),
                                tools_used: vec![tool_call_or_text.tool.clone()],
                                success: false,
                                duration_ms: 0,
                                timestamp: chrono::Utc::now().timestamp(),
                            };
                            learning_sys.record_interaction(record);
                        }

                        // TIER 1.2: Record error pattern in context memory
                        if let Ok(context_mem) = context_memory.lock() {
                            context_mem.record_error_pattern(
                                e.to_string(),
                                format!("Tool: {}", tool_call_or_text.tool),
                                "".to_string(),
                                false,
                                0.0,
                            );
                        }

                        // TIER 2: Trigger tool_failure hooks
                        if let Ok(hooks_mgr) = hooks.lock() {
                            let triggered = hooks_mgr.trigger_tool_event("tool_failure", &tool_call_or_text.tool);
                            for h in &triggered {
                                hooks_mgr.record_execution(h.id.clone(), "tool_failure".to_string(), false, Some(e.to_string()));
                            }
                        }
                    }
                }
                all_tool_calls.push(tool_call_or_text.clone());
                if tool_call_or_text.tool == "done" { done = true; }
            }

            // ── STALL DETECTION (No extra Ollama calls — reuse already-captured text) ──
            // Mirrors Electron's heuristic: check for thinking-only, instructional prose,
            // malformed JSON intent, or natural completion. Only make a correction call
            // when the model clearly tried to output tool JSON but mangled the syntax.
            if tool_calls.is_empty() && !done {
                eprintln!("[Agent] No tools in stream — analysing response text...");
                eprintln!("[Agent] Response length: {} chars", raw_llm_text.len());
                eprintln!("[Agent] Contains \"thought\": {}", raw_llm_text.contains("\"thought\""));
                eprintln!("[Agent] Contains \"tool\": {}", raw_llm_text.contains("\"tool\""));
                
                // Check if tools were rejected due to validation
                if !rejected_tools.is_empty() {
                    eprintln!("[Agent] ⚠️ {} tool calls were rejected due to missing arguments", rejected_tools.len());
                    eprintln!("[Agent] === RAW LLM RESPONSE (first 2000 chars) ===");
                    eprintln!("{}", &raw_llm_text[..raw_llm_text.len().min(2000)]);
                    eprintln!("[Agent] === END RAW LLM RESPONSE ===");
                    validation_error_count += 1;
                    
                    // Build a detailed error message with specific guidance
                    let mut error_msg = "[SYSTEM] VALIDATION ERROR: Tool calls rejected due to missing required arguments:\n\n".to_string();
                    for rejected in &rejected_tools {
                        error_msg.push_str(&format!("❌ {}\n", rejected));
                    }
                    error_msg.push_str("\n📋 REQUIRED ARGUMENTS BY TOOL:\n");
                    error_msg.push_str("• read_file, write_file, edit_file, multi_edit_file, create_file, delete_file, move_file, rename_file: MUST have \"path\"\n");
                    error_msg.push_str("• run_command: MUST have \"command\"\n");
                    error_msg.push_str("• grep_search, search_files: MUST have \"query\"\n");
                    error_msg.push_str("\n✅ EXAMPLE OF CORRECT FORMAT:\n");
                    error_msg.push_str("{\"thought\": \"Reading the file to understand the issue\", \"tool\": \"read_file\", \"args\": {\"path\": \"src/components/Chat/ChatPanel.tsx\"}}\n");
                    
                    // Add aggressive nudge if this is the 3rd consecutive validation error
                    if validation_error_count >= 3 {
                        error_msg.push_str("\n🚨 CRITICAL: This is validation error #");
                        error_msg.push_str(&validation_error_count.to_string());
                        error_msg.push_str(". You MUST provide ALL required arguments. Your next response MUST be valid JSON with complete arguments or the task will fail.");
                    } else {
                        error_msg.push_str("\n🔄 RETRY: Output your tool call with ALL required arguments filled in. Do not skip any required fields.");
                    }
                    
                    tool_results.insert(0, error_msg);
                    // Don't do stall detection - just inform the LLM to retry
                    // Continue to add results to turn_messages below
                } else if raw_llm_text.contains("\"tool\"") && raw_llm_text.len() > 100 {
                    // LLM tried to output tool calls but they were all rejected
                    eprintln!("[Agent] ⚠️ LLM output contains tool calls but they were rejected due to validation");
                    
                    // Check if this is incomplete JSON (likely truncated response)
                    let open_braces = raw_llm_text.matches('{').count();
                    let close_braces = raw_llm_text.matches('}').count();
                    let is_incomplete_json = open_braces > close_braces;
                    
                    if is_incomplete_json {
                        eprintln!("[Agent] ⚠️ INCOMPLETE JSON DETECTED: {} open braces, {} close braces", open_braces, close_braces);
                        eprintln!("[Agent] This likely means the LLM response was truncated mid-JSON");
                        eprintln!("[Agent] === RAW LLM RESPONSE (first 2000 chars) ===");
                        eprintln!("{}", &raw_llm_text[..raw_llm_text.len().min(2000)]);
                        eprintln!("[Agent] === END RAW LLM RESPONSE ===");
                        
                        // Write to debug file for inspection
                        let debug_file = format!("incomplete_json_iter_{}.txt", iteration);
                        let _ = std::fs::write(&debug_file, format!("Iteration: {}\nIncomplete JSON - Open: {}, Close: {}\n\n=== RAW LLM RESPONSE ===\n{}", iteration, open_braces, close_braces, raw_llm_text));
                        eprintln!("[Agent] Debug output written to: {}", debug_file);
                        
                        // Add a message to help the LLM understand the issue
                        tool_results.insert(0, "[SYSTEM] ERROR: Your response was incomplete or truncated. The JSON object was not fully formed (missing closing braces). Please retry with a complete JSON response: {\"thought\": \"...\", \"tool\": \"...\", \"args\": {...}}".to_string());
                    } else {
                        eprintln!("[Agent] === RAW LLM RESPONSE (first 2000 chars) ===");
                        eprintln!("{}", &raw_llm_text[..raw_llm_text.len().min(2000)]);
                        eprintln!("[Agent] === END RAW LLM RESPONSE ===");
                        
                        // Write to debug file for inspection
                        let debug_file = format!("validation_error_iter_{}.txt", iteration);
                        let _ = std::fs::write(&debug_file, format!("Iteration: {}\nValidation Error Count: {}\n\n=== RAW LLM RESPONSE ===\n{}\n\n=== REJECTED TOOLS ===\n{:?}", iteration, validation_error_count, raw_llm_text, rejected_tools));
                        eprintln!("[Agent] Debug output written to: {}", debug_file);
                    }
                    
                    validation_error_count += 1;
                    
                    let mut error_msg = "[SYSTEM] VALIDATION ERROR: Your tool calls were rejected because they are missing required arguments.\n\n📋 REQUIRED ARGUMENTS:\n• read_file, write_file, edit_file, multi_edit_file, create_file, delete_file, move_file, rename_file: MUST have \"path\"\n• run_command: MUST have \"command\"\n• grep_search, search_files: MUST have \"query\"\n\n✅ EXAMPLE:\n{\"thought\": \"Your reasoning\", \"tool\": \"read_file\", \"args\": {\"path\": \"file.tsx\"}}\n\n".to_string();
                    
                    // Add aggressive nudge if this is the 3rd consecutive validation error
                    if validation_error_count >= 3 {
                        error_msg.push_str("🚨 CRITICAL: This is validation error #");
                        error_msg.push_str(&validation_error_count.to_string());
                        error_msg.push_str(". You MUST provide ALL required arguments. Your next response MUST be valid JSON with complete arguments or the task will fail.");
                    } else {
                        error_msg.push_str("🔄 RETRY with complete arguments.");
                    }
                    
                    tool_results.insert(0, error_msg);
                    // Continue to add results to turn_messages below
                } else {
                    // No validation error this iteration
                    validation_error_count = 0;
                    let looks_complete = raw_llm_text.to_uppercase().contains("TASK COMPLETE")
                    || raw_llm_text.to_uppercase().contains("TASK DONE")
                    || (raw_llm_text.len() < 400 && iteration > 1);

                let instructional = raw_llm_text.to_lowercase().contains("you should run")
                    || raw_llm_text.to_lowercase().contains("please run")
                    || raw_llm_text.to_lowercase().contains("manually")
                    || (raw_llm_text.contains("```") && !raw_llm_text.contains("\'tool\'"));

                // Check for large JSON tool call in prose (common pattern for code generation)
                let has_large_json_tool = raw_llm_text.contains("\"tool\"") && raw_llm_text.len() > 2000;

                eprintln!("[Agent] looks_complete: {}", looks_complete);
                eprintln!("[Agent] has_large_json_tool: {}", has_large_json_tool);
                eprintln!("[Agent] raw_llm_text.len() > 3000: {}", raw_llm_text.len() > 3000);
                eprintln!("[Agent] raw_llm_text.len() > 1000 && contains \"thought\": {}", raw_llm_text.len() > 1000 && raw_llm_text.contains("\"thought\""));

                if raw_llm_text.contains("{") && (raw_llm_text.contains("\"tool\"") || raw_llm_text.contains("'tool'")) && !looks_complete {
                    // Model tried to call a tool but syntax was broken — single correction
                    // Use only the last 6 messages to avoid sending huge context to the correction call
                    eprintln!("[Agent] Malformed JSON intent detected — sending one correction nudge");
                    let pinned = &turn_messages[..4.min(turn_messages.len())];
                    let recent_start = if turn_messages.len() > 4 { turn_messages.len().saturating_sub(4) } else { 4 };
                    let recent = &turn_messages[recent_start..];
                    let mut correction_msgs: Vec<(String, String)> = pinned.to_vec();
                    correction_msgs.extend_from_slice(recent);
                    correction_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                    correction_msgs.push(("user".to_string(),
                        "CRITICAL: Your last response had a JSON syntax error or was incomplete. \
                         Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}. No markdown, no backticks.".to_string()
                    ));
                    self.suppress_stream = true;
                    let (fix, _) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    tool_calls = extract_tool_calls(&fix);
                    // Update raw_llm_text so the conversation history reflects the fix
                    raw_llm_text.push_str("\n[Correction]: ");
                    raw_llm_text.push_str(&fix);
                    if tool_calls.is_empty() { 
                        response = raw_llm_text.clone();
                        // Retry correction with a stronger nudge
                        eprintln!("[Agent] Correction failed — sending stronger nudge");
                        let mut retry_msgs: Vec<(String, String)> = pinned.to_vec();
                        retry_msgs.extend_from_slice(recent);
                        retry_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                        retry_msgs.push(("user".to_string(),
                            "FINAL WARNING: You MUST use tools. Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}. NO OTHER FORMAT.".to_string()
                        ));
                        self.suppress_stream = true;
                        let (retry_fix, _) = self.call_llm_streaming(&retry_msgs, model_name).await?;
                        self.suppress_stream = false;
                        tool_calls = extract_tool_calls(&retry_fix);
                        raw_llm_text.push_str("\n[Retry Correction]: ");
                        raw_llm_text.push_str(&retry_fix);
                    }
                } else if has_large_json_tool {
                    // LLM output a large JSON tool call in prose instead of using write_file/create_file
                    eprintln!("[Agent] Large JSON tool in prose detected ({} chars) — sending correction nudge", raw_llm_text.len());
                    let pinned = &turn_messages[..4.min(turn_messages.len())];
                    let recent_start = if turn_messages.len() > 4 { turn_messages.len().saturating_sub(4) } else { 4 };
                    let recent = &turn_messages[recent_start..];
                    let mut correction_msgs: Vec<(String, String)> = pinned.to_vec();
                    correction_msgs.extend_from_slice(recent);
                    correction_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                    correction_msgs.push(("user".to_string(),
                        "CRITICAL: You wrote a tool call in your response instead of using the appropriate tool. \
                         Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}. No markdown, no backticks.".to_string()
                    ));
                    self.suppress_stream = true;
                    let (fix, _) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    tool_calls = extract_tool_calls(&fix);
                    raw_llm_text.push_str("\n[Correction]: ");
                    raw_llm_text.push_str(&fix);
                    if tool_calls.is_empty() { response = raw_llm_text.clone(); }
                } else if raw_llm_text.len() > 3000 && !looks_complete {
                    // LLM is producing prose instead of using tools — force correction
                    eprintln!("[Agent] Large prose response ({} chars) detected — forcing tool usage", raw_llm_text.len());
                    let pinned = &turn_messages[..4.min(turn_messages.len())];
                    let recent_start = if turn_messages.len() > 4 { turn_messages.len().saturating_sub(4) } else { 4 };
                    let recent = &turn_messages[recent_start..];
                    let mut correction_msgs: Vec<(String, String)> = pinned.to_vec();
                    correction_msgs.extend_from_slice(recent);
                    correction_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                    correction_msgs.push(("user".to_string(),
                        "CRITICAL: You produced {} characters of prose instead of using tools. \
                         You MUST use write_file/create_file to create files. Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}.".to_string()
                    ));
                    self.suppress_stream = true;
                    let (fix, _) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    tool_calls = extract_tool_calls(&fix);
                    raw_llm_text.push_str("\n[Correction]: ");
                    raw_llm_text.push_str(&fix);
                    if tool_calls.is_empty() { response = raw_llm_text.clone(); }
                } else if raw_llm_text.len() > 1000 && raw_llm_text.contains("\"thought\"") && !looks_complete {
                    // LLM has reasoning (JSON thought key) but no tools — force correction
                    eprintln!("[Agent] Reasoning without tools ({} chars) detected — forcing tool usage", raw_llm_text.len());
                    let pinned = &turn_messages[..4.min(turn_messages.len())];
                    let recent_start = if turn_messages.len() > 4 { turn_messages.len().saturating_sub(4) } else { 4 };
                    let recent = &turn_messages[recent_start..];
                    let mut correction_msgs: Vec<(String, String)> = pinned.to_vec();
                    correction_msgs.extend_from_slice(recent);
                    correction_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                    correction_msgs.push(("user".to_string(),
                        "CRITICAL: You provided reasoning but no tool calls. You MUST use tools to act. \
                         Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}.".to_string()
                    ));
                    self.suppress_stream = true;
                    let (fix, _) = self.call_llm_streaming(&correction_msgs, model_name).await?;
                    self.suppress_stream = false;
                    tool_calls = extract_tool_calls(&fix);
                    raw_llm_text.push_str("\n[Correction]: ");
                    raw_llm_text.push_str(&fix);
                    if tool_calls.is_empty() { 
                        response = raw_llm_text.clone();
                        // Retry with stronger nudge
                        eprintln!("[Agent] Correction failed — sending stronger nudge");
                        let mut retry_msgs: Vec<(String, String)> = pinned.to_vec();
                        retry_msgs.extend_from_slice(recent);
                        retry_msgs.push(("assistant".to_string(), raw_llm_text.clone()));
                        retry_msgs.push(("user".to_string(),
                            "FINAL WARNING: You MUST use tools. Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}}. NO OTHER FORMAT.".to_string()
                        ));
                        self.suppress_stream = true;
                        let (retry_fix, _) = self.call_llm_streaming(&retry_msgs, model_name).await?;
                        self.suppress_stream = false;
                        tool_calls = extract_tool_calls(&retry_fix);
                        raw_llm_text.push_str("\n[Retry Correction]: ");
                        raw_llm_text.push_str(&retry_fix);
                    }
                } else if instructional && !looks_complete {
                    // Model explaining instead of acting — nudge without an LLM call
                    eprintln!("[Agent] Model is explaining instead of acting — nudging");
                    turn_messages.push(("assistant".to_string(), raw_llm_text.clone()));
                    turn_messages.push(("user".to_string(),
                        "STRICT ACTION REQUIRED: Use your tools IMMEDIATELY. \
                         Output ONLY valid JSON: {\"thought\": \"brief reason\", \"tool\": \"tool_name\", \"args\": {...}} NOW.".to_string()
                    ));
                    continue; // re-enter loop — no extra Ollama call
                } else {
                    // Natural language completion or pure thinking — treat as final response
                    eprintln!("[Agent] Treating as final response");
                    response = raw_llm_text.clone();
                }
                } // Close the else block for rejected_tools check
            }

            // ── LOOP DETECTION (mirrors Electron's repeatCount + ping-pong guard) ──
            if !tool_calls.is_empty() {
                let sig: String = tool_calls.iter()
                    .map(|tc| format!("{}:{}", tc.tool,
                        tc.args.get("path").or(tc.args.get("command"))
                            .and_then(|v| v.as_str()).unwrap_or("")))
                    .collect::<Vec<_>>().join(",");

                if sig == previous_tool_sig {
                    repeat_count += 1;
                    if repeat_count >= 3 {
                        eprintln!("[Agent] ⚠️ Stuck in repetitive loop — prompting collaboration");
                        tool_results.push("[SYSTEM] LOOP DETECTED: You have attempted the same action three times. You are likely stuck. Please use the 'ask_user' tool to explain what you are looking for and ask the user for guidance or to clarify the file's content/error.".to_string());
                        repeat_count = 0;
                    } else {
                        tool_results.push("[SYSTEM] REPETITION WARNING: You repeated the exact same tool call. Analyze why it didn't give you the info you needed and change your parameters or try a different tool.".to_string());
                    }
                } else {
                    repeat_count = 0;
                    previous_tool_sig = sig.clone();
                }

                // Ping-pong: A→B→A→B pattern
                tool_sig_history.push_back(sig);
                if tool_sig_history.len() > 4 { tool_sig_history.pop_front(); }
                if tool_sig_history.len() == 4
                    && tool_sig_history[0] == tool_sig_history[2]
                    && tool_sig_history[1] == tool_sig_history[3]
                {
                    eprintln!("[Agent] ⚠️ Ping-pong loop detected — injecting strategy change nudge");
                    tool_results.push("[SYSTEM] Alternating loop detected. You must try a completely different approach.".to_string());
                    tool_sig_history.clear();
                }
            }

            if tool_calls.is_empty() && response.is_empty() && tool_results.is_empty() {
                steps.push(AgentStep {
                    iteration,
                    tool: "reasoning".to_string(),
                    status: "failed".to_string(),
                    summary: "Agent stalled".to_string(),
                    result: Some("Model produced no tool calls and no final text response. Task aborted.".to_string()),
                    logs: None,
                    persona: Some("agent".to_string()),
                    request_id: None,
                    data: None,
                });
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("agent:phase", &serde_json::json!({
                        "phase": "execution",
                        "status": "failed",
                        "description": "Agent stalled without finishing."
                    }));
                }
                break;
            }

            // ── PHASE 5: Update conversation history (ONE message per turn) ──────────────
            // For local models, 2000 chars is ~500 tokens — keeps context clean
            const MAX_TOOL_RESULT_CHARS: usize = 2_000;
            // Cap assistant responses stored in history — prevents a single massive
            // prose dump (e.g. LLM writing code inline instead of using write_file)
            // from bloating context and hanging subsequent iterations.
            const MAX_ASSISTANT_HISTORY_CHARS: usize = 3_000;

            if !tool_calls.is_empty() {
                // ── (a) Store ASSISTANT response — truncated if oversized ──────
                let stored_assistant = if raw_llm_text.len() > MAX_ASSISTANT_HISTORY_CHARS {
                    eprintln!("[Agent] ⚠️ Truncating large assistant response ({} chars) before storing in history", raw_llm_text.len());
                    format!("{}\n... [response truncated — {} chars total]", &raw_llm_text[..MAX_ASSISTANT_HISTORY_CHARS], raw_llm_text.len())
                } else {
                    raw_llm_text.clone()
                };
                turn_messages.push(("assistant".to_string(), stored_assistant));

                // Detect: LLM wrote code in prose instead of using write_file tool
                let has_large_code_block = raw_llm_text.contains("```") && raw_llm_text.len() > 2000;
                // Also detect: LLM output a large JSON tool call in prose (common pattern for code generation)
                let has_large_json_tool = raw_llm_text.contains("\"tool\"") && raw_llm_text.len() > 2000;
                
                if (has_large_code_block || has_large_json_tool) && tool_calls.iter().all(|tc| tc.tool != "write_file" && tc.tool != "edit_file" && tc.tool != "multi_edit_file" && tc.tool != "create_file") {
                    // Automatically parse and extract tool calls from prose instead of just nudging
                    let prose_tool_calls = extract_tool_calls_from_prose(&raw_llm_text);
                    
                    if !prose_tool_calls.is_empty() {
                        eprintln!("[Agent] ✅ CODE IN PROSE DETECTED - automatically extracting {} tool calls", prose_tool_calls.len());
                        
                        // Replace the prose tool calls with the extracted ones
                        // Keep existing tool_calls that aren't prose-based
                        for call in prose_tool_calls {
                            if !tool_calls.iter().any(|tc| tc.tool == call.tool && tc.args == call.args) {
                                tool_calls.push(call.clone());
                                all_tool_calls.push(call);
                            }
                        }
                        
                        // Don't add error message - we handled it automatically
                    } else {
                        // No tool calls found in prose, add error message
                        tool_results.insert(0, "[SYSTEM] CODE IN PROSE DETECTED: You wrote code or a tool call in your response instead of using the appropriate tool. You MUST use write_file/create_file to create files. Do NOT include file contents or tool calls in your reasoning — call the tool directly.".to_string());
                        eprintln!("[Agent] ⚠️ LLM wrote code/tool in prose — no extractable tool calls found");
                    }
                }

                // Note: JSON format validation is handled by extract_tool_calls() which checks for "tool" field
                // No need to enforce <thought> tags since we're using JSON-only format

                // ── (b) Single USER message: results + guidance ─────────────────
                if !tool_results.is_empty() {
                    // Truncate each result individually
                    let truncated: Vec<String> = tool_results.iter().map(|r| {
                        if r.len() > MAX_TOOL_RESULT_CHARS {
                            format!("{}\n... (truncated for brevity, {} chars total. Use read_file with lines for more context if needed.)", &r[..MAX_TOOL_RESULT_CHARS], r.len())
                        } else {
                            r.clone()
                        }
                    }).collect();

                    let mut results_msg = truncated.join("\n\n");

                    // Append failure guidance inline
                    let failed_count = tool_results.iter()
                        .filter(|r| r.contains("]") && (r.contains("] error:") || r.contains("failed"))).count();
                    
                    if failed_count > 0 {
                        results_msg.push_str(&format!(
                            "\n\n⚠️ {} tool(s) failed. Analyze the error above, rethink your approach, and try a more specific command.",
                            failed_count
                        ));
                    }
                    
                    // ── SYNTAX-AWARE NUDGE (Breaks "blind reading" panic loops) ──
                    if results_msg.contains("PARSE_ERROR") || results_msg.contains("SyntaxError") || results_msg.contains("Expected `}`") {
                        results_msg.push_str("\n\n[SYSTEM HINT: A syntax or parse error is present. You MUST use the 'view_structure' tool on the affected file to see its skeleton. This will instantly reveal missing braces or keywords. Do NOT use run_command or delete the file.]");
                    }
                    
                    if let Some(ws) = &workspace_path {
                        results_msg.push_str(&format!("\n[Workspace: {}]", ws));
                    }
                    
                    // ── PROTOCOL REMINDER (Essential for local model consistency) ──
                    results_msg.push_str("\n\nPROMPT: Analyze the results above. Output ONLY valid JSON: {\"thought\": \"your reasoning\", \"tool\": \"tool_name\", \"args\": {...}}");

                    turn_messages.push(("user".to_string(), results_msg));
                }
            } else if !response.is_empty() {
                turn_messages.push(("assistant".to_string(), response));
            }

            if done { break; }

            if iteration >= iteration_limit {
                match self.request_iteration_continuation(iteration).await {
                    Ok(true) => {
                        iteration_limit += iteration_extension;
                        let continuation_step = AgentStep {
                            iteration,
                            tool: "reasoning".to_string(),
                            status: "alternative".to_string(),
                            summary: format!("User approved continuation. Extending limit to {} iterations.", iteration_limit),
                            result: None,
                            logs: Some(vec![format!("Continuing task after reaching {} iterations.", iteration)]),
                            persona: Some("agent".to_string()),
                            request_id: Some(format!("iteration_limit_continue_{}", iteration)),
                            data: Some(serde_json::json!({
                                "reason": "iteration_limit_extended",
                                "new_limit": iteration_limit,
                            })),
                        };
                        self.emit_step(continuation_step).await;
                    }
                    Ok(false) => {
                        status = "max_iterations_reached".to_string();
                        break;
                    }
                    Err(error) => {
                        status = "max_iterations_reached".to_string();
                        steps.push(AgentStep {
                            iteration,
                            tool: "reasoning".to_string(),
                            status: "failed".to_string(),
                            summary: "Failed to prompt for continuation".to_string(),
                            result: Some(error.to_string()),
                            logs: None,
                            persona: Some("agent".to_string()),
                            request_id: Some(format!("iteration_limit_failed_{}", iteration)),
                            data: None,
                        });
                        break;
                    }
                }
            }
        }

        if iteration >= iteration_limit && status != "completed" {
            eprintln!("[Agent] Global max iterations reached ({})", iteration);
            status = "max_iterations_reached".to_string();
        }

        // Complete execution phase
        streaming_feedback.complete_phase("execution");
        
        // TIER 3: Emit enriched metrics including context memory and hook stats
        let metrics = streaming_feedback.get_metrics();
        let context_mem_stats = context_memory.lock().ok()
            .map(|m| m.get_statistics())
            .map(|s| serde_json::json!({
                "total_patterns": s.total_patterns,
                "total_error_patterns": s.total_error_patterns,
                "total_strategies": s.total_strategies
            }))
            .unwrap_or(serde_json::json!({}));
        let hook_metrics = hooks.lock().ok()
            .map(|h| h.get_metrics())
            .map(|m| serde_json::json!({
                "total_executions": m.total_executions,
                "successful_executions": m.successful_executions,
                "failed_executions": m.failed_executions
            }))
            .unwrap_or(serde_json::json!({}));
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:metrics", &serde_json::json!({
                "total_time_ms": metrics.total_time_ms,
                "phases_completed": metrics.phases_completed,
                "status": status,
                "context_memory": context_mem_stats,
                "hooks": hook_metrics
            }));
        }

        // Save task state if task manager is available
        if let Some(task_file) = task_manager_instance {
            if let Some(ws) = &workspace_path {
                match crate::commands::task_manager::TaskManager::save_tasks_file(ws, &task_file) {
                    Ok(_) => eprintln!("[Agent] Task state saved successfully"),
                    Err(e) => eprintln!("[Agent] Failed to save task state: {}", e),
                }
            }
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
                agent_response: "Task execution complete".to_string(),
                tools_used,
                success: status != "max_iterations_reached",
                duration_ms: (std::time::Instant::now().elapsed().as_millis() as u32),
                timestamp: chrono::Utc::now().timestamp(),
            };
            l.record_interaction(record);
            // TIER 3: Analyze patterns after each task so insights are ready for next prompt
            let insights = l.analyze_patterns();
            eprintln!("[Learning] Analyzed patterns: {} insights generated", insights.len());
        }

        // TIER 2: Trigger agent_complete hooks
        if let Ok(hooks_mgr) = hooks.lock() {
            let triggered = hooks_mgr.trigger_event("agent_complete");
            if !triggered.is_empty() {
                eprintln!("[Hooks] Triggered {} hook(s) on agent_complete", triggered.len());
                for h in &triggered {
                    hooks_mgr.record_execution(h.id.clone(), "agent_complete".to_string(), true, None);
                }
            }
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

    async fn run_research_phase(
        &mut self,
        task: &str,
        workspace_path: &str,
        active_file: &Option<serde_json::Value>,
        model_name: &str,
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        app_state_ref: Arc<RwLock<AppState>>,
    ) -> Result<String> {
        let config = crate::commands::prompts::get_sub_agent_config("context-gatherer")
            .ok_or_else(|| "Context Gatherer config not found".to_string())?;
        let grounding_context = {
            let mut summary = String::new();
            if let Ok(intel) = code_intel.lock() {
                if let Ok(context) = intel.analyze_workspace_if_stale(workspace_path.to_string()) {
                    let top_symbols = context.symbols.iter().take(8)
                        .map(|symbol| format!("{} ({})", symbol.name, symbol.symbol_type))
                        .collect::<Vec<_>>();
                    summary.push_str(&format!(
                        "<workspace_grounding>\nFiles analyzed: {}\nSymbols: {}\nPatterns: {}\nTop symbols: {}\n</workspace_grounding>\n",
                        context.metrics.total_files,
                        context.metrics.total_symbols,
                        context.patterns.len(),
                        if top_symbols.is_empty() { "none".to_string() } else { top_symbols.join(", ") },
                    ));
                }
            }
            if let Ok(system) = vector_system.lock() {
                if let Ok(stats) = system.get_index_stats() {
                    summary.push_str(&format!(
                        "<semantic_index>\nChunks: {}\nFiles indexed: {}\nLast updated: {}\n</semantic_index>\n",
                        stats.total_chunks,
                        stats.total_files,
                        stats.last_updated,
                    ));
                }
            }
            summary
        };
        let issue_focus = self.build_issue_focus_context(
            &Some(workspace_path.to_string()),
            active_file,
            task,
            code_intel.clone(),
        );

        let mut sub_agent_msgs = vec![
            ("system".to_string(), config.system_prompt),
            ("user".to_string(), format!(
                "CODEBASE RESEARCH TASK:\n{}\n\nWorkspace: {}\nActive File: {:?}\n\n{}{}\n<research_rules>\n- Prefer local repository sources first: semantic_search, find_symbols, search_files, read_file.\n- Do not start by reading whole files when cached workspace context, semantic search, or symbol search can narrow the scope.\n- When an error mentions a file or line, inspect that file and a narrow line window first.\n- Expand to dependent files only after you identify the local cause.\n- Use external web research only when local evidence is insufficient.\n- When using external sources, cite URLs and clearly separate external findings from local codebase findings.\n- Call out uncertainty instead of presenting guesses as facts.\n</research_rules>",
                task,
                workspace_path,
                active_file,
                grounding_context,
                issue_focus
            )),
        ];

        self.run_sub_agent_loop(
            &mut sub_agent_msgs,
            model_name,
            config.max_iterations.unwrap_or(5),
            workspace_path,
            vector_system,
            code_intel,
            app_state_ref,
        ).await
    }

    async fn run_sub_agent_loop(
        &mut self,
        messages: &mut Vec<(String, String)>,
        model_name: &str,
        max_iters: u32,
        workspace_path: &str,
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        _app_state_ref: Arc<RwLock<AppState>>,
    ) -> Result<String> {
        let mut iteration = 0;
        let mut final_summary = String::new();

        while iteration < max_iters {
            iteration += 1;
            eprintln!("[Sub-Agent] Iteration {}/{}", iteration, max_iters);

            let (response_text, _) = self.call_llm_streaming(messages, model_name).await?;
            messages.push(("assistant".to_string(), response_text.clone()));

            let tool_calls = extract_tool_calls(&response_text);
            if tool_calls.is_empty() {
                if iteration == 1 {
                    eprintln!("[Sub-Agent] ⚠️ Agent tried to exit on iteration 1. Forcing tool usage.");
                    messages.push(("user".to_string(), "[SYSTEM] You did not use any tools. You are a Research Unit. You MUST use tools (like search_files, read_file, or semantic_search) to gather context before returning a summary. Try again.".to_string()));
                    continue;
                }
                final_summary = response_text.clone();
                break;
            }

            let mut results = Vec::new();
            for tool_call in tool_calls.iter() {
                eprintln!("[Sub-Agent] Executing tool: {}", tool_call.tool);
                // Sub-agents always use standalone execution (no complex recovery)
                let result = execute_tool_standalone(
                    tool_call,
                    &Some(workspace_path.to_string()),
                    &vector_system,
                    &code_intel,
                    self.app_handle.as_ref(),
                ).await;
                
                match result {
                    Ok(r) => results.push(format!("[{}] result:\n{}", tool_call.tool, r)),
                    Err(e) => results.push(format!("[{}] error:\n{}", tool_call.tool, e)),
                }
                
                if tool_call.tool == "done" {
                    iteration = max_iters; // Break outer loop
                }
            }
            
            let result_msg = results.join("\n\n");
            messages.push(("user".to_string(), format!("{}\n\nContinue analyzing or provide your final summary.", result_msg)));
        }

        if final_summary.is_empty() {
            final_summary = messages.last().map(|(_, c)| c.clone()).unwrap_or_default();
        }

        Ok(final_summary)
    }

    async fn call_llm_streaming(&self, messages: &[(String, String)], model: &str) -> Result<(String, u32)> {
        let mut messages_json = Vec::new();
        // Context sliding window: always keep system prompt (idx 0) + workspace primer (idx 1, 2)
        // and fill the rest of the budget with the newest messages first.
        let mut char_count = 0;
        let mut iter_messages = messages.iter().enumerate().collect::<Vec<_>>();
        iter_messages.reverse(); // traverse newest → oldest

        // Always pin the first 4 messages:
        //   0 = system prompt
        //   1 = workspace context (user)
        //   2 = workspace context ack (assistant)
        //   3 = user task message  ← CRITICAL: never drop the original task
        let mut included_indices = std::collections::HashSet::new();
        included_indices.insert(0);
        included_indices.insert(1);
        included_indices.insert(2);
        included_indices.insert(3);

        for (i, (_role, content)) in iter_messages {
            if i <= 3 || included_indices.contains(&i) { continue; }
            // Budget: 4 chars ≈ 1 token; reserve tokens for the LLM's response
            // Aim to keep the prompt size within context limit
            let limit = (self.context_length as usize * 4).saturating_sub(4_000).max(8_000);
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
                // Ensure num_ctx is passed from configuration
                "num_ctx": self.context_length,
                // Penalise repetition — prevents looping on quantised models
                "repeat_penalty": 1.1f32,
                "num_thread": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            }
        });

        let mut response_text = String::new();
        let mut token_count = 0u32;
        let mut token_batch = String::new();
        let llm_start = std::time::Instant::now();
        let mut last_stream_emit = std::time::Instant::now();
        // Smaller batches keep the UI responsive without overwhelming IPC.
        const BATCH_SIZE: usize = 16;

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

                                let should_emit = token_count % BATCH_SIZE as u32 == 0
                                    || last_stream_emit.elapsed().as_millis() >= 120;
                                if should_emit {
                                    if let Some(app) = &self.app_handle {
                                        if !self.suppress_stream && !token_batch.is_empty() {
                                            let _ = app.emit("agent:stream", StreamToken {
                                                token: token_batch.clone(),
                                                iteration: 0,
                                            });
                                            token_batch.clear();
                                        }
                                        let elapsed = llm_start.elapsed().as_secs_f32().max(0.1);
                                        let _ = app.emit("agent:metrics", &serde_json::json!({
                                            "tokens_per_second": token_count as f32 / elapsed,
                                            "total_tokens": token_count,
                                        }));
                                    }
                                    last_stream_emit = std::time::Instant::now();
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
                        let elapsed = llm_start.elapsed().as_secs_f32().max(0.1);
                        let _ = app.emit("agent:metrics", &serde_json::json!({
                            "tokens_per_second": token_count as f32 / elapsed,
                            "total_tokens": token_count,
                        }));
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

        // Normalize a path string — fixes mixed separators from LLM on Windows
        let normalize_path = |p: &str| -> std::path::PathBuf {
            let fixed = p.replace('/', std::path::MAIN_SEPARATOR_STR);
            let path = std::path::PathBuf::from(&fixed);
            if path.is_absolute() {
                path
            } else if let Some(ws) = &wp {
                std::path::Path::new(ws).join(&fixed)
            } else {
                path
            }
        };

        let tool_result: std::result::Result<String, String> = match tc.tool.as_str() {
            "done" => Ok("Task completed".to_string()),
            "read_file" => {
                match tc.args.get("path").and_then(|p| p.as_str()) {
                    Some(p) => {
                        let full = normalize_path(p);
                        let content = tokio::fs::read_to_string(&full)
                            .await
                            .map_err(|e| format!("Read failed: {}", e))?;
                        let start_line = tc.args.get("start_line").and_then(|s| s.as_u64()).map(|n| n as usize);
                        let end_line = tc.args.get("end_line").and_then(|e| e.as_u64()).map(|n| n as usize);
                        Ok(format_read_file_output(&content, start_line, end_line))
                    }
                    None => Err("No path provided".to_string())
                }
            }
            "write_file" => {
                match (tc.args.get("path").and_then(|p| p.as_str()), tc.args.get("content").and_then(|c| c.as_str())) {
                    (Some(p), Some(c)) => {
                        let full = normalize_path(p);
                        if let Some(par) = full.parent() { let _ = tokio::fs::create_dir_all(par).await; }
                        tokio::fs::write(&full, c).await.map(|_| format!("Wrote {}", p)).map_err(|e| format!("Write failed: {}", e))
                    }
                    _ => Err("Missing path or content".to_string())
                }
            }
            // create_file is an alias for write_file
            "create_file" => {
                let p = tc.args.get("path").and_then(|p| p.as_str());
                let c = tc.args.get("content").and_then(|c| c.as_str()).unwrap_or("");
                match p {
                    Some(p) => {
                        let full = normalize_path(p);
                        if let Some(par) = full.parent() { let _ = tokio::fs::create_dir_all(par).await; }
                        tokio::fs::write(&full, c).await.map(|_| format!("Created {}", p)).map_err(|e| format!("Create failed: {}", e))
                    }
                    None => Err("No path provided".to_string())
                }
            }
            "create_directory" => {
                match tc.args.get("path").and_then(|p| p.as_str()) {
                    Some(p) => {
                        tokio::fs::create_dir_all(normalize_path(p)).await.map(|_| format!("Created directory {}", p)).map_err(|e| format!("Create dir failed: {}", e))
                    }
                    None => Err("No path provided".to_string())
                }
            }
            "delete_file" => {
                match tc.args.get("path").and_then(|p| p.as_str()) {
                    Some(p) => {
                        let full = normalize_path(p);
                        if full.is_dir() {
                            tokio::fs::remove_dir_all(&full).await.map(|_| format!("Deleted {}", p)).map_err(|e| format!("Delete failed: {}", e))
                        } else {
                            tokio::fs::remove_file(&full).await.map(|_| format!("Deleted {}", p)).map_err(|e| format!("Delete failed: {}", e))
                        }
                    }
                    None => Err("No path provided".to_string())
                }
            }
            "move_file" | "rename_file" => {
                let from = tc.args.get("from").or(tc.args.get("source")).or(tc.args.get("path")).and_then(|p| p.as_str());
                let to = tc.args.get("to").or(tc.args.get("destination")).or(tc.args.get("new_path")).and_then(|p| p.as_str());
                match (from, to) {
                    (Some(f), Some(t)) => {
                        tokio::fs::rename(normalize_path(f), normalize_path(t)).await.map(|_| format!("Moved {} to {}", f, t)).map_err(|e| format!("Move failed: {}", e))
                    }
                    _ => Err("Missing from/to arguments".to_string())
                }
            }
            "edit_file" => {
                match (tc.args.get("path").and_then(|p| p.as_str()), tc.args.get("content").and_then(|c| c.as_str())) {
                    (Some(p), Some(c)) => {
                        let full = normalize_path(p);
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
                        let full = normalize_path(p);
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
                // TIER 1.3: Complete ErrorRecoverySystem integration
                // First try auto-recovery
                let recovery_result = rec.auto_recover(&e, &tc.tool, &wp);
                if recovery_result.recovered {
                    if let Some(action) = recovery_result.suggested_action {
                        eprintln!("[Recovery] Applied: {}", action);
                        Ok(format!("FIXED: {}. {}", e, recovery_result.message))
                    } else {
                        tool_result
                    }
                } else {
                    // If auto-recovery failed, try to get best strategy and execute it
                    if let Some(_best_strategy) = rec.get_best_strategy_for_error(&e.to_string()) {
                        eprintln!("[Recovery] Found recovery strategy for error: {}", e);
                        // Strategy found but execution would require async context
                        // For now, fall back to LLM recovery
                        tool_result
                    } else {
                        eprintln!("[Recovery] No recovery strategy found, falling back to LLM recovery");
                        tool_result
                    }
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
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    // Returns tool results PLUS a sentinel entry (tool=="__response__", result=Ok(full_text))
    // so the caller can reuse the already-streamed LLM text without a second Ollama call.
    ) -> Result<Vec<(ToolCall, Result<String>)>> {
        let mut tool_queue = Vec::new();
        let mut executed_results = Vec::new();
        let mut json_parser = crate::commands::streaming_agent_flow::IncrementalJsonParser::new();
        let mut tool_counter = 0u32;
        let mut rejected_tools = Vec::new(); // Track rejected tool calls
        let llm_start = std::time::Instant::now();
        let mut streamed_token_count = 0u32;
        let mut stream_batch = String::new();
        let mut last_stream_emit = std::time::Instant::now();
        const STREAM_BATCH_SIZE: usize = 12;

        // ── Sliding window: same constants as call_llm_streaming ─────────────
        let mut messages_json = Vec::new();
        let mut char_count = 0;
        let mut iter_messages = messages.iter().enumerate().collect::<Vec<_>>();
        iter_messages.reverse();

        // Pin 0 (system) + 1 (workspace ctx) + 2 (workspace ack) + 3 (task)
        let mut included_indices = std::collections::HashSet::new();
        included_indices.insert(0);
        included_indices.insert(1);
        included_indices.insert(2);
        included_indices.insert(3);

        for (i, (_role, content)) in iter_messages {
            if i <= 3 || included_indices.contains(&i) { continue; }
            let limit = (self.context_length as usize * 4).saturating_sub(4_000).max(8_000);
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
                "repeat_penalty": 1.1f32,
                "num_thread": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            }
        });

        // ─────────────────────────────────────────────────────────
        // PHASE 1: IDENTIFY ALL TOOLS
        // ─────────────────────────────────────────────────────────
        let mut raw_llm_text = String::new();

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
                                        
                                        // Store for the caller to reuse (Stall Detection)
                                        raw_llm_text.push_str(token);
                                        stream_batch.push_str(token);
                                        streamed_token_count += 1;

                                        let should_emit_stream = streamed_token_count % STREAM_BATCH_SIZE as u32 == 0
                                            || last_stream_emit.elapsed().as_millis() >= 120;
                                        if should_emit_stream {
                                            if let Some(app) = &self.app_handle {
                                                if !stream_batch.is_empty() {
                                                    let _ = app.emit("agent:stream", StreamToken {
                                                        token: stream_batch.clone(),
                                                        iteration,
                                                    });
                                                    stream_batch.clear();
                                                }

                                                let elapsed = llm_start.elapsed().as_secs_f32().max(0.1);
                                                let _ = app.emit("agent:metrics", &serde_json::json!({
                                                    "tokens_per_second": streamed_token_count as f32 / elapsed,
                                                    "total_tokens": streamed_token_count,
                                                }));
                                            }
                                            last_stream_emit = std::time::Instant::now();
                                        }

                                        // Feed to incremental JSON parser
                                        let objects = json_parser.feed(token);
                                        
                                        // Process each parsed JSON object
                                    for obj in objects {
                                        if let Some(tool_name) = obj.get("tool").and_then(|t| t.as_str()) {
                                            let args = obj.get("args").cloned().unwrap_or(serde_json::json!({}));
                                            
                                            // Validate required arguments for critical tools
                                            let (is_valid, missing_arg) = match tool_name {
                                                "read_file" | "write_file" | "edit_file" | "multi_edit_file" | "create_file" | "delete_file" | "move_file" | "rename_file" => {
                                                    if args.get("path").and_then(|p| p.as_str()).is_some() {
                                                        (true, None)
                                                    } else {
                                                        (false, Some("path"))
                                                    }
                                                },
                                                "run_command" => {
                                                    if args.get("command").and_then(|c| c.as_str()).is_some() {
                                                        (true, None)
                                                    } else {
                                                        (false, Some("command"))
                                                    }
                                                },
                                                _ => (true, None), // Other tools don't have strict validation
                                            };
                                            
                                            if !is_valid {
                                                eprintln!("[Phase 4] ⚠️ Tool '{}' missing required argument: {:?}, skipping", tool_name, missing_arg);
                                                let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                                                let missing_info = missing_arg.map(|m| format!(" (missing: \"{}\")", m)).unwrap_or_default();
                                                rejected_tools.push(format!("Tool '{}'{} with args: {}", tool_name, missing_info, args_str));
                                                continue;
                                            }
                                            
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
                if !stream_batch.is_empty() {
                    if let Some(app) = &self.app_handle {
                        let _ = app.emit("agent:stream", StreamToken {
                            token: stream_batch,
                            iteration,
                        });

                        let elapsed = llm_start.elapsed().as_secs_f32().max(0.1);
                        let _ = app.emit("agent:metrics", &serde_json::json!({
                            "tokens_per_second": streamed_token_count as f32 / elapsed,
                            "total_tokens": streamed_token_count,
                        }));
                    }
                }

                // ─────────────────────────────────────────────────────────
                // PHASE 2: EXECUTE TOOLS SEQUENTIALLY
                // ─────────────────────────────────────────────────────────
                eprintln!("[Phase 4] PHASE 2: Executing tools sequentially");
                
                let mut current_index = 0;
                while current_index < tool_queue.len() {
                    if crate::commands::agent::is_agent_cancelled() { break; }

                    let tool_call = tool_queue[current_index].clone();
                    
                    // Skip terminal tools
                    // 'done' is handled by the caller, but 'ask_user' must be executed to block/interrupt.
                    if tool_call.tool == "done" {
                        eprintln!("[Phase 4] LLM signaled completion with 'done'. Stopping queue.");
                        break; 
                    }
                    
                    if tool_call.tool == "ask_user" {
                        eprintln!("[Phase 4] LLM needs info via 'ask_user'. Executing and stopping queue.");
                        let result = self.execute_tool_with_recovery(
                            &tool_call, workspace_path, iteration, current_index,
                            recovery.clone(), app_state_ref.clone(), messages, model_name,
                            vector_system.clone(), code_intel.clone(),
                        ).await;
                        executed_results.push((tool_call.clone(), result));
                        break; // Stop the queue to allow user feedback to become the next input
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
                        vector_system.clone(),
                        code_intel.clone(),
                    ).await;

                    // Tools are executed directly without blocking LLM recovery calls.
                    // Errors are captured in `result` and fed to the next main loop iteration
                    // where the model has full context to recover naturally.
                    executed_results.push((tool_call.clone(), result));
                    current_index += 1;
                }

                eprintln!("[Phase 4] PHASE 2 COMPLETE: {} tools executed", executed_results.len());
                
                // Flush any remaining events
                self.flush_events().await;
                
                // Add rejected tools as a special entry so the caller can track them
                if !rejected_tools.is_empty() {
                    executed_results.push((
                        ToolCall { tool: "__rejected_tools__".to_string(), args: serde_json::json!({ "tools": rejected_tools }) },
                        Ok("".to_string()),
                    ));
                }
                
                // Append sentinel carrying the full raw response text so the caller can
                // reuse it for stall-detection without making another Ollama call.
                executed_results.push((
                    ToolCall { tool: "__response__".to_string(), args: serde_json::json!({}) },
                    Ok(raw_llm_text),
                ));
                
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
        _recovery: Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>,
        app_state_ref: Arc<RwLock<AppState>>,
        _turn_messages: &[(String, String)],
        _model_name: &str,
        vector_system: Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    ) -> Result<String> {
        let args_json = serde_json::to_string(&tool_call.args)
            .unwrap_or_else(|_| "{}".to_string());
        let request_id = format!("tool_{}_{}", iteration, tool_idx);

        if tool_call.tool == "run_command" {
            if let Some(command) = tool_call.args.get("command").and_then(|value| value.as_str()) {
                if is_high_risk_command(command) {
                    let approval_step = AgentStep {
                        iteration,
                        tool: tool_call.tool.clone(),
                        status: "awaiting_permission".to_string(),
                        summary: format!("Explicit approval required for high-risk command: {}", command),
                        result: None,
                        logs: Some(vec![format!("$ {}", command)]),
                        persona: Some("agent".to_string()),
                        request_id: Some(request_id.clone()),
                        data: Some(serde_json::json!({
                            "riskLevel": "high",
                            "requiresExplicitApproval": true,
                            "command": command,
                        })),
                    };
                    self.emit_step(approval_step).await;

                    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                    {
                        let mut permission_tx = crate::commands::agent::PERMISSION_TX.lock().unwrap();
                        *permission_tx = Some(tx);
                    }

                    match rx.await {
                        Ok(true) => {}
                        Ok(false) => {
                            let denied_step = AgentStep {
                                iteration,
                                tool: tool_call.tool.clone(),
                                status: "skipped".to_string(),
                                summary: format!("High-risk command blocked by user: {}", command),
                                result: Some("Permission denied for high-risk command.".to_string()),
                                logs: Some(vec![
                                    format!("$ {}", command),
                                    "Execution blocked because explicit approval was denied.".to_string(),
                                ]),
                                persona: Some("agent".to_string()),
                                request_id: Some(request_id.clone()),
                                data: Some(serde_json::json!({
                                    "riskLevel": "high",
                                    "requiresExplicitApproval": true,
                                    "command": command,
                                })),
                            };
                            self.emit_step(denied_step).await;
                            return Err("Permission denied for high-risk command.".into());
                        }
                        Err(_) => {
                            let failed_step = AgentStep {
                                iteration,
                                tool: tool_call.tool.clone(),
                                status: "failed".to_string(),
                                summary: format!("High-risk approval request failed: {}", command),
                                result: Some("Failed waiting for permission response.".to_string()),
                                logs: Some(vec![
                                    format!("$ {}", command),
                                    "The backend did not receive a permission response.".to_string(),
                                ]),
                                persona: Some("agent".to_string()),
                                request_id: Some(request_id.clone()),
                                data: Some(serde_json::json!({
                                    "riskLevel": "high",
                                    "requiresExplicitApproval": true,
                                    "command": command,
                                })),
                            };
                            self.emit_step(failed_step).await;
                            return Err("Failed waiting for permission response.".into());
                        }
                    }
                }
            }
        }

        // Emit "running" status
        let running_step = AgentStep {
            iteration,
            tool: tool_call.tool.clone(),
            status: "running".to_string(),
            summary: format!("Executing {} with args: {}", tool_call.tool, args_json),
            result: None,
            logs: None,
            persona: Some("agent".to_string()),
            request_id: Some(request_id.clone()),
            data: None,
        };
        self.emit_step(running_step).await;

        // Execute the tool - use streaming for run_command
        let tool_result = if tool_call.tool == "run_command" {
            self.execute_run_command_streaming(tool_call, workspace_path, iteration, tool_idx, app_state_ref).await
        } else {
            execute_tool_standalone(tool_call, workspace_path, &vector_system, &code_intel, self.app_handle.as_ref()).await
        };

        // Redundant mid-execution LLM recovery calls have been removed.
        // The error is instead returned in `tool_result` and properly fed to the 
        // next agent turn in `turn_messages` so the main agent LLM can self-correct natively.

        // Emit final completion or failure status
        let result_text = if tool_result.is_ok() {
            tool_result.as_ref().ok().cloned()
        } else {
            tool_result.as_ref().err().map(|e| e.to_string())
        };

        // Check if result contains error indicators (even if tool_result.is_ok())
        // Treat errors in results as failures so the agent knows to retry
        let has_errors = result_text.as_ref()
            .map(|r| r.contains("error:") || r.contains("Error") || r.contains("failed") || r.contains("Failed") || r.contains("SyntaxError") || r.contains("TypeError") || r.contains("does not provide") || r.contains("does not export") || r.contains("ERR_UNKNOWN_FILE_EXTENSION") || r.contains("ERR_MODULE_NOT_FOUND"))
            .unwrap_or(false);

        let status = if tool_result.is_ok() && !has_errors { 
            "completed" 
        } else { 
            "failed" 
        };

        let completed_step = AgentStep {
            iteration,
            tool: tool_call.tool.clone(),
            status: status.to_string(),
            summary: format!("Executed {} with args: {}", tool_call.tool, args_json),
            result: result_text.clone(),
            logs: extract_completion_logs(&tool_call.tool, result_text.as_ref()),
            persona: Some("agent".to_string()),
            request_id: Some(request_id),
            data: None,
        };
        self.emit_step(completed_step).await;

        // If result contains errors, convert to Err so agent treats it as a failure
        if has_errors && tool_result.is_ok() {
            Err(result_text.unwrap_or_else(|| "Tool execution resulted in errors".to_string()).into())
        } else {
            tool_result
        }
    }

    /// Ask LLM for recovery strategy when a tool fails
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
                logs: extract_completion_logs(&tool_call.tool, result_text.as_ref()),
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
                                        
                                        // Validate required arguments for critical tools
                                        let (is_valid, missing_arg) = match tool_name {
                                            "read_file" | "write_file" | "edit_file" | "multi_edit_file" | "create_file" | "delete_file" | "move_file" | "rename_file" => {
                                                if args.get("path").and_then(|p| p.as_str()).is_some() {
                                                    (true, None)
                                                } else {
                                                    (false, Some("path"))
                                                }
                                            },
                                            "run_command" => {
                                                if args.get("command").and_then(|c| c.as_str()).is_some() {
                                                    (true, None)
                                                } else {
                                                    (false, Some("command"))
                                                }
                                            },
                                            _ => (true, None), // Other tools don't have strict validation
                                        };
                                        
                                        if !is_valid {
                                            eprintln!("[Sub-Agent] ⚠️ Tool '{}' missing required argument: {:?}, skipping", tool_name, missing_arg);
                                            continue;
                                        }
                                        
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

    // ── INTELLIGENT PROBLEM IDENTIFICATION ──────────────────────────────────
    /// Analyze the task and generate targeted investigation guidance
    fn analyze_problem_intelligently(task: &str) -> String {
        let analysis = ProblemIdentifier::analyze_problem(task);
        
        let mut guidance = String::new();
        guidance.push_str("\n\n<intelligent_investigation>\n");
        guidance.push_str("## Problem Analysis\n\n");
        
        // Keywords
        if !analysis.keywords.is_empty() {
            guidance.push_str("**Keywords identified:** ");
            guidance.push_str(&analysis.keywords.join(", "));
            guidance.push_str("\n\n");
        }
        
        // Suspected files
        if !analysis.suspected_files.is_empty() {
            guidance.push_str("**Suspected files (prioritized by relevance):**\n");
            for file in &analysis.suspected_files {
                guidance.push_str(&format!(
                    "- `{}` (score: {}) - {}\n",
                    file.path, file.relevance_score, file.reason
                ));
            }
            guidance.push_str("\n");
        }
        
        // Search queries
        if !analysis.search_queries.is_empty() {
            guidance.push_str("**Targeted search queries (by priority):**\n");
            for query in analysis.search_queries.iter().take(5) {
                guidance.push_str(&format!(
                    "- Pattern: `{}` in `{}` - {}\n",
                    query.pattern, query.file_pattern, query.reason
                ));
            }
            guidance.push_str("\n");
        }
        
        // Investigation strategy
        guidance.push_str(&analysis.investigation_strategy);
        guidance.push_str("\n</intelligent_investigation>\n");
        
        guidance
    }

    fn build_issue_focus_context(
        &self,
        workspace_path: &Option<String>,
        active_file: &Option<serde_json::Value>,
        user_message: &str,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
    ) -> String {
        let mut ctx = String::new();
        let analysis = ProblemIdentifier::analyze_problem(user_message);
        let line_regex = Regex::new(r":(\d+)(?::\d+)?").ok();
        let explicit_file_regex = Regex::new(r"([A-Za-z0-9_./\\-]+\.(?:tsx?|jsx?|rs|py|go|json|toml|md))").ok();

        let mut candidate_files: Vec<String> = analysis
            .suspected_files
            .iter()
            .take(5)
            .map(|file| file.path.clone())
            .collect();

        if let Some(regex) = explicit_file_regex {
            for capture in regex.captures_iter(user_message) {
                if let Some(candidate) = capture.get(1).map(|m| m.as_str().to_string()) {
                    if !candidate_files.contains(&candidate) {
                        candidate_files.insert(0, candidate);
                    }
                }
            }
        }

        let referenced_line = line_regex
            .as_ref()
            .and_then(|regex| regex.captures(user_message))
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<usize>().ok());

        if candidate_files.is_empty() && active_file.is_none() {
            return ctx;
        }

        ctx.push_str("<issue_focus>\n");
        ctx.push_str("Use this focused scope before broad exploration.\n");

        if !candidate_files.is_empty() {
            ctx.push_str("Priority files:\n");
            for file in candidate_files.iter().take(5) {
                ctx.push_str(&format!("- {}\n", file));
            }
        }

        if let Some(line) = referenced_line {
            let start = line.saturating_sub(20).max(1);
            let end = line + 20;
            ctx.push_str(&format!("Referenced line window: {}-{}\n", start, end));
        }

        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                ctx.push_str(&format!("Active file: {}\n", path));
            }
        }

        if let Some(ws) = workspace_path {
            if let Ok(intel) = code_intel.lock() {
                if let Ok(context) = intel.analyze_workspace_if_stale(ws.to_string()) {
                    let related_symbols: Vec<String> = context.symbols
                        .iter()
                        .filter(|symbol| {
                            candidate_files.iter().any(|candidate| symbol.file_path.contains(candidate))
                                || analysis.keywords.iter().any(|keyword| symbol.name.to_lowercase().contains(&keyword.to_lowercase()))
                        })
                        .take(10)
                        .map(|symbol| format!("{} ({} @ {}:{})", symbol.name, symbol.symbol_type, symbol.file_path, symbol.line_number))
                        .collect();

                    if !related_symbols.is_empty() {
                        ctx.push_str("Related symbols from cached context:\n");
                        for symbol in related_symbols {
                            ctx.push_str(&format!("- {}\n", symbol));
                        }
                    }
                }
            }
        }

        ctx.push_str("Investigation policy:\n");
        ctx.push_str("- Start with semantic_search or find_symbols using the issue keywords.\n");
        ctx.push_str("- If you must read a file, read only the suspected file or referenced line window first.\n");
        ctx.push_str("- Expand to dependent files only after the local cause is identified.\n");
        ctx.push_str("- Reuse cached workspace structure and related symbols before opening additional files.\n");
        ctx.push_str("</issue_focus>\n\n");
        ctx
    }

    fn extract_referenced_line(user_message: &str) -> Option<usize> {
        Regex::new(r":(\d+)(?::\d+)?")
            .ok()
            .and_then(|regex| regex.captures(user_message))
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<usize>().ok())
    }

    #[allow(dead_code)]
    fn format_active_file_context(path: &str, content: &str, referenced_line: Option<usize>) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return format!("<active_file_content path=\"{}\" />\n", path);
        }

        let (start, end, truncated) = if let Some(line) = referenced_line {
            let start = line.saturating_sub(20).max(1);
            let end = (line + 20).min(lines.len());
            (start, end, start > 1 || end < lines.len())
        } else {
            const MAX_ACTIVE_FILE_LINES: usize = 120;
            let end = lines.len().min(MAX_ACTIVE_FILE_LINES);
            (1, end, lines.len() > MAX_ACTIVE_FILE_LINES)
        };

        let displayed = lines[start - 1..end].join("\n");
        if truncated {
            format!(
                "<active_file_content path=\"{}\" start_line=\"{}\" end_line=\"{}\" truncated=\"true\">\n{}\n... (use read_file with start_line/end_line to inspect more)\n</active_file_content>\n",
                path, start, end, displayed
            )
        } else {
            format!(
                "<active_file_content path=\"{}\" start_line=\"{}\" end_line=\"{}\">\n{}\n</active_file_content>\n",
                path, start, end, displayed
            )
        }
    }

    // ── FIX #2: System prompt now includes active file CONTENT ──────────────
    /// Lean system prompt: only static rules + shell environment + learned insights.
    /// Stable across all iterations — fits in the model's system-prompt cache.
    fn get_system_prompt(
        &self,
        detected_shell: &str,
        learning: Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>,
    ) -> String {
        let mut prompt = prompts::WHIZCODE_SYSTEM_PROMPT.to_string();

        // ── SHELL INFORMATION ──────────────────────────────────────────────
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
        prompt.push_str("\n- ALWAYS use non-interactive flags (e.g. -y, --yes, --force) for commands to prevent the agent from hanging.");
        prompt.push_str("\n- DO NOT combine multiple commands into a single 'run_command' unless the first is 'cd'.");
        prompt.push_str("\n- ALWAYS ensure you are in the correct directory before running any command.");
        prompt.push_str("</shell_environment>");

        // ── LEARNED INSIGHTS (session-level, rarely changes) ──────────────
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

        prompt
    }

    /// Builds the rich workspace context that is sent ONCE before the agent loop
    /// as a priming user message, not re-injected on every LLM call.
    /// Contains: workspace path, file tree (cached), steering rules, KIs,
    /// workflows, git status+diff, code intelligence, active file content,
    /// and dynamic prompt fragments derived from real workspace extensions.
    fn build_workspace_context(
        &self,
        workspace_path: &Option<String>,
        active_file: &Option<serde_json::Value>,
        user_message: &str,
        code_intel: Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>,
        steering: Arc<RwLock<SteeringSystem>>,
    ) -> String {
        let mut ctx = String::new();

        // ── STEERING RULES ────────────────────────────────────────────────
        if let Some(ws) = workspace_path {
            let s = steering.read();
            if let Some(steered_context) = s.get_injected_context(ws) {
                if !steered_context.is_empty() {
                    ctx.push_str("<steering_rules>\n");
                    ctx.push_str(&steered_context);
                    ctx.push_str("\n</steering_rules>\n\n");
                }
            }
        }

        if let Some(ws) = workspace_path {
            let issue_focus = self.build_issue_focus_context(
                workspace_path,
                active_file,
                user_message,
                code_intel.clone(),
            );
            ctx.push_str(&format!(
                "<workspace_root>{}\nIMPORTANT: Use this EXACT path in all file operations.</workspace_root>\n\n",
                ws
            ));

            // ── FILE TREE (cached, 5-min TTL, capped at 200 entries) ──────
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
            ctx.push_str(&format!("<workspace_structure>\n{}\n</workspace_structure>\n\n", file_tree));
            if !issue_focus.is_empty() {
                ctx.push_str(&issue_focus);
            }

            // ── DYNAMIC PROMPT FRAGMENTS (extensions derived from file tree) ─
            // Extract real extensions from the file tree instead of a hardcoded list
            let extensions: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                file_tree.lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        let dot_pos = trimmed.rfind('.')?;
                        let ext = &trimmed[dot_pos + 1..];
                        // Only accept short, clean extensions (no path separators)
                        if ext.len() <= 6 && !ext.contains('/') && !ext.contains('\\') {
                            Some(ext.to_lowercase())
                        } else {
                            None
                        }
                    })
                    .filter(|e| seen.insert(e.clone()))
                    .take(20)
                    .collect()
            };
            let prompt_manager = crate::commands::prompt_manager::PromptManager::new();
            let dynamic_suffix = prompt_manager.get_relevant_fragments(user_message, &extensions, &[ws.clone()]);
            if !dynamic_suffix.is_empty() {
                ctx.push_str(&dynamic_suffix);
                ctx.push('\n');
            }

            // ── KNOWLEDGE ITEMS ───────────────────────────────────────────
            if let Ok(lore) = crate::commands::distillation::load_relevant_knowledge(std::path::Path::new(ws)) {
                if !lore.is_empty() {
                    ctx.push_str(&lore);
                    ctx.push('\n');
                }
            }

            // ── WORKFLOWS & SKILLS ────────────────────────────────────────
            let workflows_context = crate::commands::workflows::get_workflows_context(std::path::Path::new(ws));
            if !workflows_context.is_empty() {
                ctx.push_str(&workflows_context);
                ctx.push('\n');
            }

            // ── GIT STATUS & DIFF (run once, not per iteration) ───────────
            if let Ok(repo_path) = std::path::Path::new(ws).canonicalize() {
                if let Ok(output) = std::process::Command::new("git")
                    .args(["status", "--short"])
                    .current_dir(&repo_path)
                    .output()
                {
                    let status_str = String::from_utf8_lossy(&output.stdout);
                    if !status_str.trim().is_empty() {
                        ctx.push_str(&format!("<git_status>\n{}\n</git_status>\n\n", status_str));
                    }
                }
                if let Ok(output) = std::process::Command::new("git")
                    .args(["diff", "HEAD"])
                    .current_dir(&repo_path)
                    .output()
                {
                    let diff_str = String::from_utf8_lossy(&output.stdout);
                    if !diff_str.trim().is_empty() {
                        const MAX_DIFF_CHARS: usize = 3_000;
                        let capped = if diff_str.len() > MAX_DIFF_CHARS {
                            format!("{}\n... (diff truncated)", &diff_str[..MAX_DIFF_CHARS])
                        } else {
                            diff_str.to_string()
                        };
                        ctx.push_str(&format!("<git_diff>\n{}\n</git_diff>\n\n", capped));
                    }
                }
            }

            // ── CODE INTELLIGENCE METRICS ─────────────────────────────────
            if let Ok(intel) = code_intel.lock() {
                if let Some(metrics) = intel.get_code_metrics(ws) {
                    ctx.push_str(&format!(
                        "<code_intelligence>\nMetrics: Complexity={:.1}, Maintainability={:.1}, TechnicalDebt={:.2}\n</code_intelligence>\n\n",
                        metrics.average_complexity, metrics.maintainability_index, metrics.technical_debt
                    ));
                }
            }
        }

        // ── ACTIVE FILE CONTENT (capped at 300 lines) ─────────────────────
        if let Some(file) = active_file {
            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                    let focused_content = if let Some(line) = Self::extract_referenced_line(user_message) {
                        let content_lines: Vec<&str> = content.lines().collect();
                        let start = line.saturating_sub(20).max(1);
                        let end = (line + 20).min(content_lines.len());
                        content_lines[start - 1..end].join("\n")
                    } else {
                        content.to_string()
                    };
                    let lines: Vec<&str> = focused_content.lines().collect();
                    const MAX_ACTIVE_FILE_LINES: usize = 120;
                    if lines.len() <= MAX_ACTIVE_FILE_LINES {
                        ctx.push_str(&format!("<active_file_content path=\"{}\">\n{}\n</active_file_content>\n", path, focused_content));
                    } else {
                        let displayed = lines.iter().take(MAX_ACTIVE_FILE_LINES).cloned().collect::<Vec<_>>().join("\n");
                        ctx.push_str(&format!(
                            "<active_file_content path=\"{}\" truncated=\"true\">\n{}\n... ({} more lines — use read_file for full content)\n</active_file_content>\n",
                            path, displayed, lines.len() - MAX_ACTIVE_FILE_LINES
                        ));
                    }
                } else {
                    // At minimum, tell the agent which file is active
                    ctx.push_str(&format!("<active_file path=\"{}\"/>\n", path));
                }
            }
        }

        ctx
    }
}

// ─────────────────────────────────────────────
// Standalone tool executor (used by parallel futures)
// ─────────────────────────────────────────────

fn format_read_file_output(content: &str, start_line: Option<usize>, end_line: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return "File contents (0 lines):".to_string();
    }

    let (start, end, truncated) = if start_line.is_none() && end_line.is_none() {
        const DEFAULT_PREVIEW_LINES: usize = 120;
        let end = lines.len().min(DEFAULT_PREVIEW_LINES);
        (1, end, lines.len() > DEFAULT_PREVIEW_LINES)
    } else {
        let start = start_line.unwrap_or(1).max(1);
        let end = end_line.unwrap_or(lines.len()).min(lines.len());
        (start, end.max(start), false)
    };

    let numbered_slice = lines[start - 1..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", start + i, line))
        .collect::<Vec<_>>()
        .join("\n");

    if truncated {
        format!(
            "File preview (lines {}-{} of {}):\n{}\n... (preview truncated; use read_file with start_line/end_line for a specific range)",
            start,
            end,
            lines.len(),
            numbered_slice
        )
    } else {
        format!("File contents (lines {}-{}):\n{}", start, end, numbered_slice)
    }
}

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
        // Normalize path separators — LLMs sometimes mix / and \ on Windows
        let normalized_sep = p.replace('/', std::path::MAIN_SEPARATOR_STR);
        let normalized = if normalized_sep.starts_with("/workspace/") {
            normalized_sep.replacen("/workspace/", "", 1)
        } else if normalized_sep == "/workspace" {
            String::new()
        } else {
            normalized_sep
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

    let mut tool_result: Result<String> = match tool_call.tool.as_str() {
        // ── FIX #8: read_file now accepts optional start_line / end_line ─────
        "read_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;

            let content = tokio::fs::read_to_string(resolve(path)).await?;
            let start_line = tool_call.args.get("start_line").and_then(|s| s.as_u64()).map(|n| n as usize);
            let end_line   = tool_call.args.get("end_line").and_then(|e| e.as_u64()).map(|n| n as usize);
            Ok(format_read_file_output(&content, start_line, end_line))
        }

        "view_structure" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let content = tokio::fs::read_to_string(resolve(path)).await?;
            let mut skeleton = String::new();
            let keywords = vec!["import", "export", "function", "const", "let", "var", "if", "else", "return", "class", "interface", "type", "enum", "async", "await", "try", "catch", "default"];
            
            for (i, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                
                let mut line_skele = String::new();
                let words: Vec<&str> = line.split_whitespace().collect();
                
                // If a line starts with a keyword, preserve it then skeletonize the rest
                for word in words {
                    let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if keywords.contains(&clean_word) {
                        line_skele.push_str(word);
                        line_skele.push(' ');
                    } else {
                        // Skeletonize word but preserve structure
                        for ch in word.chars() {
                            if "{}[],()<>:;=!".contains(ch) {
                                line_skele.push(ch);
                            } else {
                                line_skele.push('.');
                            }
                        }
                        line_skele.push(' ');
                    }
                }
                skeleton.push_str(&format!("{:4} | {}\n", i + 1, line_skele));
            }
            Ok(format!("Structural Skeleton of {}:\n{}", path, skeleton))
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

        // create_file is an alias for write_file — LLMs often use this name
        "create_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let content = tool_call.args.get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let resolved_path = resolve(path);
            if let Some(parent) = resolved_path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            tokio::fs::write(&resolved_path, content).await?;
            Ok(format!("Successfully created {}", path))
        }

        "create_directory" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            tokio::fs::create_dir_all(resolve(path)).await?;
            Ok(format!("Successfully created directory {}", path))
        }

        "delete_file" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            let resolved_path = resolve(path);
            if resolved_path.is_dir() {
                tokio::fs::remove_dir_all(&resolved_path).await?;
            } else {
                tokio::fs::remove_file(&resolved_path).await?;
            }
            Ok(format!("Successfully deleted {}", path))
        }

        "move_file" | "rename_file" => {
            let from = tool_call.args.get("from").or(tool_call.args.get("source")).or(tool_call.args.get("path"))
                .and_then(|p| p.as_str())
                .ok_or("Missing from/source argument")?;
            let to = tool_call.args.get("to").or(tool_call.args.get("destination")).or(tool_call.args.get("new_path"))
                .and_then(|p| p.as_str())
                .ok_or("Missing to/destination argument")?;
            tokio::fs::rename(resolve(from), resolve(to)).await?;
            Ok(format!("Successfully moved {} to {}", from, to))
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
                let start_line = edit.get("start_line").and_then(|s| s.as_u64()).map(|s| s as usize);
                let end_line   = edit.get("end_line").and_then(|e| e.as_u64()).map(|e| e as usize);

                if let (Some(sl), Some(el)) = (start_line, end_line) {
                    let lines: Vec<&str> = content.lines().collect();
                    let end_idx = el.min(lines.len());
                    let start_idx = sl.saturating_sub(1).min(end_idx);
                    
                    let mut sliced_content = lines[start_idx..end_idx].join("\n");
                    if sliced_content.contains(search) {
                        sliced_content = sliced_content.replacen(search, replace, 1);
                        // Rebuild file
                        let mut new_lines = lines[..start_idx].to_vec();
                        new_lines.push(&sliced_content);
                        if end_idx < lines.len() {
                            new_lines.extend_from_slice(&lines[end_idx..]);
                        }
                        content = new_lines.join("\n");
                        applied += 1;
                    } else {
                        errors.push(format!("Could not find search string between lines {}-{}", sl, el));
                    }
                } else {
                    // Fallback to full file search
                    if content.contains(search) {
                        content = content.replacen(search, replace, 1);
                        applied += 1;
                    } else {
                        errors.push(format!("Could not find: {:?}", &search[..search.len().min(60)]));
                    }
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
            
            // ── LONG-RUNNING COMMAND DETECTION ──────────────────────────────────
            // Prevent agent from running dev servers, watchers, and other long-running processes
            let cmd_lower = command.to_lowercase();
            let long_running_patterns = [
                "npm run dev", "npm start", "yarn start", "yarn dev",
                "webpack --watch", "jest --watch", "vitest --watch",
                "python -m http.server", "python -m SimpleHTTPServer",
                "node server", "node app", "node index",
                "npm run watch", "npm run serve",
                "ng serve", "ng start",
                "cargo run", "cargo watch",
                "go run", "go build",
                "ruby -r webrick", "python manage.py runserver",
                "rails server", "rails s",
                "php -S", "php artisan serve",
                "dotnet run", "dotnet watch",
            ];
            
            let is_long_running = long_running_patterns.iter().any(|p| cmd_lower.contains(p));
            
            if is_long_running {
                eprintln!("[run_command] ⚠️ LONG-RUNNING COMMAND BLOCKED: {}", command);
                return Err(format!(
                    "LONG_RUNNING_COMMAND_BLOCKED: '{}' is a development server or watch process that runs indefinitely. \
                    This would cause the agent to hang. \
                    If you need to verify the build works, use 'npm run build' instead. \
                    If you need to run the dev server, do it manually in your terminal.",
                    command
                ).into());
            }
            
            // ── SAFETY BLACKLIST (Prevents agent panic-deletions) ────────────────
            let blacklisted_terms = ["rm ", "del ", "remove-item", "rd ", "rmdir", "nuke", "truncate"];
            let is_destructive = blacklisted_terms.iter().any(|t| cmd_lower.contains(t));
            let is_whitelisted = cmd_lower.contains("node_modules") || cmd_lower.contains(".whizcode") || cmd_lower.contains("tmp");
            
            if is_destructive && !is_whitelisted {
                return Err("DESTRUCTIVE_COMMAND_BLOCKED: You are not permitted to delete project source files. Use edit_file instead.".into());
            }

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

        "get_file_relationships" => {
            let path = tool_call.args.get("path")
                .and_then(|p| p.as_str())
                .ok_or("Missing path argument")?;
            
            let intel = code_intel.lock().unwrap();
            let ctx = intel.analyze_workspace(ws_root.to_string()).ok();
            
            if let Some(c) = ctx {
                let target_file = resolve(path).to_string_lossy().to_string();
                
                // 1. Outbound (What this file depends on)
                let outbound: Vec<_> = c.relationships.iter()
                    .filter(|r| r.from_symbol == target_file)
                    .map(|r| format!("{}: {}", r.relationship_type, r.to_symbol))
                    .collect();

                // 2. Inbound (What depends on this file)
                let inbound: Vec<_> = c.relationships.iter()
                    .filter(|r| r.to_symbol.contains(&target_file))
                    .map(|r| r.from_symbol.clone())
                    .collect();

                let mut out = format!("Knowledge Graph for {}:\n", path);
                out.push_str(&format!("\nDEPENDENCIES (Outbound): \n{}", if outbound.is_empty() { "None found.".to_string() } else { outbound.join("\n") }));
                out.push_str(&format!("\n\nUSED BY (Inbound): \n{}", if inbound.is_empty() { "None found.".to_string() } else { inbound.join("\n") }));
                
                return Ok(out);
            }
            
            Err("No code context found for workspace. Use analyze_workspace first.".into())
        }

        "done" => Ok("Task completed successfully.".to_string()),

        "search_web" => {
            let query = tool_call.args.get("query").and_then(|q| q.as_str()).ok_or("Missing query")?;
            let results = crate::commands::web_search::search_web(query.to_string()).await?;
            let mut out = format!("Search results for '{}' (external sources; verify against local code when possible):\n", query);
            for (i, r) in results.iter().enumerate() {
                out.push_str(&format!(
                    "{}. {} ({})\n   Domain: {} | Retrieved: {}\n   {}\n",
                    i + 1,
                    r.title,
                    r.url,
                    r.domain,
                    r.retrieved_at,
                    r.snippet
                ));
            }
            Ok(out)
        }

        "read_url_content" => {
            let url = tool_call.args.get("url").and_then(|u| u.as_str()).ok_or("Missing url")?;
            let content = crate::commands::web_search::read_url_content(url.to_string()).await?;
            Ok(format!("External content from {}:\n\n{}", url, content))
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
            let question = tool_call.args.get("question")
                .or_else(|| tool_call.args.get("message"))
                .and_then(|q| q.as_str())
                .unwrap_or("What info do you need?");
            
            // Validate that this is actually a question, not a statement
            let is_question = question.trim().ends_with('?') 
                || question.to_lowercase().starts_with("what ")
                || question.to_lowercase().starts_with("which ")
                || question.to_lowercase().starts_with("where ")
                || question.to_lowercase().starts_with("when ")
                || question.to_lowercase().starts_with("why ")
                || question.to_lowercase().starts_with("how ")
                || question.to_lowercase().starts_with("do you ")
                || question.to_lowercase().starts_with("can you ")
                || question.to_lowercase().starts_with("should ");
            
            if !is_question {
                eprintln!("[ask_user] ⚠️ MISUSE DETECTED: Not actually asking a question");
                eprintln!("[ask_user] Message: {}", question);
                
                // Check if this looks like a completion statement
                if question.to_lowercase().contains("fixed") 
                    || question.to_lowercase().contains("complete")
                    || question.to_lowercase().contains("done")
                    || question.to_lowercase().contains("created")
                    || question.to_lowercase().contains("successfully") {
                    eprintln!("[ask_user] This looks like a completion statement, not a question");
                    return Ok("[SYSTEM] ERROR: You used 'ask_user' but you're not asking a question. You're making a statement. Use 'done' tool instead to indicate task completion: {\"thought\": \"Task is complete\", \"tool\": \"done\", \"args\": {}}".to_string());
                }
                
                return Ok("[SYSTEM] ERROR: 'ask_user' must be used to ASK a question, not to make statements. Your message should end with '?' and actually request information from the user.".to_string());
            }
            
            eprintln!("[ask_user] Question for user: {}", question);
            
            // Emit special event for the frontend to show a prompt
            if let Some(handle) = _app_handle {
                let _ = handle.emit("agent:ask_user", serde_json::json!({
                    "question": question
                }));
            }

            // Create a channel and wait for response via PERMISSION_TX (common channel)
            let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
            {
                let mut permission_tx = crate::commands::agent::PERMISSION_TX.lock().unwrap();
                *permission_tx = Some(tx);
            }
            
            // Wait for user to approve/deny
            match rx.await {
                Ok(true) => Ok(format!("User approved/responded to: '{}'. Proceed.", question)),
                Ok(false) => Err("User denied the request or cancelled.".into()),
                Err(_) => Err("Failed to wait for user response.".into()),
            }
        }

        _ => return Err(format!("Unknown tool: {}", tool_call.tool).into()),
    };

    // ── ZERO-COST LINTER INJECTION ──
    let is_edit_tool = ["write_file", "edit_file", "multi_edit_file", "create_file"].contains(&tool_call.tool.as_str());
    if is_edit_tool && tool_result.is_ok() {
        if let Some(path_arg) = tool_call.args.get("path").and_then(|p| p.as_str()) {
            let mut linter_output = String::new();
            if path_arg.ends_with(".ts") || path_arg.ends_with(".tsx") || path_arg.ends_with(".js") || path_arg.ends_with(".jsx") {
                let is_typescript = path_arg.ends_with(".ts") || path_arg.ends_with(".tsx");
                let is_plain_js = path_arg.ends_with(".js") || path_arg.ends_with(".mjs") || path_arg.ends_with(".cjs");

                // Prefer TypeScript-aware validation for TS/TSX. Avoid node --check for TSX because Node cannot parse it.
                if let Ok(cmd) = tokio::process::Command::new("npx").args(["tsc", "--noEmit"]).current_dir(ws_root).output().await {
                    if !cmd.status.success() {
                        let stdout = String::from_utf8_lossy(&cmd.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&cmd.stderr).to_string();
                        linter_output = if stdout.trim().is_empty() { stderr } else { stdout };
                    }
                }

                if linter_output.is_empty() && !is_typescript && is_plain_js {
                    if let Ok(cmd) = tokio::process::Command::new("node").args(["--check", path_arg]).current_dir(ws_root).output().await {
                        if !cmd.status.success() {
                            linter_output = String::from_utf8_lossy(&cmd.stderr).to_string();
                        }
                    }
                }
            } else if path_arg.ends_with(".rs") {
                if let Ok(cmd) = tokio::process::Command::new("cargo").args(["check"]).current_dir(ws_root).output().await {
                    if !cmd.status.success() {
                        linter_output = String::from_utf8_lossy(&cmd.stderr).to_string();
                    }
                }
            } else if path_arg.ends_with(".py") {
                if let Ok(cmd) = tokio::process::Command::new("python").args(["-m", "pyflakes", path_arg]).current_dir(ws_root).output().await {
                    if !cmd.status.success() {
                        linter_output = String::from_utf8_lossy(&cmd.stdout).to_string();
                    }
                }
            }
            if !linter_output.trim().is_empty() {
                // Truncate to avoid blowing up context
                let max_len = 2000;
                let c_out = if linter_output.len() > max_len { format!("{}...\n(truncated)", &linter_output[..max_len]) } else { linter_output };
                let msg = tool_result.unwrap();
                tool_result = Ok(format!("{}\n\nAs IDE feedback, the following syntax/lint errors were detected after your edit:\n{}", msg, c_out));
            }
        }
    }
    
    tool_result
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
    let write_tools = ["write_file", "edit_file", "multi_edit_file", "delete_file", "create_file", "move_file", "rename_file"];
    let file_tools  = ["read_file", "write_file", "edit_file", "multi_edit_file", "delete_file", "list_directory", "create_file", "create_directory", "move_file", "rename_file"];
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

#[allow(dead_code)]
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
        "create_file", "create_directory", "delete_file", "move_file", "rename_file",
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

fn extract_tool_calls_from_prose(response: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();
    
    // Look for JSON-like structures in the prose
    // This handles cases where LLM outputs tool calls in prose format
    
    // Pattern 1: Look for ```json ... ``` blocks
    let json_block_regex = Regex::new(r#"```(?:json)?\s*(\{[\s\S]*?\})\s*```"#).unwrap();
    for cap in json_block_regex.captures_iter(response) {
        if let Some(json_str) = cap.get(1) {
            if let Ok(call) = serde_json::from_str::<ToolCall>(json_str.as_str()) {
                eprintln!("[EXTRACT_PROSE] Found tool call in JSON block: {}", call.tool);
                tool_calls.push(call);
            }
        }
    }
    
    // Pattern 2: Look for standalone JSON objects with "tool" field
    let json_obj_regex = Regex::new(r#"\{\s*\"tool\"\s*:\s*\"([^\"]+)\"[\s\S]*?\}"#).unwrap();
    for cap in json_obj_regex.captures_iter(response) {
        let json_str = cap.get(0).unwrap().as_str();
        if let Ok(call) = serde_json::from_str::<ToolCall>(json_str) {
            eprintln!("[EXTRACT_PROSE] Found tool call in prose JSON: {}", call.tool);
            tool_calls.push(call);
        }
    }
    
    // Pattern 3: Look for code blocks with file content that should be write_file
    let code_block_regex = Regex::new(r#"```(?:\w+)?\s*([\s\S]*?)\s*```"#).unwrap();
    for cap in code_block_regex.captures_iter(response) {
        let code_content = cap.get(1).unwrap().as_str();
        // Check if this looks like code that should be written to a file
        if code_content.len() > 100 && (code_content.contains("fn ") || code_content.contains("func ") || code_content.contains("def ") || code_content.contains("class ") || code_content.contains("import ") || code_content.contains("use ")) {
            // Try to extract filename from context
            let filename_patterns = [
                r#"filename:\s*([^\s\n]+)"#,
                r#"file:\s*([^\s\n]+)"#,
                r#"to\s+(?:file\s+)?([^\s\n]+)"#,
                r#"in\s+(?:file\s+)?([^\s\n]+)"#,
            ];
            
            let mut extracted_filename = None;
            for pattern in &filename_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if let Some(cap) = re.captures(response) {
                        extracted_filename = Some(cap.get(1).unwrap().as_str().trim_matches('`').trim());
                        break;
                    }
                }
            }
            
            if let Some(filename) = extracted_filename {
                let write_call = ToolCall {
                    tool: "write_file".to_string(),
                    args: serde_json::json!({
                        "path": filename,
                        "content": code_content
                    }),
                };
                eprintln!("[EXTRACT_PROSE] Generated write_file from code block for: {}", filename);
                tool_calls.push(write_call);
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
    eprintln!("[EXTRACT_PROSE] Total unique extracted tool calls: {}", unique.len());
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
    conversation_history: Option<Vec<ConversationTurn>>,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    vector_state: State<'_, Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>>,
    intel_state: State<'_, Arc<std::sync::Mutex<crate::commands::code_intelligence::CodeIntelligence>>>,
    learning_state: State<'_, Arc<std::sync::Mutex<crate::commands::learning::LearningSystem>>>,
    steering_state: State<'_, Arc<RwLock<SteeringSystem>>>,
    recovery_state: State<'_, Arc<std::sync::Mutex<crate::commands::error_recovery::ErrorRecoverySystem>>>,
    context_memory_state: State<'_, Arc<std::sync::Mutex<crate::commands::context_memory::ContextMemory>>>,
    hooks_state: State<'_, Arc<std::sync::Mutex<crate::commands::hooks::HooksManager>>>,
    graph_state: State<'_, Arc<std::sync::Mutex<crate::commands::graph::GraphService>>>,
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
        state.inner().clone(),
        context_memory_state.inner().clone(),
        hooks_state.inner().clone(),
        graph_state.inner().clone(),
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
