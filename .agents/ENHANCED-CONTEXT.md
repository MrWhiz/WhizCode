# Enhanced Context System

## Overview
The agent now has access to rich contextual information automatically included in every request, similar to Kiro's context system.

## Available Context

### 1. Workspace Root
```xml
<workspace_root>/path/to/project</workspace_root>
```
The root directory of the currently opened workspace.

### 2. File Tree
```xml
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
```
Complete directory structure of the workspace (up to 3000 files).

### 3. Active Editor File
```xml
<active_editor_file>
  <path>src/components/App.tsx</path>
  <content>
    import React from 'react'
    // ... file content
  </content>
</active_editor_file>
```
The currently open file in the editor with its full content.

### 4. Problems/Diagnostics ✨ NEW
```xml
<problems>
src/components/App.tsx:15:5 - error TS2304: Cannot find name 'useState'.
src/components/App.tsx:23:10 - warning: Unused variable 'count'
</problems>
```
TypeScript and ESLint errors/warnings for the active file. Only included if there are issues.

### 5. Git Diff ✨ NEW
```xml
<git_diff>
diff --git a/src/App.tsx b/src/App.tsx
index 1234567..abcdefg 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -10,7 +10,7 @@
-  const [count, setCount] = useState(0)
+  const [count, setCount] = useState<number>(0)
</git_diff>
```
Shows uncommitted changes in the workspace. Only included if there are changes. Truncated to 5KB to avoid context overflow.

## How It Works

### Automatic Inclusion
All context is automatically gathered and included with every agent request. The agent doesn't need to request it explicitly.

### Smart Filtering
- **Diagnostics**: Only included if there are actual problems
- **Git Diff**: Only included if there are uncommitted changes
- **Truncation**: Large diffs are truncated to prevent context overflow

### Error Handling
If any context gathering fails (e.g., git not available, TypeScript not configured), it's silently skipped without affecting the agent.

## Benefits

### 1. Better Problem Solving
The agent can see errors and warnings immediately, allowing it to:
- Fix TypeScript errors proactively
- Address linting warnings
- Understand compilation issues

### 2. Change Awareness
With git diff context, the agent knows:
- What changes have been made
- What's currently being worked on
- Potential conflicts or issues

### 3. Reduced Tool Calls
The agent doesn't need to:
- Call `getDiagnostics` manually
- Run `git diff` commands
- Ask "what errors do you see?"

### 4. More Intelligent Responses
The agent can:
- Prioritize fixing visible errors
- Understand the current state better
- Make more informed decisions

## Example Usage

### Before (Without Enhanced Context)
```
User: "Fix the errors in this file"
Agent: "Let me check for errors first"
Agent: {"tool": "getDiagnostics", "path": "App.tsx"}
Agent: "I see you have a TypeScript error. Let me fix it..."
```

### After (With Enhanced Context)
```
User: "Fix the errors in this file"
Agent: "I can see the TypeScript error on line 15. Let me fix it..."
Agent: {"tool": "edit_file", ...}
```

## Future Enhancements

### Planned
- **Terminal Output**: Last N lines of terminal output
- **Test Results**: Recent test run results
- **Manual Context Injection**: Support for #File, #Folder, #Problems syntax

### Possible
- **Breakpoint Context**: Active debugging session info
- **Network Requests**: Recent API calls and responses
- **Performance Metrics**: Build times, bundle sizes

## Configuration

Currently, enhanced context is always enabled. Future versions may add:
- Toggle to disable specific context types
- Configurable truncation limits
- Context priority settings

## Performance Impact

### Memory
- Minimal: Context is gathered on-demand
- Diagnostics: ~1-5KB per file
- Git Diff: ~1-5KB (truncated at 5KB)

### Speed
- Diagnostics: ~100-500ms (TypeScript check)
- Git Diff: ~50-200ms (git command)
- Total overhead: ~150-700ms per request

This is acceptable for the significant improvement in agent intelligence.

## Implementation Details

### Location
Enhanced context is built in `electron/main.ts` in the `runAgentLoop` function.

### Dependencies
- TypeScript/ESLint for diagnostics
- Git for diff information
- Existing `getDiagnostics` function

### Error Handling
All context gathering is wrapped in try-catch blocks to ensure failures don't break the agent.
