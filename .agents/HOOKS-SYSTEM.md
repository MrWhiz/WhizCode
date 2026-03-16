# Hooks System

## Overview
The Hooks System provides event-driven automation for WhizCode, similar to Kiro's hooks. Hooks allow you to automatically trigger actions when specific events occur.

## Hook Types

### Event Types

#### File Events
- **fileEdited**: Triggered when a file is modified
- **fileCreated**: Triggered when a new file is created
- **fileDeleted**: Triggered when a file is deleted

#### Agent Events
- **promptSubmit**: Triggered when user sends a message to agent
- **agentStop**: Triggered when agent completes execution

#### Tool Events
- **preToolUse**: Triggered before a tool is executed
- **postToolUse**: Triggered after a tool is executed

#### Task Events
- **preTaskExecution**: Triggered before a task starts
- **postTaskExecution**: Triggered after a task completes

#### Manual Events
- **userTriggered**: Manually triggered by user

### Action Types

#### askAgent
Sends a prompt to the agent to remind it of something or request an action.

**Example:**
```json
{
  "action": "askAgent",
  "prompt": "Review this code change for security issues"
}
```

#### runCommand
Executes a shell command.

**Example:**
```json
{
  "action": "runCommand",
  "command": "npm run lint",
  "timeout": 30
}
```

## Hook Structure

```json
{
  "id": "unique-hook-id",
  "name": "Hook Name",
  "description": "What this hook does",
  "enabled": true,
  "eventType": "fileEdited",
  "filePatterns": ["*.ts", "*.tsx"],
  "toolTypes": ["write"],
  "action": "runCommand",
  "command": "npm run lint",
  "timeout": 30
}
```

### Fields

- **id**: Unique identifier (kebab-case)
- **name**: Display name
- **description**: What the hook does
- **enabled**: Whether the hook is active
- **eventType**: When to trigger (see Event Types)
- **filePatterns**: File patterns to match (for file events)
- **toolTypes**: Tool categories or regex (for tool events)
- **action**: What to do (askAgent or runCommand)
- **prompt**: Message for askAgent
- **command**: Command for runCommand
- **timeout**: Timeout in seconds for runCommand

## Tool Type Categories

### Built-in Categories
- **read**: File reading operations
  - read_file, readCode, readMultipleFiles, list_directory, search_files, grepSearch, fileSearch
- **write**: File writing operations
  - write_file, edit_file, editCode, delete_file, strReplace, smartRelocate
- **shell**: Shell commands
  - run_command
- **web**: Web operations
  - remote_web_search, webFetch
- **spec**: Spec operations
  - createSpec, updateSpec
- **\***: All tools

### Regex Patterns
You can also use regex patterns to match tool names:
- `.*sql.*` - Matches any tool with "sql" in the name
- `^read.*` - Matches tools starting with "read"

## Example Hooks

### 1. Lint on Save
```json
{
  "id": "lint-on-save",
  "name": "Lint on Save",
  "description": "Run linter when TypeScript files are edited",
  "enabled": true,
  "eventType": "fileEdited",
  "filePatterns": ["*.ts", "*.tsx"],
  "action": "runCommand",
  "command": "npm run lint",
  "timeout": 30
}
```

### 2. Review Write Operations
```json
{
  "id": "review-writes",
  "name": "Review Write Operations",
  "description": "Ask agent to verify write operations",
  "enabled": true,
  "eventType": "preToolUse",
  "toolTypes": ["write"],
  "action": "askAgent",
  "prompt": "Verify this write operation follows our coding standards."
}
```

### 3. Test After Task
```json
{
  "id": "test-after-task",
  "name": "Run Tests After Task",
  "description": "Run tests when agent completes a task",
  "enabled": true,
  "eventType": "postTaskExecution",
  "action": "runCommand",
  "command": "npm test",
  "timeout": 60
}
```

