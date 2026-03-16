# WhizCode Architecture - WhizCode-Style Agent

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     WhizCode IDE                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              React Frontend (Vite)                    │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐ │  │
│  │  │  File   │  │ Editor  │  │  Chat   │  │Terminal │ │  │
│  │  │  Tree   │  │  Area   │  │  Panel  │  │  Pane   │ │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘ │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│                           │ IPC                              │
│                           ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Electron Main Process                       │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │         WhizCode-Style Agent Loop                   │ │  │
│  │  │  ┌──────────────┐      ┌──────────────┐        │ │  │
│  │  │  │ Primary Model│      │  Tool Model  │        │ │  │
│  │  │  │  (Reasoning) │      │   (Coding)   │        │ │  │
│  │  │  └──────────────┘      └──────────────┘        │ │  │
│  │  │         │                      │                │ │  │
│  │  │         └──────────┬───────────┘                │ │  │
│  │  │                    ▼                            │ │  │
│  │  │         ┌─────────────────────┐                │ │  │
│  │  │         │   Tool Executor     │                │ │  │
│  │  │         └─────────────────────┘                │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  │                                                       │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │              Services                           │ │  │
│  │  │  • IndexingService (Semantic Search)           │ │  │
│  │  │  • CodeGraphService (Dependencies)             │ │  │
│  │  │  • DiffService (Transactional Edits)           │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
        ┌──────────────────────────────────────┐
        │         External Services            │
        │  ┌────────────┐  ┌────────────────┐ │
        │  │   Ollama   │  │ OpenAI/Gemini  │ │
        │  │  (Local)   │  │    (Cloud)     │ │
        │  └────────────┘  └────────────────┘ │
        │  ┌────────────┐  ┌────────────────┐ │
        │  │ Voyage AI  │  │   LanceDB      │ │
        │  │(Embeddings)│  │  (Vector DB)   │ │
        │  └────────────┘  └────────────────┘ │
        └──────────────────────────────────────┘
```

## Agent Flow

### Old Architecture (Two-Phase)
```
User Request
    │
    ▼
┌─────────────┐
│   Planner   │ ──► Generate Plan
└─────────────┘
    │
    ▼
Wait for Approval ⏸️
    │
    ▼
┌─────────────┐
│  Executor   │ ──► Execute Plan
└─────────────┘
    │
    ▼
Response
```

### New Architecture (Unified)
```
User Request
    │
    ▼
┌──────────────────────────────────┐
│      Unified Agent Loop          │
│                                  │
│  Iteration 1:                    │
│  ┌────────────────┐              │
│  │ Primary Model  │ ──► Analyze  │
│  └────────────────┘              │
│         │                        │
│         ▼                        │
│  ┌────────────────┐              │
│  │  Tool Model    │ ──► Execute  │
│  └────────────────┘              │
│         │                        │
│         ▼                        │
│  Iteration 2:                    │
│  ┌────────────────┐              │
│  │  Tool Model    │ ──► Execute  │
│  └────────────────┘              │
│         │                        │
│         ▼                        │
│  ... (autonomous loop)           │
│         │                        │
│         ▼                        │
│  Final Response                  │
└──────────────────────────────────┘
```

## Multi-Model Strategy

### Model Selection Logic
```
┌─────────────────────────────────────────┐
│         Agent Iteration                 │
│                                         │
│  Is this the first iteration?           │
│         │                               │
│    ┌────┴────┐                          │
│    │         │                          │
│   YES       NO                          │
│    │         │                          │
│    ▼         ▼                          │
│ Primary   Tool                          │
│  Model    Model                         │
│    │         │                          │
│    └────┬────┘                          │
│         │                               │
│         ▼                               │
│    Call AI                              │
│         │                               │
│         ▼                               │
│  Parse Response                         │
│         │                               │
│    ┌────┴────┐                          │
│    │         │                          │
│  Tool      Final                        │
│  Call    Response                       │
│    │         │                          │
│    ▼         ▼                          │
│ Execute    Return                       │
└─────────────────────────────────────────┘
```

### Model Roles

**Primary Model (First Iteration)**
- Receives: User request + full context
- Analyzes: What needs to be done
- Decides: First action to take
- Outputs: Tool call or thinking

**Tool Model (Subsequent Iterations)**
- Receives: Previous results + context
- Executes: Specific tool operations
- Generates: Code or commands
- Outputs: Tool calls

## Tool System

### Available Tools (15+)

```
Discovery & Context:
├── list_directory      → Browse folders
├── search_files        → Pattern search
├── semantic_search     → AI-powered search
└── read_file          → Read file contents

File Operations:
├── write_file         → Create/overwrite files
├── edit_file          → Targeted edits
├── delete_file        → Remove files
├── create_directory   → Make folders
├── replace_lines      → Line-based edits
├── insert_code        → Insert at position
└── apply_diffs        → Multi-file changes

Execution:
├── run_command        → Terminal commands (gated)
├── validate_project   → TypeScript check
└── run_tests          → Test execution

Analysis:
├── get_blast_radius   → Dependency impact
└── semantic_search    → Code search
```

### Tool Execution Flow

```
Agent decides to use tool
    │
    ▼
