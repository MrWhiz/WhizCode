# WhizCode SubAgent Specifications

## Overview

WhizCode uses a hierarchical agent system with specialized SubAgents for different tasks. Each SubAgent is responsible for a specific aspect of the WhizCode behavior enhancement.

## SubAgent Architecture

### SubAgent Lifecycle

```
1. SubAgent Registration
   ├─ Define SubAgentConfig
   ├─ Add system prompt
   ├─ Register in prompts.rs
   └─ Make accessible via sub_agents.rs

2. SubAgent Invocation
   ├─ User query triggers analysis
   ├─ WhizCodeIntegrationLayer routes to appropriate SubAgent
   ├─ SubAgent receives task and context
   └─ SubAgent executes with LLM

3. SubAgent Execution
   ├─ Initialize with system prompt
   ├─ Call LLM with task
   ├─ Parse tool calls if needed
   ├─ Execute tools
   ├─ Aggregate results
   └─ Return final response

4. Result Integration
   ├─ WhizCodeIntegrationLayer receives result
   ├─ Result is processed and validated
   ├─ Result is used for next phase
   └─ Result is returned to user
```

## SubAgent Specifications

### 1. Query Analyzer SubAgent

**Purpose**: Analyze and classify user queries

**File**: `src-tauri/src/commands/agents/query_analyzer_agent.rs`

**Inputs**:
- User query string
- Optional project context

**Outputs**:
- Query type (bugfix, feature, refactor, analysis, spec)
- Confidence score (0.0-1.0)
- Extracted requirements (Vec<String>)
- Complexity assessment (simple, moderate, complex)
- Estimated duration (seconds)

**System Prompt**:
```
You are the Query Analyzer SubAgent for WhizCode.
Your role is to analyze user queries and classify them.

## Classification Rules
- Bugfix: Contains "fix", "bug", "error", "crash", "broken"
- Feature: Contains "add", "implement", "create", "new"
- Refactor: Contains "refactor", "improve", "optimize", "clean"
- Analysis: Contains "analyze", "check", "review", "understand"
- Spec: Contains "spec", "requirement", "design", "plan"

## Output Format
Return a JSON object with:
{
  "query_type": "string",
  "confidence": 0.0-1.0,
  "requirements": ["string"],
  "complexity": "simple|moderate|complex",
  "estimated_duration": number
}
```

**Workflow**:
1. Extract keywords from query
2. Determine primary intent
3. Classify query type
4. Extract key requirements
5. Assess complexity based on requirements
6. Calculate confidence score
7. Return analysis

### 2. Workflow Router SubAgent

**Purpose**: Route queries to appropriate workflows

**File**: `src-tauri/src/commands/agents/workflow_router_agent.rs`

**Inputs**:
- Query type
- Query analysis results
- Project context

**Outputs**:
- Workflow name
- Agent name
- Priority level
- Prerequisites
- Estimated duration

**System Prompt**:
```
You are the Workflow Router SubAgent for WhizCode.
Your role is to route queries to appropriate workflows.

## Workflow Mapping
- bugfix → bugfix-workflow (priority: 1)
- feature → feature-implementation (priority: 3)
- refactor → refactoring-workflow (priority: 4)
- analysis → analysis-workflow (priority: 5)
- spec → spec-creation (priority: 2)

## Output Format
Return a JSON object with:
{
  "workflow": "string",
  "agent": "string",
  "priority": number,
  "prerequisites": ["string"],
  "estimated_duration": number
}
```

**Workflow**:
1. Receive query type and analysis
2. Map query type to workflow
3. Determine agent for workflow
4. Validate prerequisites
5. Prepare workflow context
6. Return routing information

### 3. Prompt Optimizer SubAgent

**Purpose**: Generate token-efficient prompts

**File**: `src-tauri/src/commands/agents/prompt_optimizer_agent.rs`

**Inputs**:
- User query
- Query type
- Context size limit
- Available context

**Outputs**:
- Optimized system prompt
- Optimized user prompt
- Estimated token count
- Optimization metadata

