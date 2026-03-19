# WhizCode Planning Integration - Practical Example

This document shows exactly how to integrate the planning system into your existing agent handler.

## Current Flow (Before)

```typescript
ipcMain.handle('execute-agent-task', async (_event, { task, model, workspacePath, activeFile, config, isAutopilotMode }) => {
  // ... workspace setup ...
  
  // Run agent loop directly
  const result = await runAgentLoop(task, model, config, workspacePath, activeFile, isAutopilotMode);
  
  return {
    response: result.finalResponse,
    steps: result.steps
  };
});
```

## New Flow (After)

```typescript
import { AgentExecutor } from './agentExecutor';
import { PlanningContext } from './whizCodePlanner';

// Create executor instance (at module level)
const agentExecutor = new AgentExecutor();

ipcMain.handle('execute-agent-task', async (_event, { task, model, workspacePath, activeFile, config, isAutopilotMode }) => {
  if (!workspacePath) {
    return {
      response: "I'm ready to help, but I need you to open a folder first...",
      steps: []
    };
  }
  
  try {
    abortRequested = false;
    agentAbortController = new AbortController();
    currentIterationLimit = MAX_AGENT_ITERATIONS;

    // ... existing workspace setup code ...

    // ============================================
    // NEW: PLANNING PHASE
    // ============================================
    const planningContext: PlanningContext = {
      userRequest: task,
      workspacePath,
      activeFile
    };

    const plan = await agentExecutor.planTask(planningContext);
    
    // Send planning step to UI
    _event.sender.send('agent:step', {
      tool: 'planner',
      summary: `Planning: ${plan.objective}`,
      status: 'done',
      planPhase: 'planning',
      data: { plan },
      requestId: `plan_${plan.id}`,
      logs: [`Created plan with ${plan.tasks.length} tasks`, `Estimated duration: ${plan.estimatedDuration}s`, `Risk level: ${plan.riskLevel}`]
    });

    // Start execution tracking
    const executionContext = agentExecutor.startExecution(plan);

    // ============================================
    // EXECUTION PHASE (existing agent loop)
    // ============================================
    agentExecutor.startPhase(plan.id, 'execution');
    
    try {
      // Run the agent loop (your existing code)
      const result = await runAgentLoop(task, model, config, workspacePath, activeFile, isAutopilotMode);
      
      // Mark execution phase as complete
      agentExecutor.endPhase(plan.id);

      // ============================================
      // NEW: SUMMARY PHASE
      // ============================================
      agentExecutor.startPhase(plan.id, 'summary');
      
      const summary = agentExecutor.getExecutionSummary(plan.id);
      
      // Send summary step to UI
      _event.sender.send('agent:step', {
        tool: 'summary',
        summary: 'Task completed',
        status: 'done',
        planPhase: 'summary',
        data: { summary },
        requestId: `summary_${plan.id}`,
        logs: ['All tasks completed successfully']
      });

      agentExecutor.endPhase(plan.id);
      agentExecutor.completeExecution(plan.id, 1.09); // creditsUsed

      return {
        response: result.finalResponse,
        steps: result.steps
      };

    } catch (err) {
      agentExecutor.endPhase(plan.id);
      throw err;
    }

  } catch (err: any) {
    console.error('Agent error:', err);
    return {
      response: `Error: ${err.message}. Check your AI provider settings.`,
      steps: []
    };
  }
});
```

## Step-by-Step Integration

### Step 1: Import Required Classes

Add at the top of `electron/main.ts`:

```typescript
import { AgentExecutor } from './agentExecutor';
import { PlanningContext } from './whizCodePlanner';
```

### Step 2: Create Executor Instance

Add after other module-level variables:

```typescript
const agentExecutor = new AgentExecutor();
```

### Step 3: Add Planning Phase

Right after workspace setup, before `runAgentLoop`:

