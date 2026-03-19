# Terminal Debugging Guide

## New Terminal Button Not Working - Troubleshooting

### Step 1: Verify Backend Integration

Check that `setupTerminalHandlers` is called in `electron/main.ts`:

```typescript
import { setupTerminalHandlers } from './terminalHandlers';

// After creating BrowserWindow:
setupTerminalHandlers(win);
```

### Step 2: Check Browser Console

Open DevTools (F12) and check for errors:
- Look for IPC send/receive errors
- Check for "terminal:create" messages

### Step 3: Check Electron Console

Look for backend errors:
- Terminal spawn errors
- Shell not found errors
- Process errors

### Step 4: Verify Shell Availability

The terminal system auto-detects available shells:

**Windows**: PowerShell, cmd
**macOS**: Zsh, bash, sh
**Linux**: Bash, sh

If a shell isn't available on your system, it won't appear in the menu.

### Step 5: Test IPC Communication

Add this to browser console to test:

```javascript
// Test getting available shells
window.ipcRenderer.invoke('terminal:getAvailableShells').then(shells => {
  console.log('Available shells:', shells);
});

// Test getting default shell
window.ipcRenderer.invoke('terminal:getDefaultShell').then(shell => {
  console.log('Default shell:', shell);
});

// Test creating a terminal
window.ipcRenderer.send('terminal:create', { 
  id: 'test_terminal', 
  type: 'bash' 
});
```

### Step 6: Check Terminal Manager

Verify `electron/terminalManager.ts` is properly spawning processes:

```typescript
// Add logging to createTerminal method
console.log(`[TERMINAL] Creating ${config.type} terminal`);
console.log(`[TERMINAL] Shell command:`, shell.command, shell.args);
```

### Common Issues

#### Issue: "Shell not found"
- **Cause**: Shell not installed on system
- **Fix**: Install the shell or use a different one

#### Issue: "Terminal not responding"
- **Cause**: IPC handlers not set up
- **Fix**: Verify `setupTerminalHandlers(win)` is called

#### Issue: "No terminals appear"
- **Cause**: Backend not initialized
- **Fix**: Check main.ts imports and setup

#### Issue: "Button click does nothing"
- **Cause**: Event handler not firing
- **Fix**: Check browser console for errors

### Debug Logging

Add logging to `MultiTerminalPane.tsx`:

```typescript
const createNewTerminal = (type: TerminalType) => {
  console.log('[TERMINAL] Creating new terminal:', type);
  
  const id = `terminal_${Date.now()}`;
  const newTerminal: Terminal = {
    id,
    type,
    name: `${type} - ${terminals.length + 1}`,
    createdAt: Date.now()
  };

  setTerminals(prev => [...prev, newTerminal]);
  setActiveTerminalId(id);
  setShowShellMenu(false);

  if (ipc) {
    console.log('[TERMINAL] Sending terminal:create to backend');
    ipc.send('terminal:create', { id, type });
  } else {
    console.error('[TERMINAL] IPC not available');
  }
};
```

### Verify File Structure

Ensure all files exist:
- ✅ `electron/terminalManager.ts`
- ✅ `electron/terminalHandlers.ts`
- ✅ `src/components/Terminal/MultiTerminalPane.tsx`
- ✅ `src/components/Terminal/TerminalPane.tsx`

### Test Checklist

- [ ] `setupTerminalHandlers(win)` is called in main.ts
- [ ] No TypeScript errors in terminal files
- [ ] Browser console shows no errors
- [ ] Electron console shows no errors
- [ ] IPC communication works (test in console)
- [ ] Shell is available on your system
- [ ] Terminal tab appears when clicking +
- [ ] Terminal content renders

### Still Not Working?

1. Check that main.ts has the import and setup call
2. Verify terminalHandlers.ts is in electron/ directory
3. Check that TerminalManager is properly exported
4. Look for any shell-specific issues on your OS
5. Try restarting the app

### Platform-Specific Issues

**Windows**:
- PowerShell might need execution policy changes
- Try cmd if PowerShell doesn't work

**macOS**:
- Zsh is default, bash might not be available
- Check `/bin/bash` exists

**Linux**:
- Bash should be available
- Check `/bin/bash` exists

---

For more help, check MULTI_TERMINAL_GUIDE.md or TERMINAL_INTEGRATION.md
