# Phase 1: Agent Loop Orchestration - Implementation Guide

## Overview
This is the CRITICAL foundation. Everything else depends on this working correctly.

## What Phase 1 Does

Transforms the agent from a "dumb loop" to an intelligent orchestrator:

```
Before (Current Tauri):
  LLM Call → Tool Execution → Repeat

After (Phase 1):
  Planning → Context Building → LLM Call → Tool Execution → Learning → Repeat
```

## Key Components to Implement

### 1. Planning Phase
```rust
// Create execution plan before running
let execution_plan = planner.create_plan(&PlanningContext {
    user_request: task.clone(),
    workspace_path: workspace_path.clone(),
    project_type: detect_project_type(&workspace_path).await,
    codebase_size: estimate_codebase_size(&workspace_path).await,
    available_tools: get_available_tools(),
    previous_plans: planner.get_plan_history().slice(-3),
});

// Send plan to UI
app_handle.emit_all("agent:plan", &execution_plan)?;
```

### 2. Context Building
```rust
// Build comprehensive project context
let mut project_context = String::new();

// Add execution plan context
if let Some(plan) = &execution_plan {
    project_context.push_str(&format!(
        "<execution_plan>\n\
         Objective: {}\n\
         Tasks: {} (Risk: {})\n\
         </execution_plan>\n",
        plan.objective,
        plan.tasks.len(),
        plan.risk_level
    ));
}

// Add learning recommendations
if let Some(recommendations) = learning_system.generate_recommendations(&task).await {
    project_context.push_str(&format!(
        "<learning_recommendations>\n{}\n</learning_recommendations>\n",
        recommendations.join("\n")
    ));
}

// Add code intelligence insights
if let Some(insights) = code_intelligence.analyze_workspace(&workspace_path).await {
    project_context.push_str(&format!(
        "<code_intelligence>\n{}\n</code_intelligence>\n",
        insights
    ));
}

// Add workspace manifest
if let Some(manifest) = build_workspace_manifest(&workspace_path).await {
    project_context.push_str(&format!(
        "<file_tree>\n{}\n</file_tree>\n",
        manifest
    ));
}

// Add active editor file
if let Some(file) = &active_file {
    project_context.push_str(&format!(
        "<active_editor_file>\n<path>{}</path>\n<content>\n{}\n</content>\n</active_editor_file>\n",
        file.get("path").unwrap_or(&"unknown".into()),
        file.get("content").unwrap_or(&"".into())
    ));
}

// Add git diff
if let Ok(diff) = get_git_diff(&workspace_path).await {
    project_context.push_str(&format!(
        "<git_diff>\n{}\n</git_diff>\n",
        diff
    ));
}

// Add terminal output
if let Some(terminal_output) = get_terminal_output().await {
    project_context.push_str(&format!(
        "<terminal_output>\n{}\n</terminal_output>\n",
        terminal_output
    ));
}

// Add steering instructions
if let Some(steering) = steering_system.build_context(&active_file).await {
    project_context.push_str(&steering);
}

// Add specs summary
if let Some(specs) = specs_system.get_summary().await {
    project_context.push_str(&specs);
}

// Add MCP tools
if let Some(mcp_tools) = mcp_service.build_tool_prompt().await {
    project_context.push_str(&mcp_tools);
}

// Add memory context
if let Some(memory) = memory_service.build_context().await {
    project_context.push_str(&memory);
}
```

