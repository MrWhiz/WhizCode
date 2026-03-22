# Phase 1 Testing Guide

## Quick Start

### 1. Build the Project
```bash
cd src-tauri
cargo build
```

### 2. Run the Application
```bash
npm run tauri dev
```

### 3. Test the Agent

Open the chat and try these test cases:

## Test Case 1: Simple Task (Bug Fix)

**Input**:
```
Fix the login bug in the authentication module
```

**Expected Output**:
1. Agent creates execution plan with 4 tasks:
   - Analyze the bug
   - Locate the source
   - Implement the fix
   - Verify the fix

2. Agent injects context including:
   - Execution plan
   - Learning recommendations
   - Context memory insights

3. Agent calls LLM with full context

4. Agent executes tools (read_file, write_file, run_command)

5. Agent records knowledge

**What to Check**:
- [ ] Plan is created and emitted to UI
- [ ] Context includes execution plan
- [ ] LLM is called
- [ ] Tools are executed
- [ ] Response is generated
- [ ] No errors occur

## Test Case 2: Feature Implementation

**Input**:
```
Add a new user profile feature
```

**Expected Output**:
1. Agent creates execution plan with 4 tasks:
   - Design the feature
   - Create files
   - Implement the feature
   - Test the feature

2. Agent injects context

3. Agent executes tools

4. Agent records knowledge

**What to Check**:
- [ ] Plan is created with correct tasks
- [ ] Context is injected
- [ ] Tools are executed
- [ ] Feature is implemented
- [ ] Knowledge is recorded

## Test Case 3: Refactoring

**Input**:
```
Refactor the database connection code
```

**Expected Output**:
1. Agent creates execution plan with 3 tasks:
   - Analyze code
   - Refactor code
   - Verify refactoring

2. Agent injects context

3. Agent executes tools

4. Agent records knowledge

**What to Check**:
- [ ] Plan is created with correct tasks
- [ ] Context is injected
- [ ] Tools are executed
- [ ] Code is refactored
- [ ] Knowledge is recorded

## Test Case 4: Analysis

**Input**:
```
Analyze the performance of the API
```

**Expected Output**:
1. Agent creates execution plan with 3 tasks:
   - Gather information
   - Analyze information
   - Provide insights

2. Agent injects context

3. Agent executes tools

4. Agent records knowledge

**What to Check**:
- [ ] Plan is created with correct tasks
- [ ] Context is injected
- [ ] Tools are executed
- [ ] Analysis is provided
- [ ] Knowledge is recorded

## Debugging

### Check Console Logs
Look for these log messages:
```
[PHASE_1] Starting Agent Loop Orchestration
[PHASE_1] Phase 1: Creating execution plan...
[PHASE_1] Phase 2: Building rich project context...
[PHASE_1] Phase 3: Running multi-turn agent loop...
[PHASE_1] Phase 4: Distilling knowledge in background...
[PLANNING] Creating execution plan for task: ...
[PLANNING] Task type: ...
[PLANNING] Created plan with X tasks
[CONTEXT] Built project context (X chars)
[LOOP] Iteration X/10
[LLM] Calling llama2 with prompt length: X
[LOOP] Executing tool: ...
[DISTILLATION] Recording knowledge from interaction
```

### Check UI Events
The agent emits these events:
- `agent:step` - Tool execution step
- `agent:plan` - Execution plan

### Check for Errors
Look for error messages in:
- Browser console
- Terminal output
- Tauri logs

## Performance Metrics

### Expected Timings
- Planning phase: < 1 second
- Context building: < 1 second
- LLM call: 5-30 seconds (depends on model)
- Tool execution: 1-10 seconds per tool
- Knowledge distillation: < 1 second

### Total Time
- Simple task: 10-40 seconds
- Complex task: 30-120 seconds

## Verification Checklist

### Planning Phase
- [ ] Plan is created
- [ ] Plan has correct task count
- [ ] Plan has correct task types
- [ ] Plan is emitted to UI

### Context Building
- [ ] Context includes execution plan
- [ ] Context includes learning recommendations
- [ ] Context includes context memory insights
- [ ] Context includes workspace path
- [ ] Context includes active file

### Multi-turn Loop
- [ ] LLM is called
- [ ] Tool calls are parsed
- [ ] Tools are executed
- [ ] Results are aggregated
- [ ] Loop continues correctly

### Knowledge Distillation
- [ ] Learning system is updated
- [ ] Context memory is updated
- [ ] No errors occur

### Error Handling
- [ ] Errors are caught
- [ ] Recovery suggestions are provided
- [ ] Agent continues after errors

## Common Issues

### Issue: Plan not created
**Solution**: Check that task classification is working
- Look for `[PLANNING] Task type:` in logs
- Verify request keywords match classification

### Issue: Context not injected
**Solution**: Check that context building is working
- Look for `[CONTEXT] Built project context` in logs
- Verify context includes all components

### Issue: LLM not called
**Solution**: Check that LLM connection is working
- Verify Ollama is running on localhost:11434
- Check LLM model name is correct

### Issue: Tools not executed
**Solution**: Check that tool execution is working
- Look for `[LOOP] Executing tool:` in logs
- Verify tool arguments are correct

### Issue: Knowledge not recorded
**Solution**: Check that distillation is working
- Look for `[DISTILLATION]` in logs
- Verify learning system and context memory are initialized

## Next Steps

1. **Run Test Case 1** - Simple bug fix task
2. **Verify all checks pass**
3. **Run Test Case 2** - Feature implementation
4. **Verify all checks pass**
5. **Run Test Case 3** - Refactoring
6. **Verify all checks pass**
7. **Run Test Case 4** - Analysis
8. **Verify all checks pass**
9. **Move to Phase 2** - Tool execution enhancement

## Success Criteria

✅ All test cases pass
✅ No errors occur
✅ Plans are created correctly
✅ Context is injected correctly
✅ Tools are executed correctly
✅ Knowledge is recorded correctly
✅ Performance is acceptable

---

**Status**: Ready for testing ✅
**Next**: Run test cases and verify
