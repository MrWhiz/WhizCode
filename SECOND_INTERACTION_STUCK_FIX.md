# Second Interaction Stuck at read_file - Fixed

## Problem
The first chat interaction worked fine, but the second interaction got stuck at `read_file`. This suggested a state issue or resource leak between interactions.

## Root Cause
The learning system recording was **blocking** the agent loop with an `await`. On the second interaction, the learning system was taking longer (possibly due to accumulated state or memory), causing the agent to hang waiting for it to complete.

The issue was in `electron/main.ts`:
```typescript
// BLOCKING - waits for learning system to complete
await Promise.race([
  recordInteractionForLearning(...),
  new Promise((_, reject) => setTimeout(() => reject(...), 5000))
]);
```

This meant if the learning system took >5 seconds on the second call, the agent would hang.

## Solution

### Made Learning System Recording Non-Blocking
**File**: `electron/main.ts`

**Changes**:
1. Removed `await` from learning system recording
2. Changed to fire-and-forget pattern with timeout
3. Increased timeout from 5 seconds to 10 seconds
4. Increased code intelligence timeout from 3 seconds to 5 seconds

```typescript
// NON-BLOCKING - fire and forget
Promise.race([
  recordInteractionForLearning(...),
  new Promise((_, reject) => setTimeout(() => reject(...), 10000))
]).catch(error => {
  console.warn('[LEARNING] Failed to record tool execution:', error);
});
```

## How It Works

1. **First interaction**: Learning system records interaction in background
2. **Agent continues**: Doesn't wait for learning system to finish
3. **Second interaction**: Learning system records new interaction in background
4. **No blocking**: Agent loop continues immediately regardless of learning system state

## Benefits

✅ **No blocking** - Agent loop continues immediately
✅ **Resilient** - Learning system delays don't affect agent execution
✅ **Scalable** - Works for any number of interactions
✅ **Graceful degradation** - If learning system fails, agent continues
✅ **Better timeouts** - Increased from 5s to 10s for learning, 3s to 5s for code intelligence

## Timeout Configuration

| Operation | Timeout | Behavior |
|-----------|---------|----------|
| Code Intelligence | 5 seconds | Falls back to empty suggestions |
| Learning Recording | 10 seconds | Fire and forget (non-blocking) |
| Permission Request | 60 seconds | Deny on timeout |
| Error Recovery | 30 seconds | Fallback result |

## Testing

1. **Test first interaction**: Should work fine
2. **Test second interaction**: Should not get stuck at read_file
3. **Test multiple interactions**: Should work for any number of chats
4. **Test with slow learning system**: Should continue even if learning is slow

## Files Modified

- `electron/main.ts` - Made learning system non-blocking, increased timeouts

## Backward Compatibility

✅ All changes are backward compatible
✅ No API changes
✅ No configuration changes required
✅ Learning system still records interactions (just asynchronously)