### 3. Multi-turn Loop with Orchestration
```rust
let mut iteration = 0;
let max_iterations = 10;
let mut conversation_history = vec![
    ("system".to_string(), system_prompt),
    ("user".to_string(), format!("{}{}", task, project_context)),
];

while iteration < max_iterations {
    iteration += 1;
    
    // Call LLM with full context
    let response = call_llm(&conversation_history, &model).await?;
    
    // Parse tool calls
    let tool_calls = extract_tool_calls(&response);
    
    if tool_calls.is_empty() {
        // Agent is done
        return Ok(AgentLoopResponse {
            response: response.clone(),
            steps,
            tool_calls: all_tool_calls,
        });
    }
    
    // Execute tools sequentially
    let mut turn_results = Vec::new();
    for tool_call in &tool_calls {
        // 1. Check cache
        if let Some(cached) = tool_result_cache.get(&tool_call).await {
            turn_results.push(format!("[CACHED] {}", cached));
            continue;
        }
        
        // 2. Fire preToolUse hooks
        hooks_system.trigger("preToolUse", &tool_call.tool).await?;
        
        // 3. Execute tool
        let result = execute_tool(&tool_call, &workspace_path).await?;
        
        // 4. Record learning
        if let Some(learning) = &learning_system {
            learning.record_interaction(&task, &result, &tool_call.tool).await?;
        }
        
        // 5. Update execution plan
        if let Some(plan) = &mut execution_plan {
            plan.update_progress(&tool_call.tool);
        }
        
        // 6. Fire postToolUse hooks
        hooks_system.trigger("postToolUse", &tool_call.tool).await?;
        
        // 7. Cache result
        tool_result_cache.set(&tool_call, &result).await?;
        
        turn_results.push(result);
    }
    
    // Add to conversation history
    conversation_history.push(("assistant".to_string(), response));
    conversation_history.push(("user".to_string(), turn_results.join("\n\n")));
}
```

### 4. Knowledge Distillation
```rust
// After agent completes, extract patterns in background
tokio::spawn(async move {
    if let Err(e) = distill_knowledge(
        &conversation_history,
        &execution_plan,
        &model,
    ).await {
        eprintln!("[LEARNING] Failed to distill knowledge: {}", e);
    }
});

async fn distill_knowledge(
    history: &[(String, String)],
    plan: &Option<ExecutionPlan>,
    model: &str,
) -> Result<()> {
    // Extract patterns from interaction
    let patterns = extract_patterns(history);
    
    // Record successful strategies
    for pattern in patterns {
        learning_system.record_pattern(&pattern).await?;
    }
    
    // Update metrics
    learning_system.update_metrics(plan).await?;
    
    Ok(())
}
```

## File Structure

### Main Changes: `src-tauri/src/commands/agent_orchestrator.rs`

