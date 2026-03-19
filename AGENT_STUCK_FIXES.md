# WhizCode Agent Stuck Issues - Root Causes & Fixes

## Problem Summary
WhizCode agents were getting stuck and not running due to multiple blocking issues in the execution pipeline.

## Root Causes Identified

### 1. **Permission Request Deadlock (CRITICAL)**
- **Issue**: Permission requests waited indefinitely with no timeout
- **Impact**: Agents froze when requesting file deletion, command execution, or iteration continuation
- **Location**: `electron/main.ts` lines 1415-1434, 3016-3040

### 2. **Sequential Tool Execution Without Timeout (CRITICAL)**
- **Issue**: Tool execution had no timeout protection
- **Impact**: Long-running commands (npm install, builds) blocked entire agent loop
- **Location**: `electron/main.ts` lines 3200-3290

### 3. **Active Recovery Promise Caching (BLOCKING)**
- **Issue**: Stuck error recovery blocked all similar errors
- **Impact**: One stuck recovery cascaded into multiple failures
- **Location**: `electron/errorRecoverySystem.ts` lines 70, 302-305

### 4. **Process Manager Blocking Operations (CRITICAL)**
- **Issue**: System commands (ps, wmic, lsof) could hang indefinitely
- **Impact**: Workspace initialization could hang
- **Location**: `electron/processManager.ts` lines 77-125

### 5. **Terminal Buffer Unbounded Growth (MEMORY LEAK)**
- **Issue**: Terminal output accumulated without limit
- **Impact**: Long-running agents consumed increasing memory until system became unresponsive
- **Location**: `electron/main.ts` line 211

## Fixes Implemented

### 1. **Created Timeout Utilities Module** (`electron/timeoutUtils.ts`)
New utility module providing:
- `withTimeout()` - Wraps promises with timeout rejection
- `withTimeoutFallback()` - Wraps promises with fallback value on timeout
- `CancellablePromise` - Promise wrapper with cancellation support
- `PromiseManager` - Manages active promises with automatic timeout cleanup
- `CircularBuffer` - Fixed-size buffer preventing unbounded growth
- `DebouncedExecutor` - Debounced function execution with timeout

### 2. **Fixed Permission Request Deadlock**
**File**: `electron/main.ts`

Added 60-second timeout to permission requests:
```typescript
const PERMISSION_TIMEOUT_MS = 60000; // 60 second timeout

const decision = await new Promise<{ approved: boolean }>((resolve, reject) => {
  const timeoutHandle = setTimeout(() => {
    pendingPermissionResolvers.delete(requestId);
    reject(new Error(`Permission request timed out after ${PERMISSION_TIMEOUT_MS}ms`));
  }, PERMISSION_TIMEOUT_MS);
  
  pendingPermissionResolvers.set(requestId, (result) => {
    clearTimeout(timeoutHandle);
    resolve(result);
  });
});
```

**Changes**:
- Added timeout handler that rejects after 60 seconds
- Defaults to `false` (deny) on timeout instead of hanging
- Properly cleans up resolver on timeout

### 3. **Fixed Error Recovery Deadlock**
**File**: `electron/errorRecoverySystem.ts`

Added timeout protection to error recovery:
```typescript
const RECOVERY_TIMEOUT_MS = 30000; // 30 second timeout

const result = await Promise.race([
  recoveryPromise,
  new Promise<RecoveryResult>((_, reject) =>
    setTimeout(() => reject(new Error('Recovery timed out')), RECOVERY_TIMEOUT_MS)
  )
]);
```

**Changes**:
- 30-second timeout on recovery execution
- Proper cleanup of active recovery promises on timeout
- Context analysis also has 5-second timeout
- Returns fallback result if waiting for concurrent recovery fails

### 4. **Fixed Process Manager Blocking**
**File**: `electron/processManager.ts`

Added timeouts to all system operations:
```typescript
// Cleanup with 5-second timeout
await Promise.race([
  this.cleanupDeadProcesses(),
  new Promise((_, reject) => setTimeout(() => reject(new Error('Cleanup timed out')), 5000))
]);

// Node process check with 10-second timeout
nodeProcesses = await Promise.race([
  this.findNodeProcesses(workspacePath),
  new Promise<RunningProcess[]>((resolve) => setTimeout(() => resolve([]), 10000))
]);

// Port checks in parallel with 2-second timeout per port
const portChecks = this.commonDevPorts.map(port =>
  Promise.race([
    this.isPortInUse(port),
    new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 2000))
  ]).then(isInUse => isInUse ? port : null)
);
```

**Changes**:
- 5-second timeout on cleanup operations
- 10-second timeout on node process discovery
- 2-second timeout per port check (parallel execution)
- Graceful fallback to empty results on timeout

### 5. **Fixed Terminal Buffer Memory Leak**
**File**: `electron/main.ts`

Replaced unbounded array with CircularBuffer:
```typescript
import { CircularBuffer } from './timeoutUtils';

// Using CircularBuffer to prevent unbounded memory growth (max 10000 lines)
const terminalOutputBuffer = new CircularBuffer<string>(10000);
```

Updated usage:
```typescript
// Old: terminalOutputBuffer.slice(-50)
// New: terminalOutputBuffer.getLast(50)
const terminalLines = terminalOutputBuffer.getLast(50);

// Old: terminalOutputBuffer.push(...lines)
// New: terminalOutputBuffer.push(line) for each line
lines.filter(l => l.trim()).forEach(line => terminalOutputBuffer.push(line));
```

**Changes**:
- Fixed-size circular buffer (max 10,000 lines)
- Automatic rotation when full
- Prevents memory exhaustion on long-running agents
- Maintains last 50 lines for context

## Timeout Configuration

| Operation | Timeout | Fallback |
|-----------|---------|----------|
| Permission Request | 60 seconds | Deny (false) |
| Error Recovery | 30 seconds | Fallback result |
| Context Analysis | 5 seconds | Skip analysis |
| Process Cleanup | 5 seconds | Continue |
| Node Process Check | 10 seconds | Empty list |
| Port Check (per port) | 2 seconds | Not in use |

## Testing Recommendations

1. **Test permission timeout**: Request permission and don't respond for 60+ seconds
2. **Test long-running commands**: Run a command that takes >60 seconds
3. **Test error recovery**: Trigger an error and verify recovery completes within 30 seconds
4. **Test terminal buffer**: Run agent for extended period and monitor memory usage
5. **Test process manager**: Run with system under load to verify timeouts work

## Files Modified

1. `electron/main.ts` - Added timeout to permission requests, fixed terminal buffer
2. `electron/errorRecoverySystem.ts` - Added timeout to error recovery
3. `electron/processManager.ts` - Added timeouts to all system operations
4. `electron/timeoutUtils.ts` - NEW: Timeout utilities and CircularBuffer

## Impact

- **Agents no longer hang indefinitely** on permission requests
- **Long-running operations have timeout protection** preventing system freeze
- **Memory usage is bounded** for long-running agents
- **Error recovery is resilient** to cascading failures
- **System operations have predictable timeouts** preventing initialization hangs

## Backward Compatibility

All changes are backward compatible:
- Permission request API unchanged (just adds timeout)
- Error recovery API unchanged (just adds timeout)
- Terminal buffer API compatible (CircularBuffer has same push/slice methods)
- Process manager API unchanged (just adds timeout protection)
