# Agent Loop Debugging Summary

## Problem Identified

The agent is stuck in an infinite loop when executing the "Create a travel vlog website" task. The execution log shows:

- **Iterations 1-3:** Agent tries to use non-existent tools (`list_workspace`, `semantic_search`, `search_files`)
- **Iterations 4-9:** Agent repeatedly reads the same spec file with identical parameters
- **Iteration 10:** Agent attempts to generate HTML but response gets truncated mid-JSON
- **Result:** Loop continues indefinitely with no progress

## Root Causes

### 1. Unknown Tool Acceptance (CRITICAL)
**Location:** `execute_tools_from_stream()` line ~4055

The tool validation logic has a catch-all case that accepts ANY unknown tool:
```rust
_ => (true, None), // Other tools don't have strict validation
```

This allows tools like `list_workspace` and `semantic_search` to be queued even though they don't exist in `execute_single_tool()`.

**Impact:** Agent wastes iterations trying to execute non-existent tools

### 2. No Validation Error Threshold
**Location:** `execute_task_streaming()` line ~1550

When tools are rejected due to validation errors, the agent just sends an error message and retries. There's no hard exit condition.

**Impact:** Agent can retry indefinitely without making progress

### 3. Weak Loop Detection
**Location:** `execute_task_streaming()` line ~1500

Current detection only catches exact repetition after 3 iterations:
```rust
if sig == previous_tool_sig {
    repeat_count += 1;
    if repeat_count >= 3 { /* warn */ }
}
```

This misses:
- Variations (same tool, different args)
- Ping-pong patterns (A→B→A→B)
- Thrashing patterns (A→B→C→A→B→C)

**Impact:** Agent can loop in complex patterns without detection

### 4. No No-Progress Detection
**Location:** `execute_task_streaming()` line ~1600

When the agent produces no tool calls (empty response), there's no counter to track consecutive failures.

**Impact:** Agent can stall indefinitely

## Valid Tools Available

The agent has access to these tools (from `execute_single_tool()`):
- `done` - Mark task complete
- `read_file` - Read file
- `write_file` - Write/create file
- `create_file` - Alias for write_file
- `create_directory` - Create directory
- `delete_file` - Delete file/directory
- `move_file` / `rename_file` - Move/rename
- `edit_file` - Edit specific lines
- `multi_edit_file` - Multiple edits
- `list_directory` - List directory
- `search_files` - Search by filename pattern
- `grep_search` - Search file contents
- `run_command` - Execute shell command
- `ask_user` - Ask user for input

**Invalid tools the agent tried:**
- `list_workspace` ❌ Not implemented
- `semantic_search` ❌ Not implemented

## Solution Overview

Three fixes are needed (see `LOOP_FIX_IMPLEMENTATION.md` for details):

### Fix 1: Unknown Tool Detection (CRITICAL)
Add a valid tools list and reject unknown tools immediately:
```rust
let valid_tools = ["done", "read_file", "write_file", ...];
if !valid_tools.contains(&tool_name) {
    reject_tool("unknown_tool");
}
```

### Fix 2: Validation Error Threshold (HIGH)
Exit after 5 consecutive validation errors:
```rust
const MAX_VALIDATION_ERRORS: u32 = 5;
if validation_error_count >= MAX_VALIDATION_ERRORS {
    exit_with_error();
}
```

### Fix 3: Enhanced Loop Detection (MEDIUM)
Detect patterns beyond exact repetition:
- Exact repetition (A,A,A)
- Ping-pong (A,B,A,B)
- Thrashing (A,B,C,A,B,C)
- No progress (5+ empty iterations)

## Expected Outcome After Fixes

```
Iteration 1: list_workspace → REJECTED (unknown tool)
Iteration 2: semantic_search → REJECTED (unknown tool)
Iteration 3: search_files → REJECTED (missing pattern)
Iteration 4: Validation error count = 3
Iteration 5: Validation error count = 4
Iteration 6: Validation error count = 5 → HARD EXIT
→ Task fails with clear error message
```

Instead of:
```
Iteration 1-10: Repeated attempts
Iteration 11+: Infinite loop
```

## Implementation Steps

1. **Read:** `LOOP_FIX_IMPLEMENTATION.md` for exact code changes
2. **Modify:** `src-tauri/src/commands/agent_streaming.rs` (3 locations)
3. **Build:** `cargo build --release`
4. **Test:** Run the travel vlog task again
5. **Verify:** Agent should exit cleanly after validation errors

## Files Analyzed

- `src-tauri/src/commands/agent_streaming.rs` - Main agent loop (6786 lines)
- `src-tauri/src/commands/streaming_agent_flow.rs` - Streaming flow (7861 lines)
- `docs/AGENT_LIFECYCLE.md` - Architecture documentation
- Execution logs from the failed task run

## Key Insights

1. **Tool validation is too permissive** - Unknown tools should be rejected immediately
2. **No hard exit conditions** - Agent needs thresholds for failure states
3. **Loop detection is incomplete** - Only catches simple patterns
4. **Error recovery is insufficient** - Validation errors should trigger exit, not retry

## Next Steps

1. Implement the three fixes in `agent_streaming.rs`
2. Test with the travel vlog task
3. Verify agent exits cleanly on validation errors
4. Consider adding telemetry to track loop patterns in production

## References

- `DEBUG_LOOP_ANALYSIS.md` - Detailed technical analysis
- `LOOP_FIX_IMPLEMENTATION.md` - Step-by-step implementation guide
- Execution logs in `.whizcode/debug/` directory
