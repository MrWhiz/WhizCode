use serde::{Deserialize, Serialize};
use crate::error::{Result, ApiError};
use tauri::Emitter;
use super::error_recovery::ErrorRecoverySystem;
use super::planner::WhizCodePlanner;
use super::learning::LearningSystem;
use super::context_memory::ContextMemory;
use super::hooks::HooksManager;
use super::tool_result_cache::ToolResultCache;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentLoopResponse {
    pub response: String,
    pub steps: Vec<AgentStep>,
    pub tool_calls: Vec<ToolCall>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionPlan {
    pub id: String,
    pub objective: String,
    pub tasks: Vec<PlanTask>,
    pub parallel_groups: usize,
    pub risk_level: String,
    pub estimated_duration: u32,
    pub adaptations: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanTask {
    pub id: String,
    pub description: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub priority: u32,
    pub complexity: String,
}

#[allow(dead_code)]
pub struct AgentOrchestrator {
    max_iterations: u32,
    conversation_history: Vec<(String, String)>,
    app_handle: Option<tauri::AppHandle>,
    error_recovery: Arc<Mutex<ErrorRecoverySystem>>,
    planner: Arc<Mutex<WhizCodePlanner>>,
    learning_system: Arc<Mutex<LearningSystem>>,
    context_memory: Arc<Mutex<ContextMemory>>,
    hooks_manager: Arc<Mutex<HooksManager>>,
    tool_result_cache: Arc<Mutex<ToolResultCache>>,
}

impl AgentOrchestrator {
    #[allow(dead_code)]
    pub fn new(
        app_handle: Option<tauri::AppHandle>,
        error_recovery: Arc<Mutex<ErrorRecoverySystem>>,
        planner: Arc<Mutex<WhizCodePlanner>>,
        learning_system: Arc<Mutex<LearningSystem>>,
        context_memory: Arc<Mutex<ContextMemory>>,
        hooks_manager: Arc<Mutex<HooksManager>>,
        tool_result_cache: Arc<Mutex<ToolResultCache>>,
    ) -> Self {
        Self {
            max_iterations: 15,
            conversation_history: Vec::new(),
            app_handle,
            error_recovery,
            planner,
            learning_system,
            context_memory,
            hooks_manager,
            tool_result_cache,
        }
    }

    #[allow(dead_code)]
    pub async fn execute_task(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
    ) -> Result<AgentLoopResponse> {
        eprintln!("[PHASE_1] Starting Agent Loop Orchestration");
        
        // ── PHASE 1: STRATEGIC PLANNING ──────────────────────────────────
        if let Some(app) = &self.app_handle {
            let _ = app.emit("agent:step", AgentStep {
                iteration: 0, tool: "planning".into(), status: "running".into(),
                summary: "Creating execution plan...".into(), result: None, logs: None,
            });
        }
        
        let execution_plan = self.create_execution_plan(&task, &workspace_path).await?;
        if let Some(app) = &self.app_handle { 
            let _ = app.emit("agent:plan", &execution_plan); 
            let _ = app.emit("agent:step", AgentStep {
                iteration: 0, tool: "planning".into(), status: "done".into(),
                summary: format!("Plan ready with {} tasks", execution_plan.tasks.len()), result: None, logs: None,
            });
        }
        
        // ── PHASE 2: BUILD RICH CONTEXT ──────────────────────────────────
        let project_context = self.build_project_context(&task, &workspace_path, &active_file, &execution_plan).await?;
        
        // ── PHASE 3: MULTI-TURN LOOP ────────────────────────────────────
        let response = self.run_agent_loop(&task, &model, &workspace_path, &active_file, &project_context, &execution_plan).await?;
        
        // ── PHASE 4: KNOWLEDGE DISTILLATION ────────────────────────────
        self.distill_knowledge_background(&response, &execution_plan).await;
        
        Ok(response)
    }

    async fn create_execution_plan(&self, task: &str, workspace_path: &Option<String>) -> Result<ExecutionPlan> {
        let context = super::planner::PlanningContext {
            user_request: task.to_string(), workspace_path: workspace_path.clone().unwrap_or_default(),
            active_file: None, recent_context: None,
        };
        if let Ok(p) = super::planner::WhizCodePlanner::create_plan(&context) {
            return Ok(ExecutionPlan {
                id: p.id, objective: p.objective, risk_level: p.risk_level, estimated_duration: p.estimated_duration,
                parallel_groups: p.parallel_groups.len(),
                adaptations: vec![],
                tasks: p.tasks.into_iter().map(|t| PlanTask { 
                    id: t.id, 
                    description: t.description, 
                    task_type: t.task_type, 
                    priority: t.priority,
                    complexity: "medium".into() // Default complexity
                }).collect(),
            });
        }
        Ok(ExecutionPlan { 
            id: "p1".into(), objective: task.into(), risk_level: "low".into(), estimated_duration: 300, 
            parallel_groups: 0, adaptations: vec![], tasks: vec![] 
        })
    }

    async fn build_project_context(&self, _task: &str, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>, execution_plan: &ExecutionPlan) -> Result<String> {
        let mut ctx = format!("\n<plan>\nObjective: {}\nTasks:\n", execution_plan.objective);
        for task in &execution_plan.tasks {
            ctx.push_str(&format!("- [ ] {}: {}\n", task.id, task.description));
        }
        ctx.push_str("</plan>\n");
        
        if let Some(ws) = workspace_path { ctx.push_str(&format!("\n<workspace>\nPath: {}\n</workspace>\n", ws)); }
        if let Some(f) = active_file { if let Some(p) = f.get("path").and_then(|p| p.as_str()) { ctx.push_str(&format!("\n<active_file>\nPath: {}\n</active_file>\n", p)); } }
        Ok(ctx)
    }

    async fn run_agent_loop(&mut self, task: &str, model: &serde_json::Value, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>, project_context: &str, _execution_plan: &ExecutionPlan) -> Result<AgentLoopResponse> {
        let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");
        let mut steps = Vec::new();
        let mut iteration = 0u32;
        let mut last_response = String::new();
        let mut all_calls = Vec::new();
        
        let system_prompt = self.get_system_prompt(workspace_path, active_file);
        let mut msgs = vec![("system".to_string(), system_prompt), ("user".to_string(), format!("{}{}", task, project_context))];
        
        while iteration < self.max_iterations {
            iteration += 1;
            if let Some(app) = &self.app_handle { let _ = app.emit("agent:step", AgentStep { iteration, tool: "llm".into(), status: "running".into(), summary: format!("Call LLM {}", iteration), result: None, logs: None }); }
            let resp = self.call_llm(&msgs, model_name).await?;
            last_response = resp.clone();
            let calls = extract_tool_calls(&resp);
            if calls.is_empty() { steps.push(AgentStep { iteration, tool: "reasoning".into(), status: "done".into(), summary: "Done".into(), result: Some(resp), logs: None }); break; }
            
            for call in &calls {
                let res = self.execute_tool(call, workspace_path).await;
                let step = AgentStep { iteration, tool: call.tool.clone(), status: if res.is_ok() { "done".into() } else { "failed".into() }, summary: format!("Tool {}", call.tool), result: res.as_ref().ok().cloned(), logs: None };
                if let Some(app) = &self.app_handle { let _ = app.emit("agent:step", &step); }
                steps.push(step);
                all_calls.push(call.clone());
                msgs.push(("assistant".to_string(), resp.clone()));
                msgs.push(("user".to_string(), format!("Result: {}", res.unwrap_or_else(|e| format!("Error: {}", e)))));
            }
        }
        Ok(AgentLoopResponse { response: last_response, steps, tool_calls: all_calls })
    }

    async fn distill_knowledge_background(&self, response: &AgentLoopResponse, _plan: &ExecutionPlan) {
        if let Ok(l) = self.learning_system.lock() {
            l.record_interaction(super::learning::InteractionRecord {
                timestamp: chrono::Local::now().timestamp(), user_request: "Agent".into(), agent_response: response.response.clone(),
                tools_used: response.steps.iter().map(|s| s.tool.clone()).collect(), success: true, duration_ms: 0,
            });
        }
    }

    async fn execute_tool(&mut self, call: &ToolCall, workspace_path: &Option<String>) -> Result<String> {
        let key = format!("{}_{}", call.tool, serde_json::to_string(&call.args).unwrap_or_default());
        if let Ok(c) = self.tool_result_cache.lock() { if let Some(r) = c.get(&key).ok().flatten() { if let Some(s) = r.as_str() { return Ok(s.to_string()); } } }
        if let Ok(h) = self.hooks_manager.lock() { let _ = h.trigger_tool_event("preToolUse", &call.tool); }
        
        let res = match call.tool.as_str() {
            "read_file" => {
                let p = call.args.get("path").and_then(|p| p.as_str()).ok_or("No path")?;
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = workspace_path { full = std::path::Path::new(ws).join(full); } }
                tokio::fs::read_to_string(&full).await.map_err(|e| format!("Read failed {}: {}", p, e).into())
            }
            "write_file" => {
                let p = call.args.get("path").and_then(|p| p.as_str()).ok_or("No path")?;
                let c = call.args.get("content").and_then(|c| c.as_str()).ok_or("No content")?;
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = workspace_path { full = std::path::Path::new(ws).join(full); } }
                if let Some(par) = full.parent() { let _ = tokio::fs::create_dir_all(par).await; }
                tokio::fs::write(&full, c).await.map(|_| format!("Wrote {}", p)).map_err(|e| format!("Write failed: {}", e).into())
            }
            "list_directory" => {
                let p = call.args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = workspace_path { full = std::path::Path::new(ws).join(full); } }
                let mut entries = Vec::new();
                let mut dir = tokio::fs::read_dir(&full).await.map_err(|e| format!("LS failed: {}", e))?;
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                    entries.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
                }
                Ok(entries.join("\n"))
            }
            "edit_file" => {
                let p = call.args.get("path").and_then(|p| p.as_str()).ok_or("No path")?;
                let c = call.args.get("content").and_then(|c| c.as_str()).ok_or("No content")?;
                
                let mut full = std::path::PathBuf::from(p);
                if !full.is_absolute() { if let Some(ws) = workspace_path { full = std::path::Path::new(ws).join(full); } }

                let args = super::advanced_tools::EditFileArgs {
                    path: full.to_string_lossy().to_string(), 
                    start_line: call.args.get("start_line").and_then(|l| l.as_u64()).map(|l| l as u32),
                    end_line: call.args.get("end_line").and_then(|l| l.as_u64()).map(|l| l as u32), 
                    content: c.to_string(),
                };
                super::advanced_tools::AdvancedToolExecutor::edit_file(&args).await.map(|r| r.output)
            }
            "run_command" => {
                let cmd_str = call.args.get("command").and_then(|c| c.as_str()).ok_or("No command")?;
                let (shell, sargs) = if cfg!(windows) { ("cmd", vec!["/C", cmd_str]) } else { ("sh", vec!["-c", cmd_str]) };
                let mut cmd = tokio::process::Command::new(shell);
                cmd.args(&sargs);
                if let Some(ws) = workspace_path { cmd.current_dir(ws); }
                let out = tokio::time::timeout(std::time::Duration::from_secs(60), cmd.output()).await
                    .map_err(|e| ApiError::from(e))??;
                Ok(format!("Stdout:\n{}\nStderr:\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
            }
            _ => Err(format!("Unknown tool: {}", call.tool).into()),
        };
        
        if let Ok(r) = &res { 
            if let Ok(c) = self.tool_result_cache.lock() { 
                let _ = c.set(key, serde_json::Value::String(r.clone()), None); 
            } 
        }
        res
    }

    async fn call_llm(&self, messages: &[(String, String)], model: &str) -> Result<String> {
        let mut prompt = String::new();
        for (r, c) in messages { if r == "system" { prompt.push_str(&format!("{}\n\n", c)); } }
        for (r, c) in messages { if r != "system" { prompt.push_str(&format!("[{}]\n{}\n\n", r.to_uppercase(), c)); } }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ApiError::from(e))?;
        let res = client.post("http://localhost:11434/api/generate").json(&serde_json::json!({ "model": model, "prompt": prompt, "stream": false })).send().await
            .map_err(|e| ApiError::from(e))?;
        let data: serde_json::Value = res.json().await.map_err(|e| ApiError::from(e))?;
        Ok(data.get("response").and_then(|r| r.as_str()).unwrap_or("Error").to_string())
    }

    fn get_system_prompt(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
        let mut p = crate::commands::prompts::KIRO_SYSTEM_PROMPT.to_string();
        if let Some(ws) = workspace_path { p = p.replace("{{workspace_path}}", ws); }
        if let Some(f) = active_file { if let Some(path) = f.get("path").and_then(|p| p.as_str()) { p = p.replace("{{active_file}}", path); } }
        p
    }
}

