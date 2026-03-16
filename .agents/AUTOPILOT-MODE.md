# Autopilot vs Supervised Mode

## Overview
WhizCode now supports two operation modes that control how the agent interacts with your files, similar to WhizCode's autonomy levels.

## Modes

### 🚀 Autopilot Mode (Autonomous)
**When enabled:**
- Agent can modify files without asking for permission
- File operations (write, edit, delete) execute immediately
- Faster workflow for trusted operations
- Best for: Experienced users, trusted tasks, rapid development

**File operations that run automatically:**
- `write_file` - Creating or overwriting files
- `edit_file` - Making targeted edits
- `delete_file` - Removing files
- `editCode` - AST-based code edits
- `strReplace` - String replacements
- `smartRelocate` - Moving files

**Still requires approval:**
- `run_command` - Terminal commands always require approval for security

### 🛡️ Supervised Mode (Default)
**When enabled:**
- Agent asks for permission before modifying files
- You review each file operation before it executes
- Safer for unfamiliar code or critical files
- Best for: Learning, critical projects, untrusted operations

**File operations that require approval:**
- `write_file` - Shows which file will be created/modified
- `edit_file` - Shows which file and how many edits
- `delete_file` - Shows which file will be deleted

**Always requires approval:**
- `run_command` - Terminal commands (in both modes)

## How to Toggle

### In the UI
1. Click the settings icon in the chat panel
2. Find "Agent Mode" section
3. Check/uncheck "Autopilot Mode"
4. Setting is saved automatically

### Indicator
The checkbox shows the current mode:
- ✅ Checked = 🚀 Autopilot (autonomous)
- ☐ Unchecked = 🛡️ Supervised (safe)

## Permission Flow

### Supervised Mode Flow
```
Agent wants to write file
    │
    ▼
Request permission
    │
    ├─► User approves ──► Execute operation
    │
    └─► User denies ───► Abort, inform agent
```

### Autopilot Mode Flow
```
Agent wants to write file
    │
    ▼
Execute immediately
    │
    ▼
Continue to next action
```

## Examples

### Example 1: Creating a New File

**Supervised Mode:**
```
Agent: I'll create the new component file
[Permission popup appears]
"Write file: src/components/NewComponent.tsx"
[Approve] [Deny]

User clicks [Approve]
Agent: ✅ Successfully wrote 45 lines to src/components/NewComponent.tsx
```

**Autopilot Mode:**
```
Agent: I'll create the new component file
Agent: ✅ Successfully wrote 45 lines to src/components/NewComponent.tsx
(No interruption)
```

### Example 2: Editing Multiple Files

**Supervised Mode:**
```
Agent: I'll update the imports in 3 files

[Permission 1/3] Edit file: src/App.tsx (2 edits)
User: [Approve]

[Permission 2/3] Edit file: src/utils/helpers.ts (1 edit)
User: [Approve]

[Permission 3/3] Edit file: src/types/index.ts (1 edit)
User: [Approve]

Agent: All files updated successfully
```

**Autopilot Mode:**
```
Agent: I'll update the imports in 3 files
Agent: ✅ Edited src/App.tsx (2 edits)
Agent: ✅ Edited src/utils/helpers.ts (1 edit)
Agent: ✅ Edited src/types/index.ts (1 edit)
Agent: All files updated successfully
(No interruptions)
```

## Best Practices

### When to Use Autopilot Mode
✅ Working on personal projects
✅ Prototyping and experimentation
✅ Trusted, well-understood tasks
✅ Repetitive refactoring operations
✅ When you trust the agent's decisions

### When to Use Supervised Mode
✅ Working on production code
✅ Unfamiliar codebases
✅ Critical or sensitive files
✅ Learning how the agent works
✅ When you want full control

### Hybrid Approach
You can toggle between modes during a session:
1. Start in Supervised mode to understand the agent's approach
2. Switch to Autopilot once you trust the pattern
3. Switch back to Supervised for critical operations

## Safety Features

### Always Safe
- **Undo available**: Use git to revert changes
- **File watching**: Changes appear immediately in editor
- **Validation**: TypeScript/ESLint errors shown in context
- **Abort option**: Stop agent execution anytime

### Additional Protection
- Terminal commands always require approval (both modes)
- Permission state persists across sessions
- Clear indicators show which mode is active

## Technical Details

### Storage
Mode preference is stored in `localStorage`:
```javascript
localStorage.getItem('isAutopilotMode') // 'true' or 'false'
```

### IPC Communication
```javascript
// Frontend sends mode to backend
ipcRenderer.invoke('execute-agent-task', {
  task, models, config,
  isAutopilotMode: true/false
})

// Backend requests permission (supervised mode only)
webContents.send('agent:step', {
  tool: 'write_file',
  status: 'awaiting_permission',
  summary: 'Write file: path/to/file.ts'
})
```

### Implementation
- Mode is passed through the entire agent loop
- Each file operation checks `isAutopilotMode`
- If false, requests permission via IPC
- If true, executes immediately

## Comparison with WhizCode

| Feature | WhizCode | WhizCode |
|---------|----------|------|
| Autopilot Mode | ✅ | ✅ |
| Supervised Mode | ✅ | ✅ |
| Command Approval | Always | Always |
| File Operation Approval | Supervised only | Supervised only |
| Toggle in UI | ✅ | ✅ |
| Persists Setting | ✅ | ✅ |

## Future Enhancements

### Planned
- Per-operation approval settings
- Approval history and patterns
- Smart approval (auto-approve safe operations)
- Approval rules (e.g., "always approve in test files")

### Possible
- Temporary autopilot (for single task)
- Approval presets (strict, balanced, permissive)
- File/folder-specific rules
- Rollback specific operations
