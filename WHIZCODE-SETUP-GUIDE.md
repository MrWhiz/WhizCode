# WhizCode-Style Agent Setup Guide

## Quick Start

Your WhizCode IDE now uses a WhizCode-inspired autonomous agent system with flexible multi-model support optimized for local LLMs.

## Model Configuration

### Understanding the Two Models

**Primary Model** - For reasoning and decision-making
- Analyzes your requests
- Plans the approach
- Makes strategic decisions
- Understands context

**Tool Model** - For code generation and execution
- Generates code
- Executes tool calls
- Produces structured output (JSON)
- Handles precise syntax

### Recommended Ollama Models

#### For Primary Model (Reasoning):
```bash
# Strong reasoning, good context understanding
ollama pull llama3:8b
ollama pull mistral:7b
ollama pull qwen2.5:7b

# Larger for better reasoning
ollama pull llama3:70b
ollama pull mixtral:8x7b
```

#### For Tool Model (Coding):
```bash
# Specialized for code
ollama pull deepseek-coder-v2:16b
ollama pull qwen2.5-coder:7b
ollama pull codellama:13b
ollama pull starcoder2:15b
```

### Configuration Examples

#### Best Performance (Recommended):
```
Primary Model: llama3:8b (Ollama)
Tool Model: deepseek-coder-v2:16b (Ollama)
```
Why: Llama3 excels at reasoning, DeepSeek-Coder excels at code generation.

#### Balanced (Single Model):
```
Primary Model: qwen2.5-coder:7b (Ollama)
Tool Model: qwen2.5-coder:7b (Ollama)
```
Why: Qwen2.5-Coder is good at both reasoning and coding.

#### Fast & Light:
```
Primary Model: llama3:3b (Ollama)
Tool Model: qwen2.5-coder:3b (Ollama)
```
Why: Smaller models for faster responses on limited hardware.

#### Hybrid (Cloud + Local):
```
Primary Model: gpt-4o (OpenAI)
Tool Model: deepseek-coder-v2:16b (Ollama)
```
Why: Use cloud for reasoning, local for code generation.

## How to Configure

1. Open WhizCode
2. Click the settings icon in the chat panel
3. Expand "Agent Configuration"
4. Select your Primary Model (reasoning)
5. Select your Tool Model (coding)
6. Click the refresh button to see available Ollama models

## Agent Behavior

### What's Different from Before:

**Old Behavior:**
- Agent creates a plan
- Waits for your approval
- Then executes the plan

**New Behavior (WhizCode-style):**
- Agent acts autonomously
- No forced approval steps (except for terminal commands)
- More natural conversation flow
- Thinks out loud when helpful
- Provides concise summaries

### When You'll See Approvals:

Only for potentially dangerous operations:
- Running terminal commands (`npm install`, `rm -rf`, etc.)
- You can check "Always Run" to auto-approve

### Thinking Process:

The agent may show its thinking:
```
<THOUGHT>
I need to read the file first to see the current structure,
then make the edit with exact indentation.
</THOUGHT>
```

This helps you understand its reasoning.

## Tips for Best Results

### 1. Be Specific
❌ "Fix the bug"  
✅ "Fix the TypeError in src/utils.ts line 42"

### 2. Provide Context
❌ "Add a button"  
✅ "Add a submit button to the login form in src/components/Login.tsx"

### 3. Let It Work
- The agent will use multiple tools automatically
- Don't interrupt unless necessary
- Trust the process

### 4. Use the Active File
- Open the file you want to work on in the editor
- The agent sees the active file content automatically
- This provides better context

### 5. Model Selection Matters
- Use reasoning models for complex planning
- Use code models for implementation
- Mix and match based on your hardware

## Troubleshooting

### Agent Keeps Repeating Actions
- The agent has built-in loop detection
- If it repeats, it will self-correct
- If stuck, click "Stop" and rephrase your request

### Agent Thinks Too Much
- The agent will nudge itself to act
- If it's overthinking, it will self-correct
- You can also say "Just do it" to push it forward

### Ollama Not Detected
- Make sure Ollama is running: `ollama serve`
- Check if models are installed: `ollama list`
- Click the refresh button in settings

### Slow Responses
- Use smaller models (3b or 7b)
- Consider using the same model for both roles
- Check your system resources

## Advanced Usage

### Custom System Context
The agent knows:
- Your operating system
- Your shell (PowerShell/bash)
- Current date
- Workspace structure
- Active file content

### Tool Capabilities
The agent can:
- Read and write files
- Search code semantically
- Run terminal commands (with approval)
- Validate TypeScript
- Run tests
- Analyze dependencies
- Create/delete files and folders

### Multi-File Operations
The agent can work across multiple files:
- Refactoring
- Feature implementation
- Bug fixes
- Code organization

## Getting Help

### Common Commands
- "Read the file X" - View file contents
- "Edit file X to do Y" - Make specific changes
- "Search for Z" - Find code patterns
- "Run tests" - Execute test suite
- "What files depend on X?" - See dependencies

### Example Requests
```
"Add error handling to the API calls in src/api/client.ts"

"Refactor the UserProfile component to use TypeScript"

"Fix the linting errors in the project"

"Create a new React component for displaying user stats"

"Update the README with installation instructions"
```

## Performance Optimization

### Hardware Recommendations

**Minimum:**
- 8GB RAM
- Use 3b models
- Same model for both roles

**Recommended:**
- 16GB RAM
- Use 7b-8b models
- Different models for each role

**Optimal:**
- 32GB+ RAM
- Use 13b-16b models
- Specialized models for each role

### Model Loading
- First request may be slow (model loading)
- Subsequent requests are faster
- Keep Ollama running in background

## What's Next?

Future enhancements planned:
- More advanced code analysis tools
- Sub-agent support for complex tasks
- Hooks for automation
- Custom instructions (steering files)
- Spec system for feature planning

---

**Need Help?** Check the logs in the terminal pane or restart the agent with the reset button.
