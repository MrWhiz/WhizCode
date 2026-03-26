# WhizCode Performance Optimization Plan

This file tracks the highest-value optimization work to make WhizCode smoother, faster, and more responsive in day-to-day coding tasks.

Status key:
- `[x]` Completed
- `[-]` In progress / partially completed
- `[ ]` Not started

## Goals

- Reduce unnecessary LLM round trips.
- Reduce repeated repo exploration and file reading.
- Improve perceived responsiveness during long-running tasks.
- Make the UI feel live without excessive rerender churn.
- Add enough telemetry to measure where time is actually being spent.

## Priority 1: Execution Flow

- `[x]` Introduce structured task working state
  - Store and reuse a compact execution state per run:
    - current goal
    - suspected files
    - completed checks
    - pending actions
    - blockers
  - Avoid rebuilding intent from full conversation history on every iteration.
  - Persist and reload the snapshot from `.whizcode/task_state.json` when the task fingerprint matches.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs`

- `[x]` Plan once, execute many
  - Read-only research tools can already run in parallel in the research phase.
  - Extend the same strategy into the main loop for safe, non-mutating tool groups.
  - Only re-query the LLM when a real decision point is reached.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`

- `[x]` Reduce unnecessary replanning
  - Reuse previous findings when the task has not materially changed.
  - Skip redundant context rebuilding between adjacent iterations.
  - Reuse the compact task working state and research summary on matching reruns.
  - Short-circuit known-success flows such as:
    - targeted file fix
    - rerun verification
    - commit-ready summarization

## Priority 2: Context and Search

- `[x]` Add persisted per-workspace context snapshot
  - Persisted `.whizcode/context_snapshot.json` per workspace.
  - Loaded the snapshot at task start and reused it in the workspace primer and issue-focus hints.
  - Refreshed the snapshot from live semantic context during the workspace context build.
  - Keep a reusable summary of:
    - key files
    - symbols
    - module relationships
    - recent edits
    - recent investigations
  - Load it at task start instead of rediscovering the workspace each time.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/context_memory.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/workspace.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/code_intelligence.rs`

- `[x]` Use semantic search first for vague tasks
  - Prompt bias and task guidance now prefer `semantic_search` before broad file reads.
  - Next step is to enforce this more consistently in execution heuristics.
  - Fall back to `find_symbols` when the identifier is known.
  - Use narrow `read_file` only after the likely file is identified.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`

- `[x]` Incremental vector and symbol refresh
  - Vector search now keeps per-file chunk caches and refreshes only touched files on edits.
  - Code intelligence now refreshes symbols and related local graph edges for edited or removed files.
  - File-edit tool execution now triggers incremental cache refreshes instead of waiting for a full workspace rebuild.
  - Refresh only changed files on file events.
  - Maintain a lightweight symbol/dependency cache beside the vector store.
  - Promote recently relevant files into future planning.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/vector_search.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/code_intelligence.rs`
    - `/F:/AntiGravity/WhizCode/src/hooks/useAppEventListeners.ts`

## Priority 3: UI Smoothness

- `[ ]` Throttle frontend live updates
  - Keep backend event streaming fast.
  - Batch frontend state commits to a small cadence such as 100-200ms.
  - Reduce unnecessary rerenders from rapid log, thought, and metric updates.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src/App.tsx`
    - `/F:/AntiGravity/WhizCode/src/hooks/useAppEventListeners.ts`
    - `/F:/AntiGravity/WhizCode/src/components/Chat/ChatPanel.tsx`

- `[x]` Separate thought stream from step state
  - Split the archived chat history, live agent activity, and thought/metrics bar into memoized render subtrees.
  - Kept live stream updates local so token flushes no longer invalidate the whole chat history tree.
  - Treat thoughts/status as a lightweight stream.
  - Treat tool steps as a slower snapshot-oriented stream.
  - Avoid rerendering the full step tree for every partial thought token.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src/components/Chat/ChatPanel.tsx`
    - `/F:/AntiGravity/WhizCode/src/components/Chat/StreamingDisplay.tsx`
    - `/F:/AntiGravity/WhizCode/src/hooks/useAppEventListeners.ts`

- `[x]` Virtualize or cap long log rendering
  - Collapsed long log blocks in the chat transcript with an expand control.
  - Capped terminal backlog rendering to the newest lines so oversized outputs stay responsive.
  - Preserve full content in history/state without painting every line at once.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src/components/Chat/ChatPanel.tsx`

## Priority 4: LLM Efficiency

- `[x]` Reduce prompt weight
  - Added a prompt budget for workspace priming so optional sections only land when there is room.
  - Kept the workspace snapshot to a single injection instead of duplicating it in the primer.
  - Trimmed dynamic suffix, knowledge, workflows, git, and metrics blocks when the prompt is already heavy.

- `[x]` Route by task type
  - Lightweight task kinds now skip expensive workspace extras and run fewer research iterations.
  - Bug-fix, refactoring, feature, and performance tasks keep the fuller context path and longer research loop.
    - repair loops
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/ai.rs`

- `[x]` Improve stream cadence
  - Token and thought streaming are batched in the backend and throttled in the UI.
  - Continue reducing hidden “silent work” periods where the UI looks stalled.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`
    - `/F:/AntiGravity/WhizCode/src/components/Chat/StreamingDisplay.tsx`

## Priority 5: Observability

- `[ ]` Add phase-level performance telemetry
  - Show:
    - planning time
    - tool execution time
    - verification time
    - total task duration
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`
    - `/F:/AntiGravity/WhizCode/src/components/Chat/ChatPanel.tsx`

- `[x]` Surface prompt truncation diagnostics
  - Backend now emits prompt inclusion and omission counts.
  - The chat panel surfaces the latest truncation snapshot during a run.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`
    - `/F:/AntiGravity/WhizCode/src/components/Chat/ChatPanel.tsx`

- `[ ]` Track tool-usage effectiveness
  - Measure:
    - semantic search usage frequency
    - file reads per task
    - repeated retries
    - repair loop count
    - success after first plan
  - Use the results to tune planner heuristics.
  - Likely areas:
    - `/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs`

## Recommended Delivery Order

1. Structured task working state
2. Main-loop parallel safe execution
3. Frontend live-update throttling
4. Persisted context snapshot
5. Prompt-weight reduction
6. Phase and tool telemetry
7. Incremental vector/symbol refresh
8. Task-type-based model routing

## Success Metrics

- Fewer file reads per task for targeted bug fixes.
- Lower average time to first visible progress.
- Lower average time to first tool execution.
- Lower average end-to-end task duration.
- Fewer repeated iterations before convergence.
- Reduced frequency of “silent” periods where users think the agent is hung.