**System Prompt**:
```
You are the Prompt Optimizer SubAgent for WhizCode.
Your role is to generate token-efficient prompts.

## Optimization Rules
- Be concise and direct
- Remove unnecessary context
- Use structured formats (JSON, markdown)
- Preserve essential information
- Estimate token usage accurately

## Output Format
Return a JSON object with:
{
  "system_prompt": "string",
  "user_prompt": "string",
  "estimated_tokens": number,
  "optimization_notes": "string"
}
```

**Workflow**:
1. Analyze query and context
2. Build system prompt based on query type
3. Build user prompt with essential context
4. Optimize for token efficiency
5. Estimate total token count
6. Return optimized prompts

### 4. Context Optimizer SubAgent

**Purpose**: Optimize context for local LLM

**File**: `src-tauri/src/commands/agents/context_optimizer_agent.rs`

**Inputs**:
- List of files with content
- User query
- Max token limit
- Workspace path

**Outputs**:
- Pruned file list
- Context summary
- Total token estimate
- Relevance scores

**System Prompt**:
```
You are the Context Optimizer SubAgent for WhizCode.
Your role is to optimize context for local LLM.

## Optimization Rules
- Score file relevance to query
- Include only relevant files
- Summarize large files
- Stay within token limit
- Preserve important information

## Output Format
Return a JSON object with:
{
  "files": [
    {
      "path": "string",
      "relevance_score": 0.0-1.0,
      "include": true|false,
      "type": "full|summary|snippet"
    }
  ],
  "total_tokens": number,
  "summary": "string"
}
```

**Workflow**:
1. Score each file for relevance
2. Sort by relevance score
3. Select files within token limit
4. Summarize large files
5. Generate context summary
6. Return optimized context

### 5. Bugfix Workflow SubAgent

**Purpose**: Fix bugs using bug condition methodology

**File**: `src-tauri/src/commands/agents/bugfix_workflow_agent.rs`

**Inputs**:
- Bug description
- Codebase context
- Affected files

**Outputs**:
- Bug condition identification
- Exploration test code
- Bug fix implementation
- Validation results

**System Prompt**:
```
You are the Bugfix Workflow SubAgent for WhizCode.
Your role is to fix bugs using bug condition methodology.

## Bugfix Process
1. Identify bug condition C(X)
2. Create exploration test that fails on buggy code
3. Locate root cause
4. Implement fix
5. Verify fix with test

## Output Format
Return a JSON object with:
{
  "bug_condition": "string",
  "exploration_test": "string",
  "root_cause": "string",
  "fix_implementation": "string",
  "validation_status": "passed|failed"
}
```

**Workflow**:
1. Analyze bug description
2. Identify bug condition
3. Create exploration test
4. Locate root cause in code
5. Implement fix
6. Validate fix works
7. Return fix implementation

### 6. Feature Implementation SubAgent

**Purpose**: Implement new features

**File**: `src-tauri/src/commands/agents/feature_implementation_agent.rs`

**Inputs**:
- Feature requirements
- Codebase context
- Architecture guidelines

**Outputs**:
- Feature design
- Implementation code
- Test code
- Documentation

**System Prompt**:
```
You are the Feature Implementation SubAgent for WhizCode.
Your role is to implement new features.

## Feature Implementation Process
1. Analyze requirements
2. Design architecture
3. Implement code
4. Create tests
5. Validate against requirements

## Output Format
Return a JSON object with:
{
  "design": "string",
  "implementation": "string",
  "tests": "string",
  "documentation": "string"
}
```

**Workflow**:
1. Analyze feature requirements
2. Design feature architecture
3. Implement feature code
4. Create comprehensive tests
5. Validate against requirements
6. Return implementation

### 7. Spec Creation SubAgent

**Purpose**: Create specifications and requirements

**File**: `src-tauri/src/commands/agents/spec_creation_agent.rs`

**Inputs**:
- Feature description
- Project context
- Specification template

**Outputs**:
- Requirements document
- Design document
- Task breakdown
- Correctness properties

**System Prompt**:
```
You are the Spec Creation SubAgent for WhizCode.
Your role is to create specifications.

## Specification Process
1. Gather requirements
2. Create design document
3. Break down into tasks
4. Define correctness properties
5. Create implementation plan

## Output Format
Return a JSON object with:
{
  "requirements": "string",
  "design": "string",
  "tasks": ["string"],
  "correctness_properties": ["string"]
}
```

