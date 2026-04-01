# Structure Steering

## File Organization

Organize files by feature/domain, not by type. Backend commands are organized by functionality (agent, tools, services). Frontend components are organized by feature (Chat, Editor, Explorer, Terminal). Each major feature has its own directory with related files.

## Naming Conventions

- Use camelCase for variables, functions, and file names (e.g., `agentStreaming.rs`, `chatPanel.tsx`)
- Use PascalCase for React components and Rust structs (e.g., `ChatPanel`, `SteeringFiles`)
- Use SCREAMING_SNAKE_CASE for constants (e.g., `MAX_RETRIES`, `DEFAULT_TIMEOUT`)
- Prefix private/internal items with underscore (e.g., `_internal_function`)
- Use descriptive names that indicate purpose (e.g., `loop_recovery` not `lr`)

## Import Patterns

- Use absolute imports from `src/` for frontend (e.g., `import { ChatPanel } from 'src/components/Chat'`)
- Use relative imports for same-directory files
- Group imports: external libraries first, then internal modules, then relative imports
- Keep import statements organized and sorted alphabetically within groups

## Architectural Decisions

- Backend uses command pattern for Tauri IPC
- Frontend uses React hooks for state management
- Streaming responses use event-based architecture
- Agent loop runs in separate Rust thread
- Context memory persists to disk for recovery
- Steering files guide agent reasoning without hard-coding behavior
- Multi-persona orchestration routes tasks to specialized agents
- Confidence scoring provides transparency to users
