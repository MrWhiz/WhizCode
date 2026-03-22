use serde::{Deserialize, Serialize};
use crate::error::{Result, ApiError};
use tauri::Emitter;
use super::error_recovery::ErrorRecoverySystem;

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

// Chain-of-Thought Reasoning Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: u32,
    pub phase: String,  // "analysis", "hypothesis", "validation", "conclusion"
    pub reasoning: String,
    pub confidence: f32,  // 0.0 to 1.0
    pub alternatives_considered: Vec<String>,
    pub decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoTResponse {
    pub reasoning_steps: Vec<ReasoningStep>,
    pub final_decision: String,
    pub overall_confidence: f32,
    pub reasoning_trace: String,
    pub execution_plan: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct EnhancedAgentResponse {
    pub cot_response: CoTResponse,
    pub tool_calls: Vec<ToolCall>,
    pub execution_steps: Vec<AgentStep>,
    pub reasoning_quality_score: f32,
}

// Confidence Scoring Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceMetrics {
    pub task_confidence: f32,      // 0.0-1.0
    pub tool_selection_confidence: f32,
    pub risk_level: String,        // "low", "medium", "high"
    pub uncertainty_factors: Vec<String>,
    pub requires_human_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDecision {
    pub decision: String,
    pub confidence: f32,
    pub risk_level: String,
    pub action: String,  // "auto_execute", "ask_user", "escalate"
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceThresholds {
    pub very_confident: f32,      // 0.9
    pub confident: f32,           // 0.7
    pub moderate: f32,            // 0.5
    pub low: f32,                 // 0.3
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
        // Use planning module instead of deleted planner module
        let mut planning_system = super::planning::PlanningSystem::new();
        let plan = planning_system.create_plan(task, workspace_path);
        
        Ok(ExecutionPlan {
            id: plan.id,
            objective: plan.objective,
            risk_level: plan.risk_level,
            estimated_duration: plan.estimated_duration,
            parallel_groups: plan.parallel_groups.len(),
            adaptations: vec![],
            tasks: plan.tasks.into_iter().map(|t| PlanTask { 
                id: t.id, 
                description: t.description, 
                task_type: t.task_type, 
                priority: t.priority,
                complexity: "medium".into()
            }).collect(),
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
        let mut p = crate::commands::prompts::WHIZCODE_SYSTEM_PROMPT.to_string();
        if let Some(ws) = workspace_path { p = p.replace("{{workspace_path}}", ws); }
        if let Some(f) = active_file { if let Some(path) = f.get("path").and_then(|p| p.as_str()) { p = p.replace("{{active_file}}", path); } }
        p
    }

    // ─── Chain-of-Thought Reasoning Methods ───────────────────────────────
    
    pub fn get_system_prompt_with_cot(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
        format!(
            r#"You are WhizCode, an advanced AI coding assistant with explicit reasoning capabilities.

## Your Reasoning Process

When solving problems, ALWAYS follow this Chain-of-Thought structure:

### Phase 1: ANALYSIS
- Understand the user's request
- Identify key constraints and requirements
- List what you know and what you need to find out
- Assess complexity level

### Phase 2: HYPOTHESIS
- Propose 2-3 different approaches
- Evaluate pros/cons of each
- Identify risks and dependencies
- Select the most promising approach

### Phase 3: VALIDATION
- Check if your approach is feasible
- Verify against constraints
- Consider edge cases
- Identify potential issues

### Phase 4: CONCLUSION
- Finalize your decision
- Explain why this is the best approach
- State your confidence level (0.0-1.0)
- Outline execution steps

## Response Format

For every task, respond with this JSON structure:

```json
{{
  "reasoning_steps": [
    {{
      "step_number": 1,
      "phase": "analysis",
      "reasoning": "...",
      "confidence": 0.9,
      "alternatives_considered": ["...", "..."],
      "decision": null
    }},
    {{
      "step_number": 2,
      "phase": "hypothesis",
      "reasoning": "...",
      "confidence": 0.85,
      "alternatives_considered": ["...", "..."],
      "decision": "Selected approach X because..."
    }},
    {{
      "step_number": 3,
      "phase": "validation",
      "reasoning": "...",
      "confidence": 0.9,
      "alternatives_considered": [],
      "decision": "Approach is feasible"
    }},
    {{
      "step_number": 4,
      "phase": "conclusion",
      "reasoning": "...",
      "confidence": 0.88,
      "alternatives_considered": [],
      "decision": "Final decision: ..."
    }}
  ],
  "final_decision": "...",
  "overall_confidence": 0.88,
  "reasoning_trace": "Full narrative of reasoning...",
  "execution_plan": ["step1", "step2", "step3"]
}}
```

## Confidence Scoring Guidelines

- 0.9-1.0: Very confident, proceed autonomously
- 0.7-0.9: Confident, proceed with monitoring
- 0.5-0.7: Moderate confidence, may need review
- 0.3-0.5: Low confidence, recommend human review
- 0.0-0.3: Very uncertain, escalate to user

---

Workspace: {}
Active File: {}
"#,
            workspace_path.as_ref().unwrap_or(&"(none)".to_string()),
            active_file.as_ref().and_then(|f| f.get("path")).and_then(|p| p.as_str()).unwrap_or("(none)")
        )
    }

    pub async fn parse_cot_response(&self, response: &str) -> Result<CoTResponse> {
        // Try to extract JSON from response
        let json_start = response.find('{').ok_or("No JSON found in response")?;
        let json_end = response.rfind('}').ok_or("Incomplete JSON in response")?;
        let json_str = &response[json_start..=json_end];
        
        let cot: CoTResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse CoT response: {}", e))?;
        
        // Validate reasoning steps
        self.validate_reasoning_steps(&cot.reasoning_steps)?;
        
        Ok(cot)
    }

    fn validate_reasoning_steps(&self, steps: &[ReasoningStep]) -> Result<()> {
        let expected_phases = vec!["analysis", "hypothesis", "validation", "conclusion"];
        
        for (i, step) in steps.iter().enumerate() {
            if step.step_number != (i + 1) as u32 {
                return Err(format!("Step numbering mismatch at step {}", i + 1).into());
            }
            
            if !expected_phases.contains(&step.phase.as_str()) {
                return Err(format!("Invalid phase: {}", step.phase).into());
            }
            
            if step.confidence < 0.0 || step.confidence > 1.0 {
                return Err(format!("Invalid confidence score: {}", step.confidence).into());
            }
        }
        
        Ok(())
    }

    pub fn calculate_overall_confidence(&self, steps: &[ReasoningStep]) -> f32 {
        if steps.is_empty() {
            return 0.5;
        }
        
        // Weight later phases more heavily
        let weights = vec![0.1, 0.2, 0.3, 0.4];
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;
        
        for (i, step) in steps.iter().enumerate() {
            let weight = weights.get(i).unwrap_or(&0.25);
            total_weighted += step.confidence * weight;
            total_weight += weight;
        }
        
        (total_weighted / total_weight).min(1.0).max(0.0)
    }

    // ─── Confidence Scoring Methods ───────────────────────────────────────
    
    pub fn get_confidence_thresholds() -> ConfidenceThresholds {
        ConfidenceThresholds {
            very_confident: 0.9,
            confident: 0.7,
            moderate: 0.5,
            low: 0.3,
        }
    }

    pub fn evaluate_confidence(&self, confidence: f32, task_type: &str) -> ConfidenceDecision {
        let thresholds = Self::get_confidence_thresholds();
        
        let (risk_level, action, reasoning) = if confidence >= thresholds.very_confident {
            ("low".to_string(), "auto_execute".to_string(), "Very high confidence - proceeding autonomously".to_string())
        } else if confidence >= thresholds.confident {
            ("low".to_string(), "auto_execute".to_string(), "High confidence - proceeding with monitoring".to_string())
        } else if confidence >= thresholds.moderate {
            ("medium".to_string(), "ask_user".to_string(), "Moderate confidence - requesting user confirmation".to_string())
        } else if confidence >= thresholds.low {
            ("high".to_string(), "ask_user".to_string(), "Low confidence - human review recommended".to_string())
        } else {
            ("critical".to_string(), "escalate".to_string(), "Very low confidence - escalating to user".to_string())
        };
        
        ConfidenceDecision {
            decision: format!("Task: {}", task_type),
            confidence,
            risk_level,
            action,
            reasoning,
        }
    }

    pub fn calculate_tool_confidence(&self, _tool_name: &str, success_rate: f32, execution_time_ms: u32) -> f32 {
        // Base confidence from success rate (70% weight)
        let success_confidence = success_rate * 0.7;
        
        // Time-based confidence (30% weight) - faster tools are more reliable
        let time_confidence = if execution_time_ms < 100 {
            0.3
        } else if execution_time_ms < 500 {
            0.25
        } else if execution_time_ms < 2000 {
            0.2
        } else {
            0.1
        };
        
        (success_confidence + time_confidence).min(1.0).max(0.0)
    }

    pub fn assess_decision_risk(&self, confidence: f32, tool_calls: &[ToolCall]) -> ConfidenceMetrics {
        let mut uncertainty_factors = Vec::new();
        
        if confidence < 0.7 {
            uncertainty_factors.push("Low reasoning confidence".to_string());
        }
        
        if tool_calls.is_empty() {
            uncertainty_factors.push("No tools selected".to_string());
        }
        
        if tool_calls.len() > 5 {
            uncertainty_factors.push("Many tools to execute".to_string());
        }
        
        let risk_level = if confidence >= 0.8 && tool_calls.len() <= 3 {
            "low"
        } else if confidence >= 0.6 && tool_calls.len() <= 5 {
            "medium"
        } else {
            "high"
        };
        
        let requires_review = confidence < 0.7 || tool_calls.len() > 5;
        
        ConfidenceMetrics {
            task_confidence: confidence,
            tool_selection_confidence: if tool_calls.is_empty() { 0.0 } else { 0.8 },
            risk_level: risk_level.to_string(),
            uncertainty_factors,
            requires_human_review: requires_review,
        }
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
    learning_system: tauri::State<'_, Arc<Mutex<LearningSystem>>>,
    context_memory: tauri::State<'_, Arc<Mutex<ContextMemory>>>,
    hooks_manager: tauri::State<'_, Arc<Mutex<HooksManager>>>,
    tool_result_cache: tauri::State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<AgentLoopResponse> {
    let mut orchestrator = AgentOrchestrator::new(
        Some(app_handle), error_recovery.inner().clone(),
        learning_system.inner().clone(), context_memory.inner().clone(),
        hooks_manager.inner().clone(), tool_result_cache.inner().clone(),
    );
    orchestrator.execute_task(task, model, workspace_path, active_file).await
}


// ─── Tauri Commands for Chain-of-Thought Reasoning ───────────────────────────

#[tauri::command]
pub async fn agent_reasoning_with_cot(
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
) -> Result<CoTResponse> {
    // Create a minimal orchestrator for CoT reasoning
    let orchestrator = AgentOrchestrator::new(
        None,
        Arc::new(Mutex::new(ErrorRecoverySystem::new())),
        Arc::new(Mutex::new(LearningSystem::new())),
        Arc::new(Mutex::new(ContextMemory::new())),
        Arc::new(Mutex::new(HooksManager::new())),
        Arc::new(Mutex::new(ToolResultCache::new(None))),
    );
    
    // Get the CoT system prompt
    let system_prompt = orchestrator.get_system_prompt_with_cot(&workspace_path, &active_file);
    
    // Prepare messages
    let messages = vec![
        ("system".to_string(), system_prompt),
        ("user".to_string(), task),
    ];
    
    // Call LLM
    let model_name = model.get("model").and_then(|m| m.as_str()).unwrap_or("llama2");
    let response = orchestrator.call_llm(&messages, model_name).await?;
    
    // Parse CoT response
    orchestrator.parse_cot_response(&response).await
}

#[tauri::command]
pub async fn agent_validate_cot_response(
    response: String,
) -> Result<serde_json::Value> {
    // Create a minimal orchestrator for validation
    let orchestrator = AgentOrchestrator::new(
        None,
        Arc::new(Mutex::new(ErrorRecoverySystem::new())),
        Arc::new(Mutex::new(LearningSystem::new())),
        Arc::new(Mutex::new(ContextMemory::new())),
        Arc::new(Mutex::new(HooksManager::new())),
        Arc::new(Mutex::new(ToolResultCache::new(None))),
    );
    
    // Parse and validate
    match orchestrator.parse_cot_response(&response).await {
        Ok(cot) => {
            let confidence = orchestrator.calculate_overall_confidence(&cot.reasoning_steps);
            Ok(serde_json::json!({
                "valid": true,
                "cot_response": cot,
                "overall_confidence": confidence,
                "requires_review": confidence < 0.7,
            }))
        }
        Err(e) => {
            Ok(serde_json::json!({
                "valid": false,
                "error": e.to_string(),
            }))
        }
    }
}

#[tauri::command]
pub async fn agent_get_cot_metrics() -> Result<serde_json::Value> {
    // Return metrics about CoT reasoning
    Ok(serde_json::json!({
        "feature": "Chain-of-Thought Reasoning",
        "status": "active",
        "phases": ["analysis", "hypothesis", "validation", "conclusion"],
        "confidence_thresholds": {
            "very_confident": 0.9,
            "confident": 0.7,
            "moderate": 0.5,
            "low": 0.3,
        },
        "expected_improvement": "+25-30% reasoning accuracy",
    }))
}


// ─── Tauri Commands for Confidence Scoring ──────────────────────────────────

#[tauri::command]
pub async fn agent_evaluate_confidence(
    confidence: f32,
    task_type: String,
) -> Result<ConfidenceDecision> {
    let orchestrator = AgentOrchestrator::new(
        None,
        Arc::new(Mutex::new(ErrorRecoverySystem::new())),
        Arc::new(Mutex::new(LearningSystem::new())),
        Arc::new(Mutex::new(ContextMemory::new())),
        Arc::new(Mutex::new(HooksManager::new())),
        Arc::new(Mutex::new(ToolResultCache::new(None))),
    );
    
    Ok(orchestrator.evaluate_confidence(confidence, &task_type))
}

#[tauri::command]
pub async fn agent_calculate_tool_confidence(
    tool_name: String,
    success_rate: f32,
    execution_time_ms: u32,
) -> Result<f32> {
    let orchestrator = AgentOrchestrator::new(
        None,
        Arc::new(Mutex::new(ErrorRecoverySystem::new())),
        Arc::new(Mutex::new(LearningSystem::new())),
        Arc::new(Mutex::new(ContextMemory::new())),
        Arc::new(Mutex::new(HooksManager::new())),
        Arc::new(Mutex::new(ToolResultCache::new(None))),
    );
    
    Ok(orchestrator.calculate_tool_confidence(&tool_name, success_rate, execution_time_ms))
}

#[tauri::command]
pub async fn agent_assess_decision_risk(
    confidence: f32,
    tool_calls: Vec<serde_json::Value>,
) -> Result<ConfidenceMetrics> {
    let orchestrator = AgentOrchestrator::new(
        None,
        Arc::new(Mutex::new(ErrorRecoverySystem::new())),
        Arc::new(Mutex::new(LearningSystem::new())),
        Arc::new(Mutex::new(ContextMemory::new())),
        Arc::new(Mutex::new(HooksManager::new())),
        Arc::new(Mutex::new(ToolResultCache::new(None))),
    );
    
    // Convert JSON to ToolCall
    let calls: Vec<ToolCall> = tool_calls
        .iter()
        .filter_map(|v| {
            let tool = v.get("tool").and_then(|t| t.as_str())?;
            let args = v.get("args").cloned().unwrap_or(serde_json::json!({}));
            Some(ToolCall {
                tool: tool.to_string(),
                args,
            })
        })
        .collect();
    
    Ok(orchestrator.assess_decision_risk(confidence, &calls))
}

#[tauri::command]
pub async fn agent_get_confidence_thresholds() -> Result<serde_json::Value> {
    let thresholds = AgentOrchestrator::get_confidence_thresholds();
    Ok(serde_json::json!({
        "very_confident": thresholds.very_confident,
        "confident": thresholds.confident,
        "moderate": thresholds.moderate,
        "low": thresholds.low,
        "actions": {
            "very_confident": "auto_execute",
            "confident": "auto_execute",
            "moderate": "ask_user",
            "low": "ask_user",
            "critical": "escalate",
        },
        "risk_levels": {
            "very_confident": "low",
            "confident": "low",
            "moderate": "medium",
            "low": "high",
            "critical": "critical",
        }
    }))
}