```typescript
// Create planning context
const planningContext: PlanningContext = {
  userRequest: task,
  workspacePath,
  activeFile
};

// Generate plan
const plan = await agentExecutor.planTask(planningContext);

// Send to UI
_event.sender.send('agent:step', {
  tool: 'planner',
  summary: `Planning: ${plan.objective}`,
  status: 'done',
  planPhase: 'planning',
  data: { plan },
  requestId: `plan_${plan.id}`,
  logs: [
    `Created plan with ${plan.tasks.length} tasks`,
    `Estimated duration: ${plan.estimatedDuration}s`,
    `Risk level: ${plan.riskLevel}`
  ]
});

// Start tracking
const executionContext = agentExecutor.startExecution(plan);
agentExecutor.startPhase(plan.id, 'execution');
```

### Step 4: Wrap Agent Loop in Try-Catch

```typescript
try {
  const result = await runAgentLoop(task, model, config, workspacePath, activeFile, isAutopilotMode);
  agentExecutor.endPhase(plan.id);
  // ... continue to summary phase ...
} catch (err) {
  agentExecutor.endPhase(plan.id);
  throw err;
}
```

### Step 5: Add Summary Phase

After agent loop completes:

```typescript
agentExecutor.startPhase(plan.id, 'summary');

const summary = agentExecutor.getExecutionSummary(plan.id);

_event.sender.send('agent:step', {
  tool: 'summary',
  summary: 'Task completed',
  status: 'done',
  planPhase: 'summary',
  data: { summary },
  requestId: `summary_${plan.id}`,
  logs: ['All tasks completed successfully']
});

agentExecutor.endPhase(plan.id);
agentExecutor.completeExecution(plan.id, 1.09);
```

## Minimal Integration (Quick Start)

If you want to add planning with minimal changes:

```typescript
// At top of file
import { AgentExecutor } from './agentExecutor';
const agentExecutor = new AgentExecutor();

// In execute-agent-task handler, after workspace setup:
const plan = await agentExecutor.planTask({
  userRequest: task,
  workspacePath,
  activeFile
});

_event.sender.send('agent:step', {
  tool: 'planner',
  summary: `Planning: ${plan.objective}`,
  status: 'done',
  planPhase: 'planning',
  data: { plan },
  requestId: `plan_${plan.id}`
});

agentExecutor.startExecution(plan);
agentExecutor.startPhase(plan.id, 'execution');

// ... existing runAgentLoop code ...

agentExecutor.endPhase(plan.id);
agentExecutor.startPhase(plan.id, 'summary');

_event.sender.send('agent:step', {
  tool: 'summary',
  summary: 'Task completed',
  status: 'done',
  planPhase: 'summary',
  requestId: `summary_${plan.id}`
});

agentExecutor.endPhase(plan.id);
agentExecutor.completeExecution(plan.id);
```

## Testing the Integration

### Test 1: Verify Planning Step Shows

1. Open WhizCode
2. Ask: "Fix the login bug"
3. Check that a planning step appears with:
   - Icon: 🛠️
   - Summary: "Planning: Fix the login bug"
   - Phase badge: "📋 PLANNING"
   - Expandable plan details

### Test 2: Verify Execution Steps Show Phase

1. After planning, execution steps should appear with:
   - Phase badge: "⚙️ EXECUTION"
   - Proper tool icons
   - Status updates (running → done)

### Test 3: Verify Summary Shows

1. After execution completes, summary step should appear with:
   - Phase badge: "📊 SUMMARY"
   - Expandable summary details
   - Task list and metrics

## Debugging

### Planning step not showing?
```typescript
// Add logging
console.log('[PLANNING] Created plan:', plan);
console.log('[PLANNING] Sending step to UI');
_event.sender.send('agent:step', { /* ... */ });
```

### Phase badges not showing?
- Check that `planPhase` is set correctly ('planning', 'execution', or 'summary')
- Verify UI is receiving the step (check browser console)
- Ensure ChatPanel.tsx has been updated

### Summary not displaying?
- Verify `agentExecutor.completeExecution()` is called
- Check that summary step has `planPhase: 'summary'`
- Ensure `data: { summary }` is included

## Performance Considerations

- Planning is fast (~100-200ms) and happens before execution
- No additional API calls are made during planning
- Execution tracking adds minimal overhead
- Summary generation is instant

## Next Steps

1. Copy the integration code into your `electron/main.ts`
2. Test with a simple request
3. Verify all three phases appear in the UI
4. Customize task planning logic as needed
5. Add additional metrics to summary if desired
