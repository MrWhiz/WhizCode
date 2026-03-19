# Additional Fixes: Agent Stuck on Tool Execution

## Problem
Agent was stuck trying to read `src/main.jsx` which doesn't exist. The tool execution had no timeout protection and poor error handling for missing files.

## Root Causes

### 1. **No Timeout on Tool Execution**
- Tools could block indefinitely without any timeout
- No distinction between fast tools (read_file) and slow tools (run_command)
- Agent would hang waiting for tool result

### 2. **Poor Error Handling for Missing Files**
- `read_file` tool didn't check if file exists before attempting to read
- Errors weren't caught properly, causing agent to hang
- No helpful error message suggesting alternatives

### 3. **No Tool-Specific Timeout Configuration**
- All tools had same timeout (or no timeout)
- Fast tools (read_file) shouldn't wait as long as slow tools (run_command)
- No way to configure timeouts per tool type

## Fixes Implemented

### 1. **Added Tool-Specific Timeouts**
**File**: `electron/main.ts`

Created `TOOL_TIMEOUTS` configuration:
```typescript
const TOOL_TIMEOUTS: Record<string, number> = {
  'read_file': 5000,           // 5 seconds
  'write_file': 10000,         // 10 seconds
  'edit_file': 10000,          // 10 seconds
  'list_directory': 10000,     // 10 seconds
  'search_files': 15000,       // 15 seconds
  'run_command': 120000,       // 2 minutes
  'getDiagnostics': 30000,     // 30 seconds
  'grepSearch': 15000,         // 15 seconds
  'fileSearch': 10000,         // 10 seconds
  'readMultipleFiles': 10000,  // 10 seconds
  'semantic_search': 20000,    // 20 seconds
  'mcp_call': 30000,           // 30 seconds
  'default': 30000             // 30 seconds
};
```

### 2. **Wrapped Tool Execution with Timeout**
**File**: `electron/main.ts`

Split `executeToolCall` into two functions:
- `executeToolCall()` - Wrapper that applies timeout
- `executeToolCallInternal()` - Actual tool execution logic

```typescript
return Promise.race([
  executeToolCallInternal(...),
  new Promise<{ result }>((_, reject) =>
    setTimeout(() => reject(new Error(`Tool execution timed out after ${toolTimeout}ms`)), toolTimeout)
  )
]).catch(error => {
  if (error.message.includes('timed out')) {
    return { result: `❌ Tool execution timed out after ${toolTimeout}ms...` };
  }
  throw error;
});
```

### 3. **Improved File Reading Error Handling**
**File**: `electron/main.ts`

Enhanced `read_file` tool:
```typescript
case 'read_file': {
  if (!toolData.path) return { result: '❌ Error: Tool "read_file" requires a "path" parameter.' };
  
  try {
    // Check if file exists first
    try {
      await fs.access(resolvedPath);
    } catch {
      return { result: `❌ Error: File not found: ${toolData.path}\n\nResolved path: ${resolvedPath}\n\nUse 'list_directory' to check available files.` };
    }
    
    const isBinary = await isBinaryFile(resolvedPath);
    if (isBinary) {
      return { result: `❌ Cannot read ${toolData.path}: This appears to be a binary file.` };
    }
    const content = await fs.readFile(resolvedPath, 'utf-8');
    const lines = content.split('\n');
    return { result: lines.map((line, i) => `${i + 1}: ${line}`).join('\n') };
  } catch (error) {
    return { result: `❌ Error reading file ${toolData.path}: ${error instanceof Error ? error.message : String(error)}` };
  }
}
```

**Changes**:
- Checks file existence before attempting to read
- Provides helpful error message with resolved path
- Suggests using `list_directory` to find correct file
- Catches all errors with descriptive messages

## Tool Timeout Configuration

| Tool | Timeout | Reason |
|------|---------|--------|
| read_file | 5s | Fast file I/O |
| write_file | 10s | File I/O + disk sync |
| edit_file | 10s | File I/O + parsing |
| list_directory | 10s | Directory traversal |
| search_files | 15s | Pattern matching across files |
| run_command | 120s | Commands can be slow (builds, installs) |
| getDiagnostics | 30s | TypeScript compilation can be slow |
| grepSearch | 15s | Regex search across codebase |
| fileSearch | 10s | Fuzzy file search |
| readMultipleFiles | 10s | Multiple file reads |
| semantic_search | 20s | AI-powered search |
| mcp_call | 30s | External MCP server calls |
| default | 30s | Unknown tools |

## Error Messages Improved

### Before
```
(Agent hangs indefinitely)
```

### After
```
❌ Error: File not found: src/main.jsx

Resolved path: /workspace/src/main.jsx

Use 'list_directory' to check available files.
```

## Testing

1. **Test missing file**: Try to read non-existent file
   - Expected: Helpful error message suggesting alternatives
   - Result: ✅ Agent continues with next step

2. **Test slow command**: Run a command that takes >120 seconds
   - Expected: Tool times out after 120 seconds
   - Result: ✅ Agent gets timeout error and continues

3. **Test fast file read**: Read a file
   - Expected: Completes within 5 seconds
   - Result: ✅ File content returned immediately

## Impact

- **Agents no longer hang on missing files** - Get helpful error messages instead
- **Tool execution has predictable timeouts** - No more indefinite waits
- **Fast tools don't wait unnecessarily** - Optimized timeout per tool type
- **Better error recovery** - Agents can adapt when tools timeout

## Files Modified

1. `electron/main.ts` - Added tool timeouts and improved error handling

## Backward Compatibility

All changes are backward compatible:
- Tool API unchanged
- Error messages are more helpful but still parseable
- Timeout behavior is transparent to callers
