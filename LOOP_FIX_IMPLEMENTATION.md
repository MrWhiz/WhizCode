# Agent Loop Fix - Implementation Guide

## Summary

The agent is stuck in a loop because:
1. It tries to use non-existent tools (`list_workspace`, `semantic_search`)
2. These tools are rejected but the agent keeps retrying
3. No hard exit condition exists for repeated validation errors

## Fix 1: Add Unknown Tool Detection (CRITICAL)

**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_tools_from_stream()` (around line 3799)
**Location:** Inside the tool validation section (around line 4055)

### Current Code (BROKEN):
```rust
let (is_valid, missing_arg) = match tool_name {
    "read_file" | "write_file" | ... => { /* validation */ }
    "run_command" => { /* validation */ }
    "ask_user" => { /* validation */ }
    _ => (true, None), // ← PROBLEM: Unknown tools pass through!
};
```

### Fixed Code:
```rust
// Define valid tools at the start of execute_tools_from_stream()
let valid_tools = [
    "done", "read_file", "write_file", "create_file", "create_directory",
    "delete_file", "move_file", "rename_file", "edit_file", "multi_edit_file",
    "list_directory", "search_files", "grep_search", "run_command", "ask_user",
    "view_structure"
];

// In the validation match:
let (is_valid, missing_arg) = match tool_name {
    "read_file" | "write_file" | "edit_file" | "create_file" | "delete_file" | "move_file" | "rename_file" => {
        if args.get("path").and_then(|p| p.as_str()).is_some() {
            (true, None)
        } else {
            (false, Some("path"))
        }
    },
    "multi_edit_file" => {
        let has_top_level_path = args.get("path").and_then(|p| p.as_str()).is_some();
        let has_paths_per_edit = args
            .get("edits")
            .and_then(|e| e.as_array())
            .or_else(|| args.get("changes").and_then(|e| e.as_array()))
            .map(|edits| !edits.is_empty() && edits.iter().all(|edit| edit.get("path").and_then(|p| p.as_str()).is_some()))
            .unwrap_or(false);
        if has_top_level_path || has_paths_per_edit {
            (true, None)
        } else {
            (false, Some("path"))
        }
    }
    "run_command" => {
        if args.get("command").and_then(|c| c.as_str()).is_some() {
            (true, None)
        } else {
            (false, Some("command"))
        }
    },
    "ask_user" => {
        if is_meaningful_ask_user_question(&args) {
            (true, None)
        } else {
            (false, Some("question"))
        }
    }
    "search_files" | "grep_search" => {
        if args.get("pattern").and_then(|p| p.as_str()).is_some() {
            (true, None)
        } else {
            (false, Some("pattern"))
        }
    }
    _ => {
        // Check if tool is in the valid list
        if valid_tools.contains(&tool_name) {
            (true, None)
        } else {
            (false, Some(&format!("unknown_tool: {}", tool_name)))
        }
    }
};
```

## Fix 2: Add Validation Error Threshold (HIGH PRIORITY)

**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_task_streaming()` (around line 1192)
**Location:** In the main execution loop (around line 1550)

### Add at the top of the loop:
```rust
const MAX_VALIDATION_ERRORS: u32 = 5;
let mut validation_error_count = 0u32;
```

### In the loop, after collecting tool results:
```rust
// Check for validation error threshold
if !rejected_tools.is_empty() {
    validation_error_count += 1;
    eprintln!("[Agent] Validation error #{}: {} tools rejected", validation_error_count, rejected_tools.len());
    
    if validation_error_count >= MAX_VALIDATION_ERRORS {
        eprintln!("[Agent] ⚠️ CRITICAL: {} consecutive validation errors. Exiting.", validation_error_count);
        status = "failed".to_string();
        response = format!(
            "Task failed: The agent produced {} consecutive validation errors. \
             This typically means the agent is trying to use tools that don't exist or \
             is unable to format tool calls correctly. \
             \n\nRejected tools:\n{}",
            validation_error_count,
            rejected_tools.iter().map(|t| format!("  - {}", t)).collect::<Vec<_>>().join("\n")
        );
        break; // Exit the main loop
    }
} else {
    validation_error_count = 0; // Reset on successful iteration
}
```

