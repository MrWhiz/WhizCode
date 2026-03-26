# WhizCode Agent Lifecycle

This document explains which part of WhizCode handles:

- query understanding
- planning
- repository context gathering
- execution
- verification
- completion

It reflects the current backend flow in the Tauri codebase.

## Short Answer

- The main top-level streaming agent handles query understanding, working-state creation, planning, execution, and completion.
- The `context-gatherer` sub-agent is used as a research helper to explore the repository before the main execution loop starts.

## Lifecycle Overview

1. Query analysis and workflow routing
2. Problem identification and working-state creation
3. Spec-driven planning and execution-plan creation
4. Workspace context construction
5. Research and planning with the `context-gatherer` sub-agent
6. Main execution loop
7. Verification
8. Completion or continuation

## 1. Query Analysis And Workflow Routing

This layer classifies the incoming request and decides which workflow label to use.

Primary files:

- [src-tauri/src/commands/whizcode_commands.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_commands.rs)
- [src-tauri/src/commands/whizcode_integration.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_integration.rs)

Important functions:

- [analyze_query](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_commands.rs#L14)
- [WhizCodeIntegrationLayer::analyze_query](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_integration.rs#L124)
- [route_query](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_commands.rs#L43)
- [WhizCodeIntegrationLayer::route_query](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_integration.rs#L245)

What it does:

- classifies requests into types like `feature`, `bugfix`, `refactor`, `analysis`, `spec`
- estimates confidence, complexity, and workflow
- does not execute the task itself

## 2. Problem Identification And Working State

This is where the request is converted into an execution-focused task shape.

Primary file:

- [src-tauri/src/commands/problem_identifier.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs)

Important functions:

- [ProblemIdentifier::analyze_problem](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs#L171)
- [ProblemIdentifier::build_working_state](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs#L187)
- [TaskWorkingState::record_tool_success](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs#L96)
- [TaskWorkingState::record_tool_failure](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs#L113)

What it does:

- extracts keywords and suspected files
- classifies the task kind such as `feature-implementation`, `bug-fix`, `refactoring`
- creates the initial working state
- sets the current goal and pending actions
- pushes the agent toward a small discovery pass followed by implementation

This is the first real planning layer.

## 3. Spec-Driven Planning And Execution-Plan Creation

Before research and execution begin, the streaming runtime now creates a spec-driven execution plan.

Primary files:

- [src-tauri/src/commands/planning.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/planning.rs)
- [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)

Important areas:

- [PlanningSystem::create_plan](/F:/AntiGravity/WhizCode/src-tauri/src/commands/planning.rs)
- [ExecutionPlan::to_prompt_block](/F:/AntiGravity/WhizCode/src-tauri/src/commands/planning.rs)
- streaming integration point: [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1081)

What it does:

- turns the user request into a structured spec brief
- generates acceptance criteria, assumptions, and definition of done
- breaks the work into ordered tasks with recommended owner agents
- injects the execution plan into the live task context before execution
- persists a spec artifact under `.whizcode/specs`
- converts the plan into tracked task phases under `.whizcode/tasks.json` and `.whizcode/tasks.md`

This is the main spec-driven planning layer.

## 4. Research And Planning With The `context-gatherer` Sub-Agent

Before the main execution loop starts, WhizCode can run a dedicated research helper.

Primary files:

- [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)
- [src-tauri/src/commands/prompts.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs)

Important functions:

- [run_research_phase](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L2217)
- [get_sub_agent_config("context-gatherer")](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L2230)
- [get_sub_agents](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs#L71)

Relevant sub-agent config:

- [context-gatherer](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs#L73)

What it does:

- explores the repository
- narrows relevant files before the main agent starts editing
- returns a research summary
- feeds that summary back into the main task state

This is the planning helper, but not the final decision-maker.

## 5. Workspace Context Construction

After research, the main agent builds the context bundle used for execution.

Primary file:

- [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)

Important functions:

- [build_workspace_context](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L4876)
- [build_workspace_context_parallel](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L4899)

Supporting snapshot logic:

- [src-tauri/src/commands/workspace.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/workspace.rs#L141)

What it does:

- loads workspace snapshot data when available
- includes steering, code intelligence, and repository context
- prepares the main prompt pair used to prime the execution loop

## 6. Main Execution Loop

This is the real top-level agent.

Primary file:

- [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)

Important sections:

- task setup and planning bootstrap: [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1081)
- workspace context priming: [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1216)
- main streaming loop call: [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1391)
- tool result tracking: [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1438)

What it does:

- sends the system prompt, workspace context, task state, and research findings to the LLM
- receives streamed tool calls
- tracks repetition, validation errors, and failures
- updates the working state after each executed tool

This is the component that both understands the request operationally and carries the plan forward.

## 7. Tool Identification And Execution

The streamed model output is parsed into tool calls and then executed.

Primary file:

- [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)

Important functions:

- [execute_tools_from_stream](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L3322)
- [execute_tool_with_recovery](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L3913)

What it does:

- incrementally parses streamed JSON tool output
- validates tool arguments
- executes tools sequentially, with some parallelism for independent read-only calls
- applies recovery logic on failures
- enforces additional runtime rules such as:
  - blocking bad `ask_user` calls
  - cutting off repeated file rereads
  - requiring real edits before verification/completion for implementation-heavy tasks

## 8. Verification

Verification is not handled by a separate planning agent. It is part of the top-level execution loop.

Primary logic:

- prompt rules in [src-tauri/src/commands/prompts.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs#L1)
- execution-time verification guards in [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L305)

What it does:

- asks the model to run build/test verification after edits
- blocks verification commands such as `npm run build` until a meaningful edit has happened for implementation/refactor/performance tasks

## 9. Completion

Completion is also owned by the main execution loop.

Important logic:

- `done` tool handling in [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L3784)
- raw-response completion rejection in [src-tauri/src/commands/agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs#L1567)

What it does:

- allows `done` only when the runtime believes the task is genuinely complete
- rejects premature completion for implementation-heavy tasks if no meaningful write has occurred

## Which Agent Actually Prepares The Plan?

The answer is:

- high-level query classification: `WhizCodeIntegrationLayer`
- concrete execution planning: `PlanningSystem` + `ProblemIdentifier` + top-level streaming orchestrator
- repository research support: `context-gatherer`

So if someone asks:

"Which agent understands the query and prepares the plan?"

The most accurate answer is:

- the main top-level orchestrator prepares the real execution plan
- the `context-gatherer` sub-agent only assists by gathering focused repository context

## Current Built-In Sub-Agents

Registered in:

- [src-tauri/src/commands/prompts.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs#L71)

Available:

- `context-gatherer`
- `general-task-execution`
- `custom-agent-creator`
- `code-reviewer`
- `test-engineer`
- `architect`
- `security-expert`
- `product-manager`
- `ux-designer`

## Practical Summary

If you want to reason about the live WhizCode flow, start here:

1. [whizcode_integration.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/whizcode_integration.rs)
2. [problem_identifier.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/problem_identifier.rs)
3. [agent_streaming.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/agent_streaming.rs)
4. [prompts.rs](/F:/AntiGravity/WhizCode/src-tauri/src/commands/prompts.rs)

That path covers the full chain from incoming query to finished execution.
