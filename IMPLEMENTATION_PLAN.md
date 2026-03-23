# Streaming Agent Flow Implementation Plan

## Current Status

The desired streaming flow is **NOT fully implemented**. Here's what exists and what's missing:

### ✅ What Exists:
1. **Streaming LLM Response** - `call_llm_streaming()` streams tokens from Ollama
2. **Streaming Tool Executor Module** - `streaming_tool_executor.rs` has incremental JSON parsing
3. **Sequential Executor Module** - `sequential_executor.rs` exists but unused
4. **Streaming Agent Flow Module** - `streaming_agent_flow.rs` (newly created)
5. **Tool Execution** - Tools are executed but in parallel groups, not sequentially

### ❌ What's Missing:
1. **Incremental JSON Parsing Integration** - Not integrated into main agent loop
2. **Immediate Tool Execution** - Tools wait for all to be identified before execution
3. **Sequential Execution** - Currently executes in parallel groups
4. **LLM Error Recovery** - No LLM feedback on tool failures
5. **Streaming Tool Queuing** - Tools aren't queued as they're identified

## Desired Flow

```
User sends prompt
    ↓
LLM starts streaming response
    ↓
[Streaming JSON Parser]
    ↓
Tool 1 identified → emit "identified" event → START EXECUTION IMMEDIATELY
    ↓
Tool 2 identified → emit "identified" event → ADD TO QUEUE
    ↓
Tool 3 identified → emit "identified" event → ADD TO QUEUE
    ↓
Tool 1 completes → emit "completed" event
    ↓
Tool 2 starts → emit "running" event
    ↓
Tool 2 fails → emit "failed" event
    ↓
Ask LLM: "Tool 2 failed with error: X. What should I do?"
    ↓
LLM responds: "Try with different args" or "Skip this tool"
    ↓
Execute recovery strategy
    ↓
Tool 2 completes → emit "completed" event
    ↓
Tool 3 starts → emit "running" event
    ↓
Tool 3 completes → emit "completed" event
    ↓
All tools done
```

## Implementation Steps

### Phase 1: Modify LLM Streaming to Use Incremental Parsing

**File**: `src-tauri/src/commands/agent_streaming.rs`

**Changes**:
1. Modify `call_llm_streaming()` to return a stream of chunks instead of full response
2. Create `stream_llm_with_tool_execution()` function that:
   - Streams LLM response
   - Parses JSON incrementally
   - Emits "identified" events as tools are parsed
   - Returns tool queue for execution

**Code Structure**:
```rust
async fn stream_llm_with_tool_execution(
    &self,
    messages: &[(String, String)],
    model: &str,
) -> Result<(Vec<ToolCall>, String)> {
    let mut flow = StreamingAgentFlow::new();
    let mut full_response = String::new();
    
    // Stream from LLM
    let client = reqwest::Client::new();
    let mut response = client.post("http://localhost:11434/api/chat")
        .json(&payload)
        .send()
        .await?;
    
    // Process chunks as they arrive
    while let Some(chunk) = response.chunk().await? {
        let text = String::from_utf8_lossy(&chunk);
        
        // Extract tokens and feed to parser
        for line in text.lines() {
            if let Ok(data) = serde_json::from_str::<Value>(line) {
                if let Some(token) = data.get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str()) {
                    
                    full_response.push_str(token);
                    
                    // Feed to incremental parser
                    let new_tools = flow.process_stream_chunk(token);
                    
                    // Emit "identified" for each new tool
                    for tool in new_tools {
                        self.emit_tool_identified(&tool).await;
                    }
                }
            }
        }
    }
    
    // Get all identified tools
    let mut tools = Vec::new();
    while let Some(tool) = flow.get_next_tool() {
        tools.push(tool);
    }
    
    Ok((tools, full_response))
}
```

### Phase 2: Implement Sequential Tool Execution

**File**: `src-tauri/src/commands/agent_streaming.rs`

**Changes**:
1. Create `execute_tools_sequentially()` function
2. Execute tools one by one
3. Emit "running" event when tool starts
4. Emit "completed" or "failed" event when tool finishes

**Code Structure**:
```rust
async fn execute_tools_sequentially(
    &self,
    tools: Vec<ToolCall>,
    workspace_path: &Option<String>,
) -> Result<Vec<(ToolCall, Result<String>)>> {
    let mut results = Vec::new();
    
    for tool in tools {
        // Emit "running" event
        self.emit_tool_running(&tool).await;
        
        // Execute tool
        let result = self.execute_single_tool(&tool, workspace_path).await;
        
        // Emit "completed" or "failed" event
        match &result {
            Ok(output) => {
                self.emit_tool_completed(&tool, output).await;
            }
            Err(error) => {
                self.emit_tool_failed(&tool, &error.to_string()).await;
                
                // Ask LLM for recovery strategy
                let recovery = self.ask_llm_for_recovery(
                    &tool,
                    &error.to_string(),
                ).await?;
                
                // Execute recovery strategy
                if recovery.should_retry {
                    // Retry the tool
                    let retry_result = self.execute_single_tool(&tool, workspace_path).await;
                    match &retry_result {
                        Ok(output) => {
                            self.emit_tool_completed(&tool, output).await;
                            results.push((tool, retry_result));
                        }
                        Err(e) => {
                            self.emit_tool_failed(&tool, &e.to_string()).await;
                            results.push((tool, retry_result));
                        }
                    }
                } else if recovery.should_skip {
                    // Skip this tool
                    results.push((tool, result));
                } else {
                    // Try alternative
                    let alt_result = self.execute_alternative_tool(&tool, workspace_path).await;
                    results.push((tool, alt_result));
                }
            }
        }
    }
    
    Ok(results)
}
```

