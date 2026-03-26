# Agent Loop Detection & Recovery Analysis

## Problem Summary

The agent is stuck in a repetitive loop where it:
1. Attempts to call non-existent tools (`list_workspace`, `semantic_search`, `search_files`)
2. Gets validation errors for missing required arguments
3. Re-reads the same spec file repeatedly (iterations 4-9)
4. Eventually attempts to generate HTML but the response gets truncated mid-JSON
5. Never makes progress toward the actual task (creating a travel vlog website)

## Root Causes Identified

### 1. **Tool Availability Mismatch**
**Location:** `execute_tools_from_stream()` in `agent_streaming.rs`

The agent is trying to use tools that don't exist in its available toolset:
- `list_workspace` - Not a valid tool
- `semantic_search` - Not a valid tool  
- `search_files` - Not a valid tool

These tools are being rejected during validation but the agent doesn't have a recovery strategy for unknown tools.

**Current Code (Line ~3900):**
```rust
let (is_valid, missing_arg) = match tool_name {
    "read_file" | "write_file" | "edit_file" | ... => { /* validation */ }
    "run_command" => { /* validation */ }
    "ask_user" => { /* validation */ }
    _ => (true, None), // ← PROBLEM: Unknown tools pass through!
};
```

### 2. **Insufficient Loop Detection**
**Location:** `execute_task_streaming()` in `agent_streaming.rs` (Line ~1500+)

The loop detection logic exists but has gaps:

**Current Implementation:**
```rust
let mut previous_tool_sig = String::new();
let mut repeat_count = 0u32;
let mut tool_sig_history: VecDeque<String> = VecDeque::with_capacity(4);

// Only triggers after 3 identical repetitions
if sig == previous_tool_sig {
    repeat_count += 1;
    if repeat_count >= 3 {
        // Send warning
    }
}
```

**Problems:**
- Only detects exact repetition (same tool + same args)
- Doesn't detect variations like `read_file` with slightly different line ranges
- Doesn't detect "thrashing" patterns (A→B→A→B)
- Doesn't track validation error accumulation
- Doesn't have a hard exit condition

### 3. **Validation Error Accumulation Not Tracked**
**Location:** `execute_task_streaming()` (Line ~1550+)

```rust
let mut validation_error_count = 0u32;
// ... later ...
if !rejected_tools.is_empty() {
    validation_error_count += 1;
    // Sends error message but doesn't exit
}
```

**Problem:** After 3+ validation errors, the agent should either:
- Ask the user for clarification
- Attempt a different strategy
- Exit with a clear error

Currently it just keeps retrying.

### 4. **No Unknown Tool Recovery**
The agent has no strategy for handling tools it doesn't recognize. When `list_workspace` fails, it should:
1. Recognize this is an unknown tool
2. Fall back to available alternatives
3. Not retry the same unknown tool

## Recommended Fixes

### Fix 1: Validate Tool Names Against Available Tools
**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_tools_from_stream()` (around line 3900)

```rust
// Add a set of valid tool names at the top of the function
let valid_tools = std::collections::HashSet::from([
    "read_file", "write_file", "edit_file", "create_file", "delete_file",
    "move_file", "rename_file", "multi_edit_file",
    "run_command", "ask_user", "done",
    "grep_search", "view_structure",
    // ... add all valid tools
]);

// In the tool validation section:
let (is_valid, missing_arg) = match tool_name {
    // ... existing validation ...
    _ => {
        if !valid_tools.contains(tool_name) {
            (false, Some(&format!("unknown_tool: {}", tool_name)))
        } else {
            (true, None)
        }
    }
};
```

### Fix 2: Enhance Loop Detection
**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_task_streaming()` (around line 1500)

Add pattern detection:
```rust
// Track last N tool signatures for pattern detection
const PATTERN_WINDOW: usize = 6;
let mut tool_sig_history: VecDeque<String> = VecDeque::with_capacity(PATTERN_WINDOW);

// After collecting tool_calls:
let current_sig = tool_calls.iter()
    .map(|tc| tc.tool.clone())
    .collect::<Vec<_>>()
    .join(",");

tool_sig_history.push_back(current_sig.clone());
if tool_sig_history.len() > PATTERN_WINDOW {
    tool_sig_history.pop_front();
}

// Detect patterns:
// 1. Exact repetition (A,A,A)
// 2. Ping-pong (A,B,A,B,A,B)
// 3. Thrashing (A,B,C,A,B,C)

let is_repeating = tool_sig_history.len() >= 3 
    && tool_sig_history[tool_sig_history.len()-1] == tool_sig_history[tool_sig_history.len()-2]
    && tool_sig_history[tool_sig_history.len()-2] == tool_sig_history[tool_sig_history.len()-3];

let is_ping_pong = tool_sig_history.len() >= 4
    && tool_sig_history[tool_sig_history.len()-1] == tool_sig_history[tool_sig_history.len()-3]
    && tool_sig_history[tool_sig_history.len()-2] == tool_sig_history[tool_sig_history.len()-4];

if is_repeating || is_ping_pong {
    eprintln!("[Agent] ⚠️ Loop pattern detected!");
    // Force a different strategy or exit
}
```

