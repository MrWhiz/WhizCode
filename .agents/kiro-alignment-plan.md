# Kiro Alignment Plan for WhizCode IDE

## Current Architecture Analysis

### What WhizCode Has:
1. **Two-Phase Agent System**: Planner (generates plan) → Executor (executes plan)
2. **Basic Tool Set**: 15+ tools for file operations, search, commands
3. **Simple System Prompts**: Short, directive-based prompts
4. **Permission Gating**: Only for run_command
5. **Loop Detection**: Basic ping-pong and repetition detection
6. **Multi-Model Support**: Ollama, OpenAI, Gemini (KEEP THIS - very valuable for local LLMs!)

### What Kiro Has (Target State):
1. **Unified Agent**: Single agent with rich context and decision-making
2. **Extensive Tool Set**: 25+ tools including semantic operations, code analysis, sub-agents
3. **Rich System Prompt**: Detailed identity, capabilities, rules, response style
4. **Sophisticated Context**: File trees, diagnostics, active editor, git status
5. **Advanced Features**: Hooks, steering files, specs, MCP integration
6. **Autonomous Decision-Making**: Proactive tool selection, context gathering

## Key Changes Required

### 1. System Prompt Enhancement
- Add Kiro's identity and personality
- Include detailed response style guidelines
- Add comprehensive rules and best practices
- Include tool usage patterns and decision trees

### 2. Tool Set Expansion
- Add: getDiagnostics, readCode, editCode, semanticRename, smartRelocate
- Add: grepSearch, fileSearch, readMultipleFiles
- Add: Sub-agent invocation (context-gatherer)
- Enhance: Better error messages, validation, rollback

### 3. Context Management
- Add file tree visualization
- Add active editor file tracking
- Add git diff context
- Add problem/diagnostic context

### 4. Conversation Flow
- Remove forced plan approval step
- Make agent more autonomous
- Add thinking/reasoning display
- Better final summaries

### 5. Decision-Making Logic
- Proactive context gathering
- Smart tool selection
- Better error recovery
- Loop prevention improvements

## Implementation Strategy

### Phase 1: Core System Prompt (Priority: HIGH)
- Create unified KIRO_SYSTEM_PROMPT with full identity, capabilities, rules, response style
- KEEP multi-model support but make it flexible:
  - Primary Model: Main reasoning and decision-making
  - Tool Model: Optimized for tool calling and code generation
  - Allow same model for both or different models based on user preference

### Phase 2: Agent Loop Refactor (Priority: HIGH)
- Simplify to single-phase architecture (no forced plan approval)
- Keep multi-model capability but use it intelligently:
  - Use Primary Model for reasoning/planning steps
  - Use Tool Model for code generation/execution steps
- Make agent more autonomous and proactive

### Phase 3: Tool Enhancement (Priority: MEDIUM)
- Add readCode, editCode tools
- Add getDiagnostics tool
- Add grepSearch, fileSearch tools
- Enhance existing tools with better error handling

### Phase 4: Context Enhancement (Priority: MEDIUM)
- Add file tree to context
- Add active file tracking
- Add diagnostics context

### Phase 5: Advanced Features (Priority: LOW)
- Add sub-agent support (context-gatherer)
- Add hooks system
- Add steering files
- Add MCP integration

## Success Criteria

1. Agent behaves autonomously without forced approval steps
2. Agent uses appropriate tools proactively
3. Agent provides concise, helpful responses
4. Agent handles errors gracefully
5. Agent avoids loops and repetition
6. Agent works seamlessly with Ollama models
