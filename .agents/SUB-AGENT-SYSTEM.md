# Sub-Agent System Implementation

## Overview
WhizCode now has a Kiro-like sub-agent system that allows the main agent to delegate specialized tasks to focused sub-agents.

## Available Sub-Agents

### 1. context-gatherer
**Purpose**: Analyzes repository structure to identify relevant files and content sections

**When to use**:
- Starting work on an unfamiliar codebase
- Investigating bugs across multiple files
- Understanding how components interact
- Finding relevant files for a feature

**Example**:
```json
{
  "tool": "invokeSubAgent",
  "agentName": "context-gatherer",
  "task": "Find all files related to user authentication and login flow"
}
```

### 2. general-task-execution
**Purpose**: General-purpose sub-agent for executing delegated tasks

**When to use**:
- Delegating well-defined subtasks
- Parallelizing independent work
- Isolating context for specific features

**Example**:
```json
{
  "tool": "invokeSubAgent",
  "agentName": "general-task-execution",
  "task": "Add error handling to all API calls in the services directory"
}
```

### 3. custom-agent-creator
**Purpose**: Creates and configures new specialized agents

**When to use**:
- User wants a new specialized agent
- Recurring task patterns need automation
- Custom workflows need dedicated agents

**Example**:
```json
{
  "tool": "invokeSubAgent",
  "agentName": "custom-agent-creator",
  "task": "Create an agent specialized in writing unit tests for React components"
}
```

## How It Works

1. **Main agent** identifies a task suitable for delegation
2. **Invokes sub-agent** using the `invokeSubAgent` tool
3. **Sub-agent runs** with its specialized system prompt and tools
4. **Results returned** to main agent for integration
5. **Main agent continues** with the sub-agent's findings

## Key Features

- **Specialized prompts**: Each sub-agent has a focused system prompt
- **Iteration limits**: Sub-agents have appropriate iteration limits (8-15)
- **Recursion prevention**: Sub-agents cannot invoke other sub-agents
- **Full tool access**: Sub-agents can use all available tools
- **Independent context**: Each sub-agent maintains its own conversation

## Architecture

```
Main Agent
    ├── Uses general system prompt
    ├── Can invoke sub-agents
    └── Integrates sub-agent results

Sub-Agents
    ├── context-gatherer (exploration & analysis)
    ├── general-task-execution (implementation)
    └── custom-agent-creator (agent design)
```

## Benefits

1. **Modularity**: Specialized agents for specific tasks
2. **Focus**: Each agent has a clear, limited scope
3. **Efficiency**: Appropriate iteration limits prevent waste
4. **Clarity**: Clear separation of concerns
5. **Extensibility**: Easy to add new specialized agents

## Adding New Sub-Agents

To add a new sub-agent, edit `electron/subAgents.ts`:

```typescript
'my-new-agent': {
  name: 'my-new-agent',
  description: 'Brief description of what this agent does',
  maxIterations: 10,
  systemPrompt: `Your specialized system prompt here...`
}
```

## Usage Tips

- Use `context-gatherer` at the start of complex tasks
- Delegate independent subtasks to `general-task-execution`
- Let sub-agents complete their work before continuing
- Trust sub-agent results - they're specialized for their tasks
