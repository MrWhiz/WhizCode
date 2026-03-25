# Task Management Integration - Design

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Orchestrator                        │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ execute_task()                                       │   │
│  │  1. Create/Load tasks.md                             │   │
│  │  2. Initialize context memory                        │   │
│  │  3. Initialize hooks manager                         │   │
│  │  4. Run agent loop                                   │   │
│  │  5. Update task status on completion                 │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Agent Streaming                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ execute_tool_with_recovery()                         │   │
│  │  1. Execute tool                                     │   │
│  │  2. On failure: Call error recovery                  │   │
│  │  3. Record recovery attempt                          │   │
│  │  4. Fall back to LLM recovery if needed              │   │
│  │  5. Record result in context memory                  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Infrastructure Systems                    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │ Task Manager │ │Error Recovery│ │Context Memory│         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
│  ┌──────────────┐ ┌──────────────┐                          │
│  │Hooks Manager │ │Graph Service │                          │
│  └──────────────┘ └──────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

## Integration Flow Diagrams

### Flow 1: Task Lifecycle

```
Agent Orchestrator
    ↓
[1] Create/Load tasks.md
    ├─ TaskManager::create_tasks_file()
    └─ TaskManager::load_tasks_file()
    ↓
[2] Initialize Infrastructure
    ├─ ContextMemory::new()
    ├─ HooksManager::new()
    ├─ ErrorRecoverySystem::new()
    └─ GraphService::new()
    ↓
[3] Run Agent Loop
    ├─ For each task:
    │  ├─ TaskManager::update_task_status(InProgress)
    │  ├─ Execute task
    │  ├─ TaskManager::update_task_status(Completed)
    │  └─ Record result
    └─ Continue
    ↓
[4] Finalize
    ├─ TaskManager::save_tasks_file()
    └─ ContextMemory::persist()
```

### Flow 2: Error Recovery

```
Agent Streaming
    ↓
[1] Execute Tool
    ├─ Call tool
    └─ Capture result
    ↓
[2] Check Result
    ├─ Success? → Record in context memory → Continue
    └─ Failure? → Go to [3]
    ↓
[3] Attempt Recovery
    ├─ ErrorRecoverySystem::execute_recovery_strategy()
    ├─ Retry tool with recovery
    └─ Check result
    ↓
[4] Recovery Outcome
    ├─ Success? → Record recovery → Continue
    └─ Failure? → Go to [5]
    ↓
[5] Fall Back to LLM
    ├─ Add error to context
    ├─ Ask LLM for recovery
    └─ Continue with LLM suggestion
```

### Flow 3: Context Memory

```
During Execution
    ↓
[1] Record Patterns
    ├─ ContextMemory::record_code_pattern()
    ├─ ContextMemory::record_error_pattern()
    └─ ContextMemory::record_successful_strategy()
    ↓
[2] Query Context
    ├─ ContextMemory::get_relevant_code_patterns()
    ├─ ContextMemory::get_similar_error_patterns()
    └─ ContextMemory::get_best_strategies()
    ↓
[3] Use Context
    ├─ Inform LLM of patterns
    ├─ Suggest strategies
    └─ Optimize execution
    ↓
[4] Persist
    └─ ContextMemory::persist()
```

### Flow 4: Hooks

```
Event Occurs
    ↓
[1] Trigger Hook
    ├─ HooksManager::trigger_file_event() (file events)
    ├─ HooksManager::trigger_event() (execution events)
    └─ HooksManager::trigger_task_event() (task events)
    ↓
[2] Find Matching Hooks
    ├─ Match event type
    ├─ Match file patterns
    └─ Filter enabled hooks
    ↓
[3] Execute Hook Actions
    ├─ askAgent: Send message to agent
    └─ runCommand: Execute shell command
    ↓
[4] Record Execution
    └─ HooksManager::record_execution()
```

### Flow 5: Graph Service

```
Code Analysis
    ↓
[1] Build Graph
    ├─ GraphService::build_dependency_graph()
    ├─ Identify nodes (files, functions, classes)
    └─ Identify edges (dependencies)
    ↓
[2] Analyze Graph
    ├─ GraphService::analyze_dependencies()
    ├─ GraphService::get_circular_dependencies()
    └─ GraphService::get_dependency_chains()
    ↓
[3] Use Graph
    ├─ Suggest optimizations
    ├─ Warn about circular deps
    └─ Recommend refactoring
    ↓
[4] Track Changes
    └─ GraphService::update_graph()
```

## Data Structures

### Task State
```rust
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Queued,
}
```

### Recovery State
```rust
pub struct RecoveryAttempt {
    pub error_type: String,
    pub strategy_id: String,
    pub success: bool,
    pub timestamp: u64,
    pub execution_time_ms: u32,
}
```

