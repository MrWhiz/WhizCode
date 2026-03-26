# Agent Loop Flow Diagram

## Current (Broken) Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 1: Agent tries list_workspace                         │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "list_workspace", "args": {}}          │
│ 2. Tool validation: _ => (true, None)  ← ACCEPTS UNKNOWN TOOL   │
│ 3. Tool queued: list_workspace                                  │
│ 4. Tool execution: Unknown tool: list_workspace                 │
│ 5. Result: REJECTED                                             │
│ 6. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 2: Agent tries semantic_search                        │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "semantic_search", "args": {...}}      │
│ 2. Tool validation: _ => (true, None)  ← ACCEPTS UNKNOWN TOOL   │
│ 3. Tool queued: semantic_search                                 │
│ 4. Tool execution: Unknown tool: semantic_search                │
│ 5. Result: REJECTED                                             │
│ 6. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 3: Agent tries search_files (missing pattern)         │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "search_files", "args": {}}            │
│ 2. Tool validation: Missing "pattern" argument                  │
│ 3. Tool rejected: VALIDATION ERROR                              │
│ 4. Agent receives error, continues loop                         │
│ 5. validation_error_count = 1 (no threshold check)              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATIONS 4-9: Agent reads same file repeatedly                │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "read_file", "path": "...", ...}       │
│ 2. Tool validation: PASSES (has path)                           │
│ 3. Tool queued: read_file                                       │
│ 4. Tool execution: Reads file                                   │
│ 5. Result: File content returned                                │
│ 6. Loop detection: Detects repetition after 3 iterations        │
│ 7. Agent receives warning, continues loop                       │
│ 8. repeat_count increments but no hard exit                     │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 10: Agent attempts HTML generation                    │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: Large JSON with HTML content                    │
│ 2. Response gets truncated mid-JSON                             │
│ 3. JSON parsing fails                                           │
│ 4. No tools extracted                                           │
│ 5. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    ┌─────────────────┐
                    │  INFINITE LOOP  │
                    │  (No exit)      │
                    └─────────────────┘
```

## Fixed Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 1: Agent tries list_workspace                         │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "list_workspace", "args": {}}          │
│ 2. Tool validation: Check valid_tools array                     │
│ 3. Result: NOT IN VALID TOOLS                                   │
│ 4. Tool rejected: UNKNOWN TOOL                                  │
│ 5. validation_error_count = 1                                   │
│ 6. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 2: Agent tries semantic_search                        │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "semantic_search", "args": {...}}      │
│ 2. Tool validation: Check valid_tools array                     │
│ 3. Result: NOT IN VALID TOOLS                                   │
│ 4. Tool rejected: UNKNOWN TOOL                                  │
│ 5. validation_error_count = 2                                   │
│ 6. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 3: Agent tries search_files (missing pattern)         │
├─────────────────────────────────────────────────────────────────┤
│ 1. LLM outputs: {"tool": "search_files", "args": {}}            │
│ 2. Tool validation: Missing "pattern" argument                  │
│ 3. Tool rejected: VALIDATION ERROR                              │
│ 4. validation_error_count = 3                                   │
│ 5. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 4: More validation errors                             │
├─────────────────────────────────────────────────────────────────┤
│ 1. validation_error_count = 4                                   │
│ 2. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 5: More validation errors                             │
├─────────────────────────────────────────────────────────────────┤
│ 1. validation_error_count = 5                                   │
│ 2. Agent receives error, continues loop                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION 6: THRESHOLD REACHED                                  │
├─────────────────────────────────────────────────────────────────┤
│ 1. validation_error_count >= MAX_VALIDATION_ERRORS (5)          │
│ 2. Check: if validation_error_count >= 5 { EXIT }              │
│ 3. Status: FAILED                                               │
│ 4. Response: Clear error message with rejected tools            │
│ 5. Break from main loop                                         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    ┌─────────────────┐
                    │  CLEAN EXIT     │
                    │  (With error)   │
                    └─────────────────┘
```

## Tool Validation Logic

### Current (Broken)
```
Tool Name Input
      ↓
┌─────────────────────────────────┐
│ Match tool_name                 │
├─────────────────────────────────┤
│ "read_file" → Check path        │
│ "write_file" → Check path       │
│ "run_command" → Check command   │
│ "ask_user" → Check question     │
│ _ → (true, None) ← ACCEPTS ALL  │
└─────────────────────────────────┘
      ↓
   ACCEPT (even unknown tools!)
```

### Fixed
```
Tool Name Input
      ↓
┌─────────────────────────────────┐
│ Check valid_tools array         │
├─────────────────────────────────┤
│ "done" ✓                        │
│ "read_file" ✓                   │
│ "write_file" ✓                  │
│ ... (14 valid tools)            │
│ "list_workspace" ✗ NOT FOUND    │
│ "semantic_search" ✗ NOT FOUND   │
└─────────────────────────────────┘
      ↓
   REJECT (unknown tool)
      ↓
┌─────────────────────────────────┐
│ If in valid_tools:              │
│   Match tool_name               │
│   "read_file" → Check path      │
│   "write_file" → Check path     │
│   "run_command" → Check command │
│   "ask_user" → Check question   │
│   "search_files" → Check pattern│
│   _ → (true, None)              │
└─────────────────────────────────┘
      ↓
   ACCEPT or REJECT (based on args)
```

## Loop Detection Enhancement

### Current (Weak)
```
Tool Signature: "read_file:path1"
                      ↓
            Compare with previous
                      ↓
        Same? repeat_count++
        Different? repeat_count = 0
                      ↓
        repeat_count >= 3?
        YES → Send warning
        NO → Continue
                      ↓
        (No hard exit, just warning)
```

### Fixed (Strong)
```
Tool Signature: "read_file:path1"
                      ↓
        Add to history (last 6)
                      ↓
    ┌───────────────────────────────┐
    │ Check for patterns:           │
    ├───────────────────────────────┤
    │ 1. Exact repetition (A,A,A)   │
    │ 2. Ping-pong (A,B,A,B,A,B)    │
    │ 3. Thrashing (A,B,C,A,B,C)    │
    │ 4. No progress (5+ empty)     │
    └───────────────────────────────┘
                      ↓
        Pattern detected?
        YES → Send warning + force change
        NO → Continue
                      ↓
        (Can detect complex patterns)
```

## Validation Error Accumulation

### Current (No Threshold)
```
Iteration 1: validation_error_count = 1
Iteration 2: validation_error_count = 2
Iteration 3: validation_error_count = 3
Iteration 4: validation_error_count = 4
Iteration 5: validation_error_count = 5
Iteration 6: validation_error_count = 6
...
Iteration ∞: validation_error_count = ∞
             (No exit condition)
```

### Fixed (With Threshold)
```
Iteration 1: validation_error_count = 1
Iteration 2: validation_error_count = 2
Iteration 3: validation_error_count = 3
Iteration 4: validation_error_count = 4
Iteration 5: validation_error_count = 5
Iteration 6: Check: validation_error_count >= 5?
             YES → EXIT with error
             (Clean exit)
```

## Summary

| Aspect | Current | Fixed |
|--------|---------|-------|
| Unknown tools | Accepted | Rejected |
| Validation errors | Infinite retries | Exit after 5 |
| Loop detection | Simple (A,A,A) | Complex (A,B,A,B) |
| No-progress detection | None | 5 iteration limit |
| Exit condition | None | Multiple conditions |
| Result | Infinite loop | Clean failure |