### Phase 3: Implement LLM Error Recovery

**File**: `src-tauri/src/commands/agent_streaming.rs`

**Changes**:
1. Create `ask_llm_for_recovery()` function
2. Send error context to LLM
3. Parse recovery strategy from LLM response

**Code Structure**:
```rust
async fn ask_llm_for_recovery(
    &self,
    tool: &ToolCall,
    error: &str,
) -> Result<RecoveryStrategy> {
    let recovery_prompt = format!(
        "Tool '{}' failed with error: {}\n\
         Args were: {}\n\
         What should I do?\n\
         Options:\n\
         1. Retry with same args\n\
         2. Skip this tool\n\
         3. Try alternative approach\n\
         Respond with just the number (1, 2, or 3)",
        tool.tool,
        error,
        serde_json::to_string(&tool.args)?
    );
    
    // Call LLM with recovery prompt
    let (response, _) = self.call_llm_streaming(
        &[("user".to_string(), recovery_prompt)],
        "llama2"
    ).await?;
    
    // Parse response
    let strategy = if response.contains("1") {
        RecoveryStrategy {
            should_retry: true,
            should_skip: false,
        }
    } else if response.contains("2") {
        RecoveryStrategy {
            should_retry: false,
            should_skip: true,
        }
    } else {
        RecoveryStrategy {
            should_retry: false,
            should_skip: false,
        }
    };
    
    Ok(strategy)
}
```

### Phase 4: Integrate into Main Agent Loop

**File**: `src-tauri/src/commands/agent_streaming.rs`

**Changes**:
1. Modify `execute_task_streaming()` to use new flow
2. Replace current parallel execution with sequential execution
3. Use streaming JSON parsing

**Code Structure**:
```rust
async fn execute_task_streaming(
    &mut self,
    task: String,
    model: serde_json::Value,
    workspace_path: Option<String>,
    active_file: Option<serde_json::Value>,
    prior_history: Vec<ConversationTurn>,
    detected_shell: String,
    // ... other params
) -> Result<StreamingAgentResponse> {
    let mut all_steps = Vec::new();
    let mut all_tool_calls = Vec::new();
    let mut iteration = 0u32;
    
    loop {
        iteration += 1;
        
        // Build messages for LLM
        let mut turn_messages = vec![
            ("system".to_string(), system_prompt.clone()),
            ("user".to_string(), task.clone()),
        ];
        
        // Add prior history
        for turn in &prior_history {
            turn_messages.push((turn.role.clone(), turn.content.clone()));
        }
        
        // Stream LLM response and parse tools incrementally
        let (tools, response) = self.stream_llm_with_tool_execution(
            &turn_messages,
            &model_name
        ).await?;
        
        if tools.is_empty() {
            // No more tools, we're done
            break;
        }
        
        // Execute tools sequentially
        let results = self.execute_tools_sequentially(
            tools.clone(),
            &workspace_path
        ).await?;
        
        // Collect results for next iteration
        for (tool, result) in results {
            match result {
                Ok(output) => {
                    all_tool_calls.push(tool.clone());
                    // Add to history for next iteration
                }
                Err(e) => {
                    // Tool failed even after recovery
                    eprintln!("Tool {} failed: {}", tool.tool, e);
                }
            }
        }
        
        // Check if we should continue
        if tools.iter().any(|t| t.tool == "done") {
            break;
        }
    }
    
    Ok(StreamingAgentResponse {
        response: "Task completed".to_string(),
        steps: all_steps,
        tool_calls: all_tool_calls,
        total_tokens: 0,
        status: "completed".to_string(),
    })
}
```

## Testing

### Unit Tests
```bash
cargo test streaming_agent_flow
cargo test streaming_tool_executor
```

### Integration Tests
1. Test incremental JSON parsing with partial chunks
2. Test tool identification events
3. Test sequential execution
4. Test error recovery flow
5. Test LLM feedback on failures

## Benefits

1. **Real-time Feedback** - Users see tools as they're identified
2. **Faster Execution** - First tool starts immediately, not waiting for all
3. **Better Error Handling** - LLM can provide recovery strategies
4. **Transparent** - Users see exactly what's happening
5. **Resilient** - Failures don't stop the entire task

## Timeline

- **Phase 1**: 2-3 hours (streaming + incremental parsing)
- **Phase 2**: 2-3 hours (sequential execution)
- **Phase 3**: 1-2 hours (LLM error recovery)
- **Phase 4**: 1-2 hours (integration + testing)

**Total**: ~6-10 hours of development

## Notes

- The `streaming_agent_flow.rs` module has been created with the core logic
- The `streaming_tool_executor.rs` and `sequential_executor.rs` modules exist but need integration
- Current implementation uses parallel execution which needs to be replaced with sequential
- Error recovery is partially implemented in `failure_learning.rs` but needs LLM integration
