# Process Management System

WhizCode now includes an intelligent process management system that automatically detects and handles running development server instances to prevent conflicts.

## Features

### Automatic Process Detection
When starting development servers (like `npm run dev`, `yarn start`, `vite`, etc.), WhizCode will:

1. **Check for existing processes** that might conflict with the new server
2. **Identify port conflicts** on common development ports (3000, 4000, 5000, etc.)
3. **Prompt for permission** to stop conflicting processes
4. **Clean up dead processes** from the tracking system

### Supported Process Types
- **Development servers**: npm/yarn/pnpm dev, vite, webpack-dev-server, etc.
- **Build processes**: npm/yarn/pnpm build, webpack, etc.
- **Test runners**: jest, vitest, etc.
- **Static servers**: serve, http-server, etc.

### Common Development Ports Monitored
- 3000, 3001 (React, Next.js)
- 4000, 4173 (Vite preview)
- 5000, 5173 (Vite dev)
- 8000, 8080, 8081 (Various servers)
- 9000 (Various tools)

## Usage

### Automatic Handling
The process management system works automatically when you use the `run_command` tool to start development servers. No additional configuration is needed.

Example:
```json
{"tool": "run_command", "command": "npm run dev"}
```

If WhizCode detects conflicting processes, it will:
1. Show you what processes are running
2. Ask for permission to stop them
3. Stop the processes if approved
4. Start your new development server

### Manual Process Checking
You can manually check for running processes using the `check_processes` tool:

```json
{"tool": "check_processes"}
```

This will show you:
- Currently running development-related processes
- Ports that are in use
- Process types and commands

## Process Classification

The system classifies processes into categories:
- **dev-server**: Development servers and live-reload tools
- **build**: Build and compilation processes
- **test**: Test runners and testing tools
- **other**: Other Node.js processes

## Platform Support

The process management system works on:
- **Windows**: Uses `tasklist`, `netstat`, and `taskkill`
- **macOS/Linux**: Uses `ps`, `lsof`, and `kill`

## Process Storage

WhizCode tracks running processes in:
- **Workspace-specific**: `.whizcode/processes/running-processes.json`
- **Global**: `%APPDATA%/whizcode/processes/running-processes.json` (Windows) or `~/.config/whizcode/processes/running-processes.json` (macOS/Linux)

## Error Handling

The system gracefully handles:
- **Permission errors**: When unable to stop processes
- **Dead processes**: Automatically cleaned up from tracking
- **Platform differences**: Uses appropriate commands for each OS
- **Network errors**: When checking port availability

## Benefits

1. **Prevents port conflicts** that cause "EADDRINUSE" errors
2. **Avoids resource waste** from multiple running servers
3. **Improves development workflow** by handling process management automatically
4. **Provides visibility** into what's running in your development environment

## Example Workflow

1. You ask WhizCode to start a development server
2. WhizCode detects you have an old server still running on port 3000
3. WhizCode asks: "Found running dev server on port 3000. Stop it first?"
4. You approve, and WhizCode stops the old server
5. WhizCode starts your new development server successfully

This eliminates the common frustration of having to manually find and kill processes when switching between projects or restarting servers.