**Workflow**:
1. Analyze feature description
2. Extract requirements
3. Create design document
4. Break down into tasks
5. Define correctness properties
6. Return specification

### 8. Refactoring SubAgent

**Purpose**: Refactor code for improvement

**File**: `src-tauri/src/commands/agents/refactoring_agent.rs`

**Inputs**:
- Code to refactor
- Improvement goals
- Codebase context

**Outputs**:
- Refactored code
- Improvement summary
- Validation results

**System Prompt**:
```
You are the Refactoring SubAgent for WhizCode.
Your role is to refactor code for improvement.

## Refactoring Process
1. Analyze code structure
2. Identify improvements
3. Implement changes
4. Ensure functionality preserved
5. Validate improvements

## Output Format
Return a JSON object with:
{
  "refactored_code": "string",
  "improvements": ["string"],
  "validation_status": "passed|failed"
}
```

**Workflow**:
1. Analyze code structure
2. Identify improvement opportunities
3. Implement refactoring
4. Ensure functionality preserved
5. Validate improvements
6. Return refactored code

### 9. Analysis SubAgent

**Purpose**: Analyze codebase and provide insights

**File**: `src-tauri/src/commands/agents/analysis_agent.rs`

**Inputs**:
- Codebase context
- Analysis query
- Project structure

**Outputs**:
- Analysis results
- Insights and patterns
- Recommendations
- Improvement suggestions

**System Prompt**:
```
You are the Analysis SubAgent for WhizCode.
Your role is to analyze codebase and provide insights.

## Analysis Process
1. Analyze codebase structure
2. Identify patterns
3. Generate insights
4. Suggest improvements
5. Provide recommendations

## Output Format
Return a JSON object with:
{
  "analysis": "string",
  "patterns": ["string"],
  "insights": ["string"],
  "recommendations": ["string"]
}
```

**Workflow**:
1. Analyze codebase structure
2. Identify patterns and issues
3. Generate insights
4. Suggest improvements
5. Provide recommendations
6. Return analysis results

## SubAgent Registration

All SubAgents must be registered in `src-tauri/src/commands/prompts.rs`:

```rust
pub fn get_sub_agents() -> Vec<SubAgentConfig> {
    vec![
        SubAgentConfig {
            name: "query-analyzer".to_string(),
            description: "Analyzes and classifies user queries".to_string(),
            system_prompt: QUERY_ANALYZER_PROMPT.to_string(),
        },
        SubAgentConfig {
            name: "workflow-router".to_string(),
            description: "Routes queries to appropriate workflows".to_string(),
            system_prompt: WORKFLOW_ROUTER_PROMPT.to_string(),
        },
        // ... more agents
    ]
}
```

## SubAgent Invocation

SubAgents are invoked through the existing SubAgentExecutor:

```rust
let executor = SubAgentExecutor::new();
let result = executor.execute_sub_agent(
    "query-analyzer".to_string(),
    "Fix the bug where app crashes on zero quantity".to_string(),
    Some(workspace_path)
).await?;
```

## SubAgent Communication

SubAgents communicate through:

1. **Input**: Task description and context
2. **Processing**: LLM call with system prompt
3. **Output**: JSON-formatted results
4. **Integration**: Results parsed and used by WhizCodeIntegrationLayer

## Error Handling

Each SubAgent must handle:

- Invalid input gracefully
- LLM connection failures
- Malformed output
- Timeout scenarios
- Fallback to default behavior

## Testing SubAgents

Each SubAgent must be tested for:

- Accuracy of classification/analysis
- Token efficiency
- Response time (< 5 seconds)
- Error handling
- Integration with other agents
- Backward compatibility

## Success Criteria

- All 9 SubAgents implemented and registered
- Each SubAgent passes accuracy tests
- SubAgent response time < 5 seconds
- SubAgent output format is consistent
- SubAgents integrate seamlessly with WhizCodeIntegrationLayer
- No breaking changes to existing functionality
