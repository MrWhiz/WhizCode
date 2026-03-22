# Quick Test Guide - Agent Execution

## Prerequisites
- Node.js and npm installed
- Rust and Cargo installed
- Project built and ready

## Quick Start Test

### Step 1: Start the Application
```bash
npm run tauri dev
```

### Step 2: Send a Test Task
In the UI, send one of these tasks:

**Test 1: Create a File**
```
Create a file called hello.txt with content "Hello, World!"
```
Expected: File should be created in the workspace

**Test 2: Read a File**
```
Read the package.json file
```
Expected: File contents should be displayed

**Test 3: Run a Command**
```
Run npm --version
```
Expected: npm version should be displayed

## What to Look For

### Success Indicators
- ✓ Agent responds with steps
- ✓ Tools are executed (read_file, write_file, run_command)
- ✓ Results are displayed in the UI
- ✓ No errors in the console

### Console Logs (Check Browser DevTools)
```
[LLM] Calling llama2 with prompt length: ...
[LOOP] Iteration 1/10
[LOOP] Executing tool: write_file
[CACHE] Cached result for write_file
[DISTILLATION] Recording knowledge from interaction
```

### If LLM Fails (Expected without Ollama)
```
[LLM] Attempt 1/3 to call LLM
[LLM] Connection error: Failed to connect to LLM: ...
[LLM] Retrying in 2 seconds...
[LLM] Attempt 2/3 to call LLM
[LLM] Connection error: Failed to connect to LLM: ...
[LLM] Retrying in 2 seconds...
[LLM] Attempt 3/3 to call LLM
[LLM] Connection error: Failed to connect to LLM: ...
[LLM] All LLM attempts failed, using fallback response
[LLM] Using fallback response
```

This is NORMAL and EXPECTED. The agent will still execute tools using the fallback response.

## Troubleshooting

### Issue: Agent doesn't respond
**Solution**: 
1. Check browser console for errors
2. Check terminal for Rust errors
3. Verify workspace path is set correctly

### Issue: Tools don't execute
**Solution**:
1. Check that workspace path exists
2. Verify file permissions
3. Check console logs for specific errors

### Issue: File operations fail
**Solution**:
1. Ensure workspace directory exists
2. Check file permissions
3. Verify paths are correct

## Advanced Testing

### Test with Ollama (Optional)
If you want to test with actual LLM:

```bash
# 1. Install Ollama from https://ollama.ai
# 2. Start Ollama
ollama serve

# 3. Pull a model
ollama pull llama2

# 4. Start the app
npm run tauri dev

# 5. Send a task
# The agent will now use the real LLM instead of fallback
```

### Monitor Performance
Check the console for timing information:
```
[STRATEGIC_PLANNING] Took 123ms
[CONTEXT_BUILDING] Took 456ms
[AGENT_LOOP] Took 789ms
[DISTILLATION] Took 234ms
```

## Expected Behavior

### Without Ollama
1. Agent tries to connect to LLM (fails)
2. Agent retries 2 more times
3. Agent uses intelligent fallback response
4. Tools execute successfully
5. Results displayed to user

### With Ollama
1. Agent connects to LLM (succeeds)
2. LLM generates response
3. Tools extracted from response
4. Tools execute successfully
5. Results displayed to user

## Success Criteria

✓ Agent responds to user input
✓ Tools are executed (at least one)
✓ Results are displayed
✓ No crashes or errors
✓ Logs show proper execution flow

If all criteria are met, the agent system is working correctly!