### Context State
```rust
pub struct CodePattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: u32,
    pub last_seen: u64,
}

pub struct ErrorPattern {
    pub error_type: String,
    pub frequency: u32,
    pub best_strategy: Option<String>,
    pub last_seen: u64,
}

pub struct SuccessfulStrategy {
    pub strategy: String,
    pub task_type: String,
    pub success_rate: f32,
    pub last_used: u64,
}
```

### Hook State
```rust
pub struct Hook {
    pub id: String,
    pub name: String,
    pub event_type: String,
    pub patterns: Vec<String>,
    pub action: HookAction,
    pub enabled: bool,
}

pub enum HookAction {
    AskAgent(String),
    RunCommand(String),
}
```

### Graph State
```rust
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub circular_deps: Vec<Vec<String>>,
}

pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub file_path: String,
}

pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}
```

## Implementation Strategy

### Phase 2.1: Task Management Integration
1. Update `agent_orchestrator.rs::execute_task()` to:
   - Create/load tasks.md at start
   - Initialize TaskManager
   - Update task status during execution
   - Save tasks.md on completion

2. Update `agent_streaming.rs::execute_tool_with_recovery()` to:
   - Record tool execution in task context
   - Update task progress

### Phase 2.2: Error Recovery Integration
1. Update `agent_streaming.rs::execute_tool_with_recovery()` to:
   - Catch tool failures
   - Call ErrorRecoverySystem::execute_recovery_strategy()
   - Record recovery attempts
   - Fall back to LLM recovery

2. Create error recovery hooks in:
   - Tool execution failure handler
   - Recovery attempt tracker

### Phase 2.3: Context Memory Integration
1. Update `learning.rs::record_interaction()` to:
   - Call ContextMemory::record_code_pattern()
   - Call ContextMemory::record_error_pattern()
   - Call ContextMemory::record_successful_strategy()

2. Update agent loop to:
   - Query ContextMemory for relevant patterns
   - Pass patterns to LLM
   - Use patterns for optimization

### Phase 2.4: Hooks Integration
1. Create file watchers to:
   - Detect file changes
   - Trigger HooksManager::trigger_file_event()

2. Update agent execution to:
   - Trigger HooksManager::trigger_event() on events
   - Execute hook actions

3. Create task event triggers:
   - Trigger on task start
   - Trigger on task completion
   - Trigger on task failure

### Phase 2.5: Graph Service Integration
1. Update code intelligence to:
   - Call GraphService::build_dependency_graph()
   - Call GraphService::analyze_dependencies()
   - Use graphs for optimization

2. Create graph visualization:
   - Display dependency graphs in UI
   - Show circular dependencies
   - Suggest optimizations

## Error Handling

### Task Management Errors
- File I/O errors: Log and continue with in-memory state
- Status update errors: Retry with exponential backoff
- Persistence errors: Queue for retry

### Error Recovery Errors
- Recovery strategy not found: Fall back to LLM recovery
- Recovery execution timeout: Escalate to LLM
- Recovery failure: Record and continue

### Context Memory Errors
- Memory full: Evict old entries
- Concurrent access: Use locks
- Persistence errors: Queue for retry

### Hooks Errors
- Hook execution timeout: Log and continue
- Hook action failure: Record and continue
- Hook matching errors: Log and skip

### Graph Service Errors
- Graph build failure: Log and continue
- Circular dependency detection timeout: Log and continue
- Graph persistence errors: Queue for retry

## Testing Strategy

### Unit Tests
- Task status transitions
- Error recovery strategies
- Context memory operations
- Hook matching and execution
- Graph building and analysis

### Integration Tests
- Task lifecycle end-to-end
- Error recovery with tool execution
- Context memory with learning system
- Hooks with file watchers
- Graph service with code intelligence

### Error Scenario Tests
- Tool execution failures
- Recovery strategy failures
- Memory full scenarios
- Concurrent access scenarios
- File I/O errors

## Performance Considerations

### Task Management
- Lazy load tasks.md
- Cache task state in memory
- Batch status updates
- Async persistence

### Error Recovery
- Cache recovery strategies
- Parallel recovery attempts
- Timeout on recovery
- Fallback to LLM quickly

### Context Memory
- Limit memory size
- Evict old entries
- Index for fast queries
- Async persistence

### Hooks
- Async hook execution
- Timeout on hook actions
- Non-blocking event triggers
- Batch hook execution

### Graph Service
- Incremental graph updates
- Cache graph analysis
- Lazy circular dependency detection
- Async graph building

## Rollback Plan

If integration fails:
1. Disable task management (use in-memory only)
2. Disable error recovery (use LLM recovery only)
3. Disable context memory (use fresh context)
4. Disable hooks (use manual triggers)
5. Disable graph service (use basic analysis)

All systems have fallbacks to ensure agent continues working.