```rust
use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use tauri::Emitter;
use super::error_recovery::ErrorRecoverySystem;
use super::planner::WhizCodePlanner;
use super::learning::LearningSystem;
use super::context_memory::ContextMemory;
use super::code_intelligence::CodeIntelligence;
use super::hooks::HooksSystem;
use super::tool_result_cache::ToolResultCache;
use super::steering::SteeringSystem;
use super::planner::SpecsSystem;
use super::mcp_service::MCPService;
use super::memory::MemoryService;

pub struct AgentOrchestrator {
    max_iterations: u32,
    conversation_history: Vec<(String, String)>,
    app_handle: Option<tauri::AppHandle>,
    error_recovery: ErrorRecoverySystem,
    planner: WhizCodePlanner,
    learning_system: Option<LearningSystem>,
    context_memory: Option<ContextMemory>,
    code_intelligence: Option<CodeIntelligence>,
    hooks_system: Option<HooksSystem>,
    tool_result_cache: Option<ToolResultCache>,
    steering_system: Option<SteeringSystem>,
    specs_system: Option<SpecsSystem>,
    mcp_service: Option<MCPService>,
    memory_service: Option<MemoryService>,
}

impl AgentOrchestrator {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            max_iterations: 10,
            conversation_history: Vec::new(),
            app_handle,
            error_recovery: ErrorRecoverySystem::new(),
            planner: WhizCodePlanner::new(),
            learning_system: None,
            context_memory: None,
            code_intelligence: None,
            hooks_system: None,
            tool_result_cache: None,
            steering_system: None,
            specs_system: None,
            mcp_service: None,
            memory_service: None,
        }
    }
    
    pub async fn execute_task(
        &mut self,
        task: String,
        model: serde_json::Value,
        workspace_path: Option<String>,
        active_file: Option<serde_json::Value>,
    ) -> Result<AgentLoopResponse> {
        // Phase 1: Strategic Planning
        let execution_plan = self.create_execution_plan(&task, &workspace_path).await?;
        
        // Phase 2: Build Rich Context
        let project_context = self.build_project_context(
            &task,
            &workspace_path,
            &active_file,
            &execution_plan,
        ).await?;
        
        // Phase 3: Multi-turn Loop with Orchestration
        let response = self.run_agent_loop(
            &task,
            &model,
            &workspace_path,
            &active_file,
            &project_context,
            &execution_plan,
        ).await?;
        
        // Phase 4: Knowledge Distillation (background)
        self.distill_knowledge_background(&response).await;
        
        Ok(response)
    }
    
    async fn create_execution_plan(
        &self,
        task: &str,
        workspace_path: &Option<String>,
    ) -> Result<Option<ExecutionPlan>> {
        // Implementation here
        Ok(None)
    }
    
    async fn build_project_context(
        &self,
        task: &str,
        workspace_path: &Option<String>,
        active_file: &Option<serde_json::Value>,
        execution_plan: &Option<ExecutionPlan>,
    ) -> Result<String> {
        // Implementation here
        Ok(String::new())
    }
    
    async fn run_agent_loop(
        &mut self,
        task: &str,
        model: &serde_json::Value,
        workspace_path: &Option<String>,
        active_file: &Option<serde_json::Value>,
        project_context: &str,
        execution_plan: &Option<ExecutionPlan>,
    ) -> Result<AgentLoopResponse> {
        // Implementation here
        Ok(AgentLoopResponse {
            response: String::new(),
            steps: Vec::new(),
            tool_calls: Vec::new(),
        })
    }
    
    async fn distill_knowledge_background(&self, response: &AgentLoopResponse) {
        // Implementation here
    }
}
```

## Implementation Checklist

- [ ] Add planning phase
- [ ] Build workspace manifest
- [ ] Build execution plan context
- [ ] Build learning recommendations context
- [ ] Build code intelligence context
- [ ] Build active file context
- [ ] Build git diff context
- [ ] Build terminal output context
- [ ] Build steering context
- [ ] Build specs context
- [ ] Build MCP tools context
- [ ] Build memory context
- [ ] Implement multi-turn loop
- [ ] Integrate tool caching
- [ ] Integrate hooks system
- [ ] Integrate error recovery
- [ ] Integrate learning recording
- [ ] Integrate plan progress tracking
- [ ] Implement knowledge distillation
- [ ] Test with simple task
- [ ] Test with complex task
- [ ] Verify all context is injected
- [ ] Verify planning works
- [ ] Verify learning records
- [ ] Verify hooks fire

## Testing Strategy

### Test 1: Simple Task
```
User: "Create a hello world file"
Expected: Agent creates file without asking
```

### Test 2: Complex Task
```
User: "Create a React component with TypeScript"
Expected: Agent creates plan, executes steps, learns from interaction
```

### Test 3: Error Recovery
```
User: "Run a command that fails"
Expected: Agent recovers and suggests alternative
```

### Test 4: Learning
```
User: "Do task A, then task B"
Expected: Agent learns pattern and suggests it for similar tasks
```

## Success Criteria

- ✅ Agent creates execution plans
- ✅ Agent injects all 12+ types of context
- ✅ Agent runs multi-turn loop correctly
- ✅ Agent caches tool results
- ✅ Agent integrates hooks
- ✅ Agent records learning
- ✅ Agent distills knowledge
- ✅ Agent produces same results as Electron

## Next Steps

1. Implement the structure above
2. Test with simple task
3. Verify all context is injected
4. Move to Phase 2 (Tool Execution Enhancement)
