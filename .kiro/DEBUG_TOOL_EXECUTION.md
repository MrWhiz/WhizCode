# Debug Tool Execution Issues

## Problem
Agent shows "Executed run_command" and "Executed write_file" but with "(No logs yet)" - tools are being called but not actually executing or returning results.

## Root Causes to Check

### 1. Tool Arguments Not Parsing Correctly
The XML tool call format might not be parsing the arguments correctly.

**Check in Console Logs**:
```
[TOOL] Executing tool: write_file
[TOOL] Tool args: {"path":"...","content":"..."}
[TOOL] Workspace path: Some("...")
```

If `Tool args` shows `{}` (empty), the parsing failed.

### 2. File Paths Not Resolving
The tool might be trying to write to a path that doesn't exist or isn't accessible.

**Check in Console Logs**:
```
[TOOL] Writing file: /path/to/file.txt (123 bytes)
[TOOL] Successfully wrote file: /path/to/file.txt
```

If you see an error instead, the file operation failed.

### 3. Workspace Path Not Set
The workspace path might be None, causing file operations to fail.

**Check in Console Logs**:
```
[TOOL] Workspace path: None
```

This would cause relative paths to fail.

### 4. Tool Extraction Not Finding Tools
The XML parsing might not be extracting tool calls correctly.

**Check in Console Logs**:
```
[LOOP] Iteration 1/10
[LOOP] No tool calls, agent is done
```

If you see "No tool calls", the extraction failed.

## Step-by-Step Debugging

### Step 1: Check Browser Console
1. Open the app (npm run tauri dev)
2. Press F12 to open DevTools
3. Go to Console tab
4. Look for errors

### Step 2: Check Terminal Output
1. Look at the terminal where you ran `npm run tauri dev`
2. Search for `[TOOL]` logs
3. Look for `[LLM]` logs
4. Look for `[LOOP]` logs

### Step 3: Send a Simple Task
```
Create a file called test.txt with content hello
```

### Step 4: Check the Logs

**Expected Log Sequence**:
```
[PHASE_1] Starting Agent Loop Orchestration
[PLANNING] Creating execution plan for task
[CONTEXT] Built project context
[LOOP] Iteration 1/10
[LLM] Calling llama2 with prompt length: 1234
[LLM] Attempt 1/3 to call LLM
[LLM] Connection error: Failed to connect to LLM
[LLM] Retrying in 2 seconds...
[LLM] Attempt 2/3 to call LLM
[LLM] Connection error: Failed to connect to LLM
[LLM] Retrying in 2 seconds...
[LLM] Attempt 3/3 to call LLM
[LLM] Connection error: Failed to connect to LLM
[LLM] All LLM attempts failed, using fallback response
[LLM] Using fallback response
[LOOP] Executing tool: write_file
[TOOL] Executing tool: write_file
[TOOL] Tool args: {"path":"src/index.ts","content":"// Main entry point\nexport default {}"}
[TOOL] Workspace path: Some("/path/to/workspace")
[TOOL] Writing file: src/index.ts (45 bytes)
[TOOL] Successfully wrote file: src/index.ts
[CACHE] Cached result for write_file
[DISTILLATION] Recording knowledge from interaction
```

### Step 5: Identify the Issue

**If you see**:
```
[TOOL] Tool args: {}
```
→ **Problem**: XML parsing failed. Arguments not extracted.

**If you see**:
```
[TOOL] Workspace path: None
```
→ **Problem**: Workspace path not set. File operations will fail.

**If you see**:
```
[TOOL] Error writing file: Permission denied
```
→ **Problem**: File permissions issue.

**If you see**:
```
[TOOL] Error writing file: No such file or directory
```
→ **Problem**: Parent directory doesn't exist.

**If you see**:
```
[LOOP] No tool calls, agent is done
```
→ **Problem**: Tool extraction failed. No tools found in response.

## Common Issues and Fixes

### Issue 1: Empty Tool Arguments
**Symptom**: `[TOOL] Tool args: {}`

**Cause**: XML parsing failed

**Fix**: Check the fallback response format in `call_llm()`. The XML format must be:
```xml
<tool_call>
<tool_name>write_file</tool_name>
<tool_args>{"path":"file.txt","content":"hello"}</tool_args>
</tool_call>
```

### Issue 2: Workspace Path is None
**Symptom**: `[TOOL] Workspace path: None`

**Cause**: Workspace path not passed from frontend

**Fix**: In the UI, ensure workspace path is set before sending task

### Issue 3: No Tool Calls Found
**Symptom**: `[LOOP] No tool calls, agent is done`

**Cause**: Tool extraction not finding XML blocks

**Fix**: Check that the fallback response contains `<tool_call>` tags

### Issue 4: File Not Created
**Symptom**: Tool says "Successfully wrote" but file doesn't exist

**Cause**: Path is relative and workspace path is wrong

**Fix**: Use absolute paths or ensure workspace path is correct

## Testing Tool Execution Directly

### Test 1: Create a File
```
Task: Create a file called hello.txt with content "Hello, World!"

Expected Logs:
[TOOL] Writing file: hello.txt (13 bytes)
[TOOL] Successfully wrote file: hello.txt

Expected Result:
File hello.txt created in workspace with content "Hello, World!"
```

### Test 2: Read a File
```
Task: Read the package.json file

Expected Logs:
[TOOL] Reading file: package.json
[TOOL] Successfully read file: 1234 bytes

Expected Result:
File contents displayed in UI
```

### Test 3: Run a Command
```
Task: Run npm --version

Expected Logs:
[TOOL] Running command: npm --version
[TOOL] Command completed with status: exit status: 0
[TOOL] Stdout length: 10

Expected Result:
npm version displayed in UI
```

## Advanced Debugging

### Enable Rust Backtrace
```bash
RUST_BACKTRACE=1 npm run tauri dev
```

### Check File System Permissions
```bash
# On Linux/Mac
ls -la /path/to/workspace

# On Windows
dir /path/to/workspace
```

### Verify Workspace Path
In the UI, check what workspace path is being sent:
1. Open DevTools (F12)
2. Go to Network tab
3. Look for `execute_agent_loop` call
4. Check the `workspace_path` parameter

### Monitor File System Changes
```bash
# On Linux/Mac
watch -n 1 'ls -la /path/to/workspace'

# On Windows
Get-ChildItem /path/to/workspace -Recurse | Watch-Object
```

## Summary of Debug Steps

1. ✓ Send a simple task
2. ✓ Check browser console for errors
3. ✓ Check terminal for `[TOOL]` logs
4. ✓ Identify which log is missing
5. ✓ Match the issue to the list above
6. ✓ Apply the fix
7. ✓ Test again

## If Still Not Working

1. Check that workspace path is set correctly
2. Verify file permissions
3. Check that parent directories exist
4. Try with absolute paths
5. Check terminal for error messages
6. Review the logs carefully for clues

The logs will tell you exactly what's happening. Look for `[TOOL]` prefix in the logs to see tool execution details.