### 4. Format on Create
```json
{
  "id": "format-on-create",
  "name": "Format New Files",
  "description": "Format newly created JavaScript/TypeScript files",
  "enabled": true,
  "eventType": "fileCreated",
  "filePatterns": ["*.js", "*.ts", "*.jsx", "*.tsx"],
  "action": "runCommand",
  "command": "npx prettier --write",
  "timeout": 10
}
```

## File Location

Hooks are stored in:
```
.kiro/hooks/
  ├── lint-on-save.json
  ├── review-writes.json
  └── test-after-task.json
```

Each hook is a separate JSON file named `{hook-id}.json`.

## Usage

### Via IPC (Programmatic)

```javascript
// List all hooks
const hooks = await ipcRenderer.invoke('hooks:list');

// Get specific hook
const hook = await ipcRenderer.invoke('hooks:get', 'lint-on-save');

// Save hook
await ipcRenderer.invoke('hooks:save', {
  id: 'my-hook',
  name: 'My Hook',
  // ... other fields
});

// Delete hook
await ipcRenderer.invoke('hooks:delete', 'my-hook');

// Reload hooks from disk
await ipcRenderer.invoke('hooks:reload');
```

### Via File System

1. Create a JSON file in `.kiro/hooks/`
2. Name it `{hook-id}.json`
3. Add hook configuration
4. Reload hooks or restart app

## Hook Execution Flow

### File Event Flow
```
File Changed
    │
    ▼
Check enabled hooks for event type
    │
    ▼
Match file patterns
    │
    ▼
Execute matching hooks
    │
    ├─► askAgent ──► Send prompt to agent
    │
    └─► runCommand ─► Execute shell command
```

### Tool Event Flow
```
Tool About to Execute (preToolUse)
    │
    ▼
Check enabled hooks
    │
    ▼
Match tool types/patterns
    │
    ▼
Execute matching hooks
    │
    ├─► askAgent ──► Agent reviews operation
    │
    └─► runCommand ─► Run validation command
    │
    ▼
Proceed with tool execution
```

## Best Practices

### Performance
- Use specific file patterns to avoid unnecessary triggers
- Set reasonable timeouts for commands
- Disable hooks you're not using

### Security
- Be careful with runCommand hooks
- Validate command strings
- Use timeouts to prevent hanging

### Organization
- Use descriptive IDs and names
- Group related hooks
- Document what each hook does

## Advanced Features

### Circular Dependency Detection
PreToolUse hooks can create infinite loops. The system detects and prevents this:

```
Hook A requires Tool X
    │
    ▼
Tool X triggers Hook A again
    │
    ▼
CIRCULAR PATTERN DETECTED
    │
    ▼
Skip nested hook invocation
```

### Access Control
PreToolUse hooks can be used for access control:

```json
{
  "id": "require-approval",
  "name": "Require Approval for Deletions",
  "eventType": "preToolUse",
  "toolTypes": ["delete_file"],
  "action": "askAgent",
  "prompt": "Confirm this file deletion is safe and necessary."
}
```

If the hook output indicates denial, the tool call is blocked.

## Future Enhancements

### Planned
- UI for managing hooks
- Hook templates
- Hook marketplace
- Conditional execution (if/then logic)
- Hook chaining
- Hook priorities

### Possible
- Async hooks
- Hook debugging
- Hook analytics
- Custom hook actions
- Hook versioning

## Comparison with Kiro

| Feature | WhizCode | Kiro |
|---------|----------|------|
| File Events | ✅ | ✅ |
| Tool Events | ✅ | ✅ |
| Task Events | ✅ | ✅ |
| askAgent Action | ✅ | ✅ |
| runCommand Action | ✅ | ✅ |
| File Patterns | ✅ | ✅ |
| Tool Categories | ✅ | ✅ |
| Regex Patterns | ✅ | ✅ |
| UI Management | ⏳ Planned | ✅ |
| Hook Templates | ⏳ Planned | ✅ |

## Implementation Status

✅ Core hooks system
✅ Event detection
✅ File pattern matching
✅ Tool type matching
✅ Hook storage (JSON files)
✅ IPC handlers
⏳ Hook execution (TODO)
⏳ UI for management (TODO)
⏳ Hook templates (TODO)