fn extract_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let re = regex::Regex::new(r"<tool_call>[\s\S]*?<tool_name>(.*?)</tool_name>[\s\S]*?<tool_args>(.*?)</tool_args>[\s\S]*?</tool_call>").unwrap();
    for cap in re.captures_iter(response) {
        if let (Some(n), Some(a)) = (cap.get(1), cap.get(2)) {
            if let Ok(args) = serde_json::from_str::<serde_json::Value>(a.as_str()) { calls.push(ToolCall { tool: n.as_str().to_string(), args }); }
        }
    }
    calls
}

#[tauri::command]
pub async fn execute_agent_loop(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    app_handle: tauri::AppHandle,
    error_recovery: tauri::State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
    planner: tauri::State<'_, Arc<Mutex<WhizCodePlanner>>>,
    learning_system: tauri::State<'_, Arc<Mutex<LearningSystem>>>,
    context_memory: tauri::State<'_, Arc<Mutex<ContextMemory>>>,
    hooks_manager: tauri::State<'_, Arc<Mutex<HooksManager>>>,
    tool_result_cache: tauri::State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<AgentLoopResponse> {
    let mut orchestrator = AgentOrchestrator::new(
        Some(app_handle), error_recovery.inner().clone(), planner.inner().clone(),
        learning_system.inner().clone(), context_memory.inner().clone(),
        hooks_manager.inner().clone(), tool_result_cache.inner().clone(),
    );
    orchestrator.execute_task(task, model, workspace_path, active_file).await
}
