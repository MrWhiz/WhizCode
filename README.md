# WhizCode - WhizCode-Style Autonomous AI Coding IDE

WhizCode is an autonomous AI coding assistant with a VS Code-inspired interface, powered by local LLMs through Ollama. Now featuring WhizCode-style autonomous agent behavior with flexible multi-model support.

Built with React, TypeScript, Vite, and Electron.

## ✨ What's New - WhizCode Alignment

WhizCode now uses a **unified autonomous agent** inspired by WhizCode, with:
- 🤖 **Autonomous behavior** - No forced approval steps, natural conversation flow
- 🧠 **Multi-model optimization** - Use different models for reasoning vs coding
- 💭 **Visible thinking** - See the agent's reasoning process
- 🔄 **Smart loop prevention** - Automatic detection and correction
- 📝 **Concise responses** - Minimal, conversational summaries

See [WHIZCODE-SETUP-GUIDE.md](WHIZCODE-SETUP-GUIDE.md) for detailed configuration.

## Features

### Core Capabilities
- **Autonomous Agent**: WhizCode-style behavior with proactive tool usage
- **Multi-Model Support**: Separate models for reasoning and code generation
- **15+ Tools**: File operations, search, terminal, validation, and more
- **9 New Tools**: readCode, editCode, getDiagnostics, grepSearch, fileSearch, readMultipleFiles, semanticRename, smartRelocate, strReplace
- **Semantic Search**: Vector-based code search with Voyage AI
- **Dependency Analysis**: Blast radius calculation for code changes
- **VS Code Aesthetic**: Familiar dark theme interface
- **Local-First**: Works with Ollama, OpenAI, or Gemini

### Agent Features
- Read, write, and edit files with exact indentation matching
- Search code semantically or with patterns
- Run terminal commands (with approval)
- Validate TypeScript projects
- Run test suites
- Analyze file dependencies
- Multi-file diff operations with rollback

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or newer)
- [Ollama](https://ollama.ai/) (for local LLMs)
- Recommended: 16GB+ RAM for optimal performance

### Recommended Ollama Models

```bash
# For reasoning (Primary Model)
ollama pull llama3:8b

# For coding (Tool Model)
ollama pull deepseek-coder-v2:16b

# Or use a balanced model for both
ollama pull qwen2.5-coder:7b
```

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd WhizCode
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Start Ollama (in a separate terminal):
   ```bash
   ollama serve
   ```

## How to Execute

### Development Mode

```bash
npm run dev
```

This will:
- Start the Vite dev server for hot-reloading
- Launch the Electron application

### Building for Production

```bash
npm run build
```

This creates optimized bundles for both the frontend and Electron main process.

## Configuration

### Multi-Model Setup

WhizCode supports using different models for different purposes:

**Primary Model** (Reasoning & Planning):
- Analyzes requests and makes decisions
- Best: Models with strong reasoning (llama3, mistral, qwen)

**Tool Model** (Code Generation):
- Generates code and executes tools
- Best: Code-specialized models (deepseek-coder, codellama)

### Configuration Examples

**Optimized (Recommended):**
```
Primary: llama3:8b (Ollama)
Tool: deepseek-coder-v2:16b (Ollama)
```

**Balanced:**
```
Primary: qwen2.5-coder:7b (Ollama)
Tool: qwen2.5-coder:7b (Ollama)
```

**Hybrid:**
```
Primary: gpt-4o (OpenAI)
Tool: deepseek-coder-v2:16b (Ollama)
```

### How to Configure

1. Open WhizCode
2. Click the settings icon in the chat panel
3. Expand "Agent Configuration"
4. Select your Primary Model (for reasoning)
5. Select your Tool Model (for coding)
6. Add API keys if using OpenAI or Gemini

See [WHIZCODE-SETUP-GUIDE.md](WHIZCODE-SETUP-GUIDE.md) for detailed configuration guide.

## Usage

### Opening a Workspace

1. Click "File" → "Open Folder"
2. Select your project directory
3. The agent will index your project automatically

### Chatting with the Agent

Simply type your request in the chat panel:

```
"Add error handling to the API client"
"Refactor the UserProfile component to use TypeScript"
"Fix the linting errors in src/utils.ts"
"Create a new React component for displaying charts"
```

The agent will:
- Analyze your request
- Use appropriate tools automatically
- Show its thinking process
- Provide concise summaries

### Permission System

The agent will ask for approval before:
- Running terminal commands
- You can check "Always Run" to auto-approve

All other operations (reading/writing files) happen automatically.

## Architecture

### Agent System
- **Unified Agent**: Single autonomous agent (no forced planning phase)
- **Multi-Model**: Flexible model selection for different tasks
- **Tool System**: 24 tools for file operations, search, execution, and refactoring
- **Loop Prevention**: Automatic detection of repetitive actions
- **Context Management**: Project manifest, active file, conversation history

### Services
- **IndexingService**: Semantic code search with Voyage AI + LanceDB
- **CodeGraphService**: Dependency analysis with Tree-sitter
- **DiffService**: Transactional multi-file changes with rollback

### Tech Stack
- **Frontend**: React, TypeScript, Vite
- **Backend**: Electron, Node.js
- **AI**: Ollama, OpenAI, Gemini support
- **Search**: Voyage AI embeddings, LanceDB vector DB
- **Parsing**: Tree-sitter for code analysis

## Project Structure

- `src/` - Contains the React frontend code (Components, CSS).
- `electron/` - Contains the Electron main process code and IPC handlers.
- `public/` - Static assets.
- `index.html` - The main HTML entry point for the Vite application.
- `vite.config.ts` - Vite configuration, including Electron plugin setup.
