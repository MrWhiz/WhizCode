# Streaming Tool Execution Architecture

## Overview

This document describes the new streaming tool execution flow that enables:
1. **Streaming JSON parsing** - Parse tool calls as they arrive from the LLM
2. **Immediate identification** - Emit "identified" status as soon as each tool call is parsed
3. **Sequential execution** - Execute tools one by one in order
4. **Error recovery** - Get LLM feedback on failures and retry or skip as needed
5. **Continuous streaming** - Keep receiving LLM response while executing tools

## Flow Diagram

```
LLM Response Stream
        ↓
[Streaming JSON Parser]
        ↓
Tool 1 identified → emit "identified" event
        ↓
Tool 2 identified → emit "identified" event
        ↓
Tool 3 identified → emit "identified" event
        ↓
[All tools identified signal]
        ↓
Execute Tool 1 sequentially
        ├─ Success → emit "completed" event
        └─ Failure → ask LLM for recovery strategy
                    ├─ Retry → execute again
                    ├─ Skip → move to next
                    └─ Alternative → skip for now
        ↓
Execute Tool 2 sequentially
        ├─ Success → emit "completed" event
        └─ Failure → ask LLM for recovery strategy
        ↓
Execute Tool 3 sequentially
        ├─ Success → emit "completed" event
        └─ Failure → ask LLM for recovery strategy
        ↓
All tools completed
```

## Components

### 1. StreamingToolExecutor (`streaming_tool_executor.rs`)

Handles streaming JSON parsing and tool queuing.

**Key Features:**
- `parse_streaming_json()` - Parse JSON objects from streaming response
- `get_next_tool()` - Get the next tool to execute
- `mark_tool_completed()` - Mark tool as done and emit event
- `mark_tool_failed()` - Mark tool as failed and emit event
- `IncrementalJsonParser` - Stateful JSON parser for streaming data

**Events Emitted:**
- `ToolIdentified` - When a tool call is parsed from the stream
- `ToolStarted` - When tool execution begins
- `ToolCompleted` - When tool execution succeeds
- `ToolFailed` - When tool execution fails
- `AllToolsIdentified` - When all tools from current response are parsed

### 2. SequentialToolExecutor (`sequential_executor.rs`)

Orchestrates sequential tool execution with error recovery.

**Key Features:**
- `execute_from_stream()` - Main execution loop
- `set_llm_callback()` - Set callback for LLM error recovery
- Automatic retry on failure if LLM suggests it
- Skip failed tools if LLM suggests it
- Continue with next tool after recovery

**Error Recovery Flow:**
1. Tool fails with error
2. Send error context to LLM via callback
3. LLM responds with recovery strategy
4. Execute strategy (retry, skip, or alternative)
5. Continue with next tool

### 3. IncrementalJsonParser

Stateful parser that extracts complete JSON objects from streaming data.

**Algorithm:**
1. Buffer incoming data
2. Find JSON object start (`{`)
3. Count braces to find object end (`}`)
4. Extract complete object
5. Return parsed JSON
6. Repeat with remaining buffer

## Integration Points

### Frontend (React)

The frontend receives events via Tauri:

```typescript
// Listen for tool identification
agent.events.onToolIdentified((event) => {
  // Show tool in UI with "identified" status
  // Start visual indicator
})

// Listen for tool completion
agent.events.onToolCompleted((event) => {
  // Update tool status to "completed"
  // Show result
})

// Listen for tool failure
agent.events.onToolFailed((event) => {
  // Update tool status to "failed"
  // Show error message
})
```

### Backend (Rust)

Integration in `agent_streaming.rs`:

```rust
// Create executor
let mut executor = SequentialToolExecutor::new(event_tx);

// Set LLM callback for error recovery
executor.set_llm_callback(|error_context| {
    // Call LLM with error context
    // Return recovery strategy
});

// Parse streaming response and execute tools
let results = executor.execute_from_stream(
    &llm_response,
    |tool| {
        // Execute tool and return result
    }
).await?;
```

## Benefits

1. **Real-time Feedback** - Users see tool calls as they're identified
2. **Better Error Handling** - LLM can provide recovery strategies
3. **Sequential Execution** - Tools execute in order, dependencies respected
4. **Resilient** - Failures don't stop the entire task
5. **Transparent** - Users see exactly what's happening

## Example Scenario

```
User: "Read file.txt and write the content to output.txt"

LLM Response (streaming):
{"tool": "read_file", "args": {"path": "file.txt"}}
{"tool": "write_file", "args": {"path": "output.txt", "content": "..."}}

Frontend Timeline:
1. Tool 1 identified: read_file
2. Tool 2 identified: write_file
3. All tools identified
4. Tool 1 started
5. Tool 1 completed: "Read 1024 bytes"
6. Tool 2 started
7. Tool 2 completed: "Wrote 1024 bytes"
8. Task complete
```

## Error Recovery Example

```
Tool 1: read_file → Success
Tool 2: write_file → FAILS (permission denied)

LLM Recovery:
"The write failed due to permission denied. 
 Try creating the directory first or using a different path."

Backend:
1. Emit ToolFailed event
2. Ask LLM for recovery strategy
3. LLM suggests: "Try creating the directory first"
4. Execute: mkdir output_dir
5. Retry: write_file to output_dir/output.txt
6. Success → continue

Tool 3: done → Success
```

## Testing

Unit tests are included in both modules:

```bash
cargo test streaming_tool_executor
cargo test sequential_executor
```

## Future Enhancements

1. **Parallel Tool Groups** - Execute independent tools in parallel
2. **Tool Dependencies** - Specify which tools depend on others
3. **Conditional Execution** - Skip tools based on previous results
4. **Tool Timeouts** - Configurable timeout per tool
5. **Retry Policies** - Exponential backoff, max retries
6. **Tool Caching** - Cache results of identical tool calls
