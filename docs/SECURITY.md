# WhizCode Security Implementation

## Overview
This document outlines the security measures implemented in WhizCode to protect against common vulnerabilities.

## Security Fixes Applied

### 1. Path Traversal Prevention
**Issue**: IPC handlers could read/write files outside the workspace boundary.

**Solution**: 
- Added `validatePathInWorkspace()` function that ensures all file operations stay within the workspace
- All file operation handlers (`fs:readFile`, `fs:writeFile`, `fs:rename`, `fs:delete`) now validate paths
- Rejects paths containing `..`, null bytes, or starting with `~`

**Implementation**:
```typescript
validatePathInWorkspace(filePath, workspacePath);
```

### 2. Command Injection Prevention
**Issue**: Git commands used string interpolation, allowing command injection.

**Solution**:
- Replaced `execAsync()` with `execFile()` for git operations
- Uses argument arrays instead of shell strings
- Prevents shell metacharacter interpretation

**Before**:
```typescript
await execAsync(`git commit -m "${message.replace(/"/g, '\\"')}"`, { cwd: path });
```

**After**:
```typescript
await execFileAsync('git', ['commit', '-m', validMessage], { cwd: validPath });
```

### 3. Input Validation
**Issue**: No validation on user inputs from IPC handlers.

**Solution**:
- Added `validateStringInput()` for general string validation with length limits
- Added `validateFilePath()` for file path validation
- All IPC handlers now validate inputs before use

**Limits**:
- File paths: 500 characters max
- General strings: 10,000 characters max
- File content: 50MB max
- Git messages: 1,000 characters max

### 4. Sensitive Data Encryption
**Issue**: Azure tokens stored in plaintext.

**Solution**:
- Implemented AES-256-CBC encryption for sensitive data
- Tokens encrypted before storage, decrypted on load
- Uses `crypto.scryptSync()` for key derivation

**Implementation**:
```typescript
// Save
await fs.writeFile(AZURE_TOKEN_FILE, JSON.stringify({ 
  token: encryptData(token), 
  expires 
}), 'utf-8');

// Load
const token = decryptData(parsed.token);
```

**Environment Variable**:
Set `WHIZCODE_ENCRYPTION_KEY` for custom encryption key (defaults to 'default-key-change-in-production').

## Security Utilities

### `securityUtils.ts`
Provides the following functions:

- `validatePathInWorkspace(filePath, workspacePath)` - Prevents path traversal
- `sanitizeShellInput(input)` - Escapes shell metacharacters
- `encryptData(data, key?)` - Encrypts sensitive data
- `decryptData(encryptedData, key?)` - Decrypts sensitive data
- `validateStringInput(input, maxLength?)` - Validates string input
- `validateFilePath(input)` - Validates file paths

## IPC Handler Security

All IPC handlers now include:
1. Input validation
2. Path validation (where applicable)
3. Error logging with security context
4. Length limits on inputs

### Updated Handlers
- `fs:readFile` - Validates path and workspace boundary
- `fs:writeFile` - Validates path, content, and workspace boundary
- `fs:rename` - Validates both paths and workspace boundary
- `fs:delete` - Validates path and workspace boundary
- `git:status` - Validates workspace path
- `git:stage` - Validates paths using execFile
- `git:commit` - Validates message and path using execFile

## Best Practices

### For Developers
1. Always validate user input before using it
2. Use `execFile()` instead of `exec()` for external commands
3. Use path validation for file operations
4. Encrypt sensitive data before storage
5. Log security-related errors with `[SECURITY]` prefix

### For Users
1. Set `WHIZCODE_ENCRYPTION_KEY` environment variable for production
2. Keep WhizCode updated for security patches
3. Only open trusted workspaces
4. Review file operations in logs

## Testing Security

To test the security measures:

```bash
# Test path traversal prevention
# Should fail: attempting to read outside workspace
ipcRenderer.invoke('fs:readFile', '../../sensitive.txt', '/workspace');

# Test command injection prevention
# Should fail: git message with shell metacharacters
ipcRenderer.invoke('git:commit', { 
  path: '/workspace', 
  message: 'test"; rm -rf /' 
});

# Test input validation
# Should fail: exceeds length limit
ipcRenderer.invoke('fs:writeFile', '/file.txt', 'x'.repeat(60000000));
```

## Future Improvements

1. Add rate limiting to IPC handlers
2. Implement request signing for critical operations
3. Add audit logging for sensitive operations
4. Implement code signing for releases
5. Add security headers to renderer process

## Reporting Security Issues

If you discover a security vulnerability, please email security@whizcode.dev with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

Do not publicly disclose security issues until they have been addressed.