Parse tool call (JSON)
    │
    ▼
Is it run_command?
    │
    ├─YES─► Request permission
    │           │
    │           ├─Approved─► Execute
    │           └─Denied──► Abort
    │
    └─NO──► Execute directly
                │
                ▼
            Return result
                │
                ▼
        Add to conversation
                │
                ▼
        Next iteration
```

## Context Management

### Context Structure
```xml
<project_context>
  <workspace_root>/path/to/project</workspace_root>
  
  <file_tree>
    - src/
      - components/
        - App.tsx
        - Button.tsx
      - utils/
        - helpers.ts
    - package.json
    - tsconfig.json
  </file_tree>
  
  <active_editor_file>
    <path>src/components/App.tsx</path>
    <content>
      import React from 'react'
      ...
    </content>
  </active_editor_file>
</project_context>
```

### Context Flow
```
Workspace opened
    │
    ▼
Index files (manifest)
    │
    ▼
Build file tree
    │
    ▼
Initialize services
    │
    ├─► IndexingService (semantic search)
    ├─► CodeGraphService (dependencies)
    └─► DiffService (transactions)
    │
    ▼
Watch for changes
    │
    ▼
Update context on change
```

## Loop Prevention

### Detection Mechanisms

**1. Direct Repetition**
```
Tool Call A
    │
    ▼
Tool Call A (same)
    │
    ▼
⚠️ WARNING: Don't repeat
```

**2. Ping-Pong Loop**
```
Tool Call A
    │
    ▼
Tool Call B
    │
    ▼
Tool Call A (again)
    │
    ▼
Tool Call B (again)
    │
    ▼
⚠️ LOOP DETECTED: Change strategy
```

**3. Thinking Loop**
```
<THOUGHT>...</THOUGHT>
    │
    ▼
<THOUGHT>...</THOUGHT>
    │
    ▼
⚠️ NUDGE: Take action
```

## Service Architecture

### IndexingService
```
File Change
    │
    ▼
Parse with Tree-sitter
    │
    ▼
Extract semantic chunks
    │
    ▼
Generate embeddings (Voyage AI)
    │
    ▼
Store in LanceDB
    │
    ▼
Enable semantic search
```

### CodeGraphService
```
File Change
    │
    ▼
Parse imports/exports
    │
    ▼
Build dependency graph
    │
    ▼
Calculate dependents
    │
    ▼
Enable blast radius queries
```

### DiffService
```
Multi-file changes requested
    │
    ▼
Parse diff blocks
    │
    ▼
Validate all changes
    │
    ├─Valid─► Apply all
    │           │
    │           ├─Success─► Commit
    │           └─Failure─► Rollback
    │
    └─Invalid─► Reject all
```

## Communication Flow

### IPC Messages

**Frontend → Backend**
```javascript
ipcRenderer.invoke('execute-agent-task', {
  task: "User request",
  primaryModel: { provider, model },
  toolModel: { provider, model },
  workspacePath: "/path",
  activeFile: { path, content },
  config: { keys }
})
```

**Backend → Frontend**
```javascript
// Step updates
webContents.send('agent:step', {
  tool: 'read_file',
  status: 'running',
  summary: 'Reading App.tsx',
  iteration: 1
})

// Permission requests
webContents.send('agent:step', {
  tool: 'run_command',
  status: 'awaiting_permission',
  command: 'npm install'
})
```

## Performance Characteristics

### Memory Usage
```
Base Application:     ~200MB
+ Ollama Model (7b):  ~4GB
+ Ollama Model (16b): ~10GB
+ Vector DB:          ~100MB per 1000 files
+ Code Graph:         ~50MB per 1000 files
```

### Response Times
```
First Request:        2-10s (model loading)
Subsequent:           0.5-3s (model cached)
Tool Execution:       0.1-1s (file ops)
Semantic Search:      0.5-2s (vector query)
```

## Security Model

### Sandboxing
```
┌─────────────────────────────────┐
│      Electron Renderer          │
│  (No Node.js access)            │
│         │                       │
│         │ IPC                   │
│         ▼                       │
│  ┌──────────────────┐           │
│  │  Preload Script  │           │
│  │  (Controlled)    │           │
│  └──────────────────┘           │
│         │                       │
│         │ IPC                   │
│         ▼                       │
│  ┌──────────────────┐           │
│  │   Main Process   │           │
│  │  (Full Access)   │           │
│  └──────────────────┘           │
└─────────────────────────────────┘
```

### Permission Gates
- ✅ File read/write: Automatic
- ⚠️ Terminal commands: Requires approval
- ✅ Search operations: Automatic
- ✅ Validation: Automatic

## Extensibility Points

### Future Enhancements

**Phase 3: Advanced Tools**
- readCode (AST-based)
- editCode (AST-based)
- getDiagnostics (LSP)
- semanticRename
- smartRelocate

**Phase 4: Enhanced Context**
- Git integration
- Diagnostic context
- Terminal output
- Test results

**Phase 5: Advanced Features**
- Sub-agents
- Hooks system
- Steering files
- MCP integration
- Spec system