### Fix 3: Hard Exit on Validation Error Threshold
**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_task_streaming()` (around line 1550)

```rust
const MAX_VALIDATION_ERRORS: u32 = 5;

if !rejected_tools.is_empty() {
    validation_error_count += 1;
    
    if validation_error_count >= MAX_VALIDATION_ERRORS {
        eprintln!("[Agent] ⚠️ CRITICAL: {} validation errors. Exiting.", validation_error_count);
        status = "failed".to_string();
        response = format!(
            "Task failed: The agent produced {} consecutive validation errors. \
             This typically means the agent is trying to use tools that don't exist or \
             is unable to format tool calls correctly. Please check the agent configuration.",
            validation_error_count
        );
        break; // Exit the main loop
    }
}
```

### Fix 4: Improve Stall Detection
**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_task_streaming()` (around line 1600)

```rust
// Track consecutive iterations with no progress
let mut no_progress_count = 0u32;

// After tool execution:
if tool_calls.is_empty() && !done {
    no_progress_count += 1;
    if no_progress_count >= 5 {
        eprintln!("[Agent] ⚠️ No progress for 5 iterations. Forcing exit.");
        status = "failed".to_string();
        response = "Task failed: Agent made no progress for 5 consecutive iterations. \
                   This suggests the agent is stuck or unable to proceed.".to_string();
        break;
    }
} else if !tool_calls.is_empty() {
    no_progress_count = 0; // Reset on any tool execution
}
```

## Current Behavior vs. Expected Behavior

### Current (Broken)
```
Iteration 1: Tries list_workspace → REJECTED (unknown tool)
Iteration 2: Tries semantic_search → REJECTED (unknown tool)
Iteration 3: Tries search_files → REJECTED (unknown tool)
Iteration 4-9: Reads spec file repeatedly (same window)
Iteration 10: Attempts HTML generation → TRUNCATED
→ Loop continues indefinitely
```

### Expected (After Fixes)
```
Iteration 1: Tries list_workspace → REJECTED (unknown tool)
Iteration 2: Tries semantic_search → REJECTED (unknown tool)
Iteration 3: Tries search_files → REJECTED (unknown tool)
Iteration 4: Validation error count = 3
Iteration 5: Validation error count = 4
Iteration 6: Validation error count = 5 → HARD EXIT
→ Task fails with clear error message
```

## Implementation Priority

1. **HIGH:** Fix 1 (Tool validation) - Prevents unknown tools from being queued
2. **HIGH:** Fix 3 (Validation error threshold) - Prevents infinite retry loops
3. **MEDIUM:** Fix 2 (Pattern detection) - Catches more complex loop patterns
4. **MEDIUM:** Fix 4 (No progress detection) - Catches stalls from other causes

## Testing Strategy

After implementing fixes:

1. **Unit Test:** Verify unknown tools are rejected
2. **Integration Test:** Run the travel vlog task and verify it exits cleanly
3. **Regression Test:** Ensure valid tasks still complete successfully
4. **Edge Case Test:** Test with various invalid tool combinations

## Valid Tools (from execute_single_tool)

The agent has access to these tools:
- `done` - Mark task as complete
- `read_file` - Read file content
- `write_file` - Write/create file
- `create_file` - Alias for write_file
- `create_directory` - Create directory
- `delete_file` - Delete file or directory
- `move_file` / `rename_file` - Move/rename file
- `edit_file` - Edit specific lines in a file
- `multi_edit_file` - Apply multiple edits to a file
- `list_directory` - List directory contents
- `search_files` - Search for files by pattern
- `grep_search` - Search file contents by pattern
- `run_command` - Execute shell command
- `ask_user` - Ask user for input

**Invalid tools the agent tried to use:**
- `list_workspace` - ❌ Not implemented
- `semantic_search` - ❌ Not implemented
- `search_files` - ✅ Actually valid, but agent may have used wrong arguments

## Files to Modify

- `src-tauri/src/commands/agent_streaming.rs` - Main fixes (4 locations)
  - Line ~4055: Tool validation in `execute_tools_from_stream()`
  - Line ~1500: Loop detection in `execute_task_streaming()`
  - Line ~1550: Validation error tracking
  - Line ~1600: Stall detection
