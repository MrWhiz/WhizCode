# AI IDE Capability Roadmap

This file tracks WhizCode's progress toward a top-tier AI coding IDE.

Status key:
- `[x]` Completed
- `[-]` Partially completed / usable but not mature
- `[ ]` Not completed

## Must Have

- `[x]` Execution reliability basics
  - Live step merging preserves richer task state and completed command logs.
- `[-]` Durable task state
  - Workspace chat/task history now persists to `.whizcode/history` and reloads on reopen.
  - Still missing multi-thread history UX and richer task resume flows.
- `[-]` Verification loop
  - WhizCode now runs an automatic lightweight review pass after code-changing tasks and attaches reviewer findings to the completed task.
  - It now performs one bounded automatic repair pass from review findings and re-runs verification afterward.
  - Still missing deeper multi-pass orchestration and smarter fix selection.
- `[-]` Structured failure recovery
  - Recovery services and strategy tracking exist.
  - Review findings now produce structured recovery guidance automatically.
  - One bounded repair pass is now wired into the main task flow.
  - Still missing richer recovery strategy selection and backend-owned retry policies.
- `[-]` Approval and sandbox hardening
  - Permission prompts exist and high-risk commands now disable auto-run countdown.
  - High-risk `run_command` requests now require explicit backend approval before execution.
  - Still missing reusable approval policies and broader non-command policy enforcement.
- `[-]` Review workflow
  - Source Control can now run a lightweight changed-file review and show findings.
  - Still missing a full findings-first review mode with inline comments/severity/confidence workflow.
- `[-]` Git workflow
  - Real git status, stage, and commit are implemented in the IDE.
  - Still missing branch, diff, rollback, PR-oriented summaries, and safer advanced git flows.

## Should Have

- `[-]` Parallel sub-agent orchestration with ownership
  - Sub-agent orchestration now supports delegated work items with explicit owners and owned file scopes.
  - Parallel execution and result aggregation are implemented, but integration into the main planning/execution loop is still missing.
- `[-]` Stronger workspace/context grounding
  - Code intelligence now refreshes on workspace changes instead of blindly reusing stale analysis.
  - Semantic search honors file filters, supports incremental file updates, and research prompts now inject grounding summaries with files/symbols/index coverage.
  - Still needs deeper symbol-ranking and tighter context selection inside every task iteration.
- `[ ]` Rich inline editor UX for AI suggestions and code review
- `[-]` Repeatable workflows / SOPs
  - Workflow infrastructure exists, but end-user execution UX needs polish.
- `[-]` Better memory model
  - Context memory commands now read and write real shared state instead of placeholders.
  - Snapshot/inspection and targeted deletion for preferences/projects are available for better trust and hygiene.
  - Still needs persistence across app restarts and a stronger end-user inspection UI.
- `[-]` Source-aware research discipline
  - Research prompts now enforce local-first evidence gathering and explicit separation of external sources.
  - Web search results include source metadata such as domain and retrieval time.
  - Still needs frontend citation UX and stronger policy enforcement for external research usage.
- `[ ]` Team collaboration artifacts and sharing

## Nice To Have

- `[ ]` Scheduled automations / recurring maintenance tasks
- `[-]` Plugin and tool ecosystem maturity
  - MCP/tool surfaces exist, but trust/versioning/discoverability need work.
- `[ ]` Advanced monorepo / multi-workspace intelligence
- `[ ]` Benchmarking and reliability dashboard
- `[ ]` Distinct pairing modes (executor, teacher, reviewer, planner)

## Recent Progress

- Preserved completed task logs in chat history.
- Persisted workspace chat/task history and reload on workspace reopen.
- Replaced stub git status with real git status/stage/commit support.
- Added lightweight review findings for changed files in Source Control.
- Disabled permission auto-run for clearly high-risk commands.
- Added automatic post-task verification summaries and reviewer/recovery steps for code-changing tasks.
- Added one bounded automatic repair pass after failed verification findings.
- Enforced explicit backend approval for high-risk command execution.
- Added ownership-aware sub-agent orchestration primitives for delegated parallel work.
- Made context memory commands stateful and added snapshot/delete operations for inspection and cleanup.
- Improved freshness-aware workspace grounding and added source metadata to external research results.