## Fix 3: Improve Loop Detection (MEDIUM PRIORITY)

**File:** `src-tauri/src/commands/agent_streaming.rs`
**Function:** `execute_task_streaming()` (around line 1500)
**Location:** In the main execution loop

### Current code (around line 1500):
```rust
let mut previous_tool_sig = String::new();
let mut repeat_count = 0u32;
let mut tool_sig_history: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(4);
```

### Enhanced version:
```rust
let mut previous_tool_sig = String::new();
let mut repeat_count = 0u32;
const PATTERN_WINDOW: usize = 6;
let mut tool_sig_history: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(PATTERN_WINDOW);
let mut no_progress_count = 0u32;

// After collecting tool_calls:
let current_sig = tool_calls.iter()
    .map(|tc| tc.tool.clone())
    .collect::<Vec<_>>()
    .join(",");

tool_sig_history.push_back(current_sig.clone());
if tool_sig_history.len() > PATTERN_WINDOW {
    tool_sig_history.pop_front();
}

// Detect exact repetition (A,A,A)
let is_repeating = tool_sig_history.len() >= 3 
    && tool_sig_history[tool_sig_history.len()-1] == tool_sig_history[tool_sig_history.len()-2]
    && tool_sig_history[tool_sig_history.len()-2] == tool_sig_history[tool_sig_history.len()-3];

// Detect ping-pong (A,B,A,B,A,B)
let is_ping_pong = tool_sig_history.len() >= 4
    && tool_sig_history[tool_sig_history.len()-1] == tool_sig_history[tool_sig_history.len()-3]
    && tool_sig_history[tool_sig_history.len()-2] == tool_sig_history[tool_sig_history.len()-4];

if is_repeating || is_ping_pong {
    eprintln!("[Agent] ⚠️ Loop pattern detected!");
    tool_results.push("[SYSTEM] LOOP DETECTED: You are repeating the same actions. Try a different approach or make a concrete edit.".to_string());
}

// Track no-progress iterations
if tool_calls.is_empty() && !done {
    no_progress_count += 1;
    if no_progress_count >= 5 {
        eprintln!("[Agent] ⚠️ No progress for 5 iterations. Forcing exit.");
        status = "failed".to_string();
        response = "Task failed: Agent made no progress for 5 consecutive iterations.".to_string();
        break;
    }
} else if !tool_calls.is_empty() {
    no_progress_count = 0; // Reset on any tool execution
}
```

## Testing the Fix

After implementing these changes:

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Test with the travel vlog task:**
   - The agent should now reject `list_workspace` and `semantic_search` as unknown tools
   - After 5 validation errors, it should exit cleanly with an error message
   - The loop should not continue indefinitely

3. **Expected behavior:**
   ```
   Iteration 1: Tries list_workspace → REJECTED (unknown tool)
   Iteration 2: Tries semantic_search → REJECTED (unknown tool)
   Iteration 3: Tries search_files → REJECTED (missing pattern)
   Iteration 4: Validation error count = 3
   Iteration 5: Validation error count = 4
   Iteration 6: Validation error count = 5 → HARD EXIT
   → Task fails with clear error message
   ```

## Why These Fixes Work

1. **Fix 1** prevents unknown tools from being queued in the first place
2. **Fix 2** provides a hard exit condition after repeated failures
3. **Fix 3** detects more complex loop patterns and prevents thrashing

Together, they ensure the agent either:
- Makes progress (tools execute successfully)
- Fails fast (validation errors exceed threshold)
- Detects loops and adjusts strategy

## Files Modified

- `src-tauri/src/commands/agent_streaming.rs` (3 locations)
  - Line ~4055: Add valid_tools array and unknown tool detection
  - Line ~1550: Add validation error threshold check
  - Line ~1500: Enhance loop detection with pattern analysis

## Rollback Plan

If issues arise, revert to the previous commit:
```bash
git revert HEAD
```

The changes are isolated to the agent streaming logic and don't affect other systems.
