# WhizCode - AI-Powered Code Editor

WhizCode is a modern, autonomous AI coding assistant with a VS Code-inspired interface, powered by local LLMs through Ollama. Built with React, TypeScript, Vite, and Tauri.

## ✨ Features

### 🤖 Autonomous Multi-Agent System

- **Sub-Agent Orchestration** - Complex tasks are routed through specialized personas: `STRATEGIC PLANNER`, `RESEARCHER`, `EXECUTOR`, and `REVIEWER`.
- **Knowledge Distillation (The Brain)** - Automatically extracts overarching architectural rules and stores them in `.whizcode/knowledge` for cross-session context.
- **Structured Workflows & Modular Skills** - Define custom SOPs in `.whizcode/workflows/*.md` for the agent to follow precisely during strategic planning.
- **Adaptive Learning** - Behavioral adaptation based on success patterns.
- **Code Intelligence** - Deep semantic code analysis and understanding.
- **Enhanced MCP Integration** - Comprehensive marketplace and tool ecosystem.
- **Vector Search Engine** - Semantic code search and contextual RAG recommendations.
- **Advanced Error Recovery** - Autonomous multi-layered error diagnosis and self-healing.
- **Live Preview Environment** - Agent can instantly render and interact with frontend code (React/Vite).
- **Rich Media & Tech Graphics** - Native support for AI-generated architecture `mermaid` diagrams and UI mockups.
- **Visible Thinking Process** - Watch the agent's reasoning live via a transparent, animated Glassmorphism UI.

### 🛠️ Comprehensive Tool Suite

- **File Operations**: Read, write, edit, delete files with exact indentation matching.
- **Code Analysis**: Semantic search, dependency analysis, TypeScript validation.
- **Web Browsing**: `search_web` and `read_url_content` for live API research.
- **Rich Media**: `generate_image` for UI prototyping and native `MermaidDiagram` support.
- **Terminal Integration**: Run background processes and compilation checks.
- **Project Management**: Built-in "Feature Specs" tab for interactive AI-driven product management and checklist tracking.

### 🎨 Professional IDE Experience

- **VS Code-inspired UI** - Familiar dark theme interface with activity bar
- **File Explorer** - Full-featured file tree with context menus
- **Integrated Terminal** - Built-in terminal with command execution
- **Multi-tab Editor** - Monaco editor with syntax highlighting
- **Source Control** - Git integration with status and diff views
- **Search Panel** - Project-wide search with results preview

### 🔧 Advanced Capabilities

- **Multi-Persona Planning System** - Orchestrates tasks via specialized phases instead of single monolithic LLM requests.
- **Context Memory & 'Brain Health'** - A dedicated dashboard to monitor vector indexing, distilled interactions, and self-healing recovery rates.
- **Code Intelligence Engine** - Deep semantic analysis with refactoring suggestions.
- **Feature Specs Dashboard** - Break down large requests into tracked, AI-managed task lists.
- **Autopilot/Supervised Modes** - User control over file operation autonomy.
- **Custom Workflow Protocol** - Agent dynamically discovers custom company standard workflows located in your workspace.

## 🚀 Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or newer)
- [Rust](https://rustup.rs/) (required for Tauri backend compilation)
- [Ollama](https://ollama.ai/) (for local LLMs)
- Recommended: 16GB+ RAM for optimal performance

### Installation

1. **Clone and install:**

   ```bash
   git clone <repository-url>
   cd WhizCode
   npm install
   ```

2. **Install Ollama model:**

   ```bash
   # Recommended model for all tasks
   ollama pull qwen2.5-coder:latest
   ```

3. **Start Ollama:**

   ```bash
   ollama serve
   ```

4. **Run WhizCode:**
   ```bash
   npm run dev
   ```

### Configuration

1. Open WhizCode
2. Click the settings icon in the chat panel
3. Expand "Agent Configuration"
4. Set Primary Model: `llama3:8b` (for reasoning)
5. Set Tool Model: `deepseek-coder-v2:16b` (for coding)

## 💡 Usage Examples

### Basic Commands

```
"Open a folder and show me the project structure"
"Read the package.json file"
"Add a new React component called UserCard"
"Fix any TypeScript errors in the project"
"Search for all TODO comments"
```

### Advanced Operations

```
"Analyze the codebase and suggest improvements based on best practices"
"Learn from my coding patterns and adapt your suggestions accordingly"
"Create a strategic plan for implementing user authentication with error recovery"
"Refactor the UserProfile component using learned patterns from previous successes"
"Analyze code quality metrics and provide intelligent refactoring suggestions"
```

## 🏗️ Architecture

## 🏗️ Architecture

### Multi-Agent Orchestration

WhizCode has evolved past monolithic LLM execution. It now uses a phased orchestration pipeline:

**The Orchestrator Loop (🧠 + 🛠️)**

- **Strategic Planner**: Analyzes requests and outputs a rigid JSON execution plan.
- **Researcher**: Investigates unknown APIs via Web Search and checks internal Context Memory.
- **Executor**: Generates code, edits files, and interacts with the terminal.
- **Reviewer**: Audits changes for compilation errors and logic bugs before proceeding.

This multi-stage approach drastically reduces hallucinations and improves code reliability.

### Core Components

```
┌─────────────────────────────────────────┐
│              React Frontend             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│  │Explorer │ │ Editor  │ │Live Prev│   │
│  │Brain/Spec│ └───┬─────┘ └───────┬─┘   │
│  └─────────┘     │               │     │
└──────────────────┼───────────────┼─────┘
                   │    Tauri API  │
┌──────────────────▼───────────────▼────┐
│           Tauri Backend (Rust)        │
│  ┌─────────────────────────────────┐  │
│  │    Multi-Persona Agent Loop     │  │
│  │  ┌──────────┐ ┌──────────────┐  │  │
│  │  │ Planner  │ │  Researcher  │  │  │
│  │  └──────────┘ └──────────────┘  │  │
│  │  ┌──────────┐ ┌──────────────┐  │  │
│  │  │ Executor │ │   Reviewer   │  │  │
│  │  └──────────┘ └──────────────┘  │  │
│  └─────────────────────────────────┘  │
│  ┌─────────────────────────────────┐  │
│  │           Core Services         │  │
│  │ • RAG / Vector DB               │  │
│  │ • Distillation (Brain)          │  │
│  │ • Workflows / Skills Engine     │  │
│  │ • Image / Media Generation      │  │
│  └─────────────────────────────────┘  │
└───────────────────────────────────────┘
```

### Key Services

- **Knowledge Distillation**: Persistent extraction of structural context between conversation resets.
- **Error Recovery System**: Multi-layered fallback strategies for compiler-level healing.
- **Code Intelligence**: AST analysis and cross-file refactoring tracking.
- **IndexingService**: Semantic code search using local `.whizcode` vector embeddings.
- **Feature Specs Dashboard**: UI-driven tracking of agent-generated implementation checklists.

## 🎛️ Configuration Options

### Model Configurations

**Recommended (Ollama):**

```
Model: qwen2.5-coder:latest
```

**Alternative Options:**

```
Model: deepseek-coder-v2:latest (Ollama)
Model: llama3:latest (Ollama)
Model: gpt-4o (OpenAI)
Model: gemini-1.5-flash (Gemini)
Model: anthropic.claude-3-5-sonnet-20241022-v2:0 (AWS Bedrock)
```

**Performance Tiers:**

- **High Performance**: qwen2.5-coder:32b, deepseek-coder-v2:16b
- **Balanced**: qwen2.5-coder:7b, llama3:8b
- **Fast & Light**: qwen2.5-coder:3b, llama3:3b

### Operating Modes

**Autopilot Mode:**

- Agent modifies files autonomously
- Faster workflow
- Best for trusted operations

**Supervised Mode:**

- Agent asks permission before file operations
- Safer for critical projects
- User maintains control

## 🔧 Advanced Features

### Brain Health Dashboard

Access cognitive diagnostics through the Brain Health panel in the activity bar:

- **Context Memory & RAG Stats**: View exactly how many code patterns and chunks are indexed.
- **Hero Recovery**: Track the AI's success rate at autonomously fixing compiler/logic errors.
- **Knowledge Distillation**: Monitor overarching project rules the AI has extracted into `.whizcode/knowledge`.

### Feature Specs Panel

Turn abstract ideas into structured development plans:

- Request the AI to "Create a spec for feature X".
- WhizCode generates a full markdown specification and breaks it down into an interactive checkbox task list.
- The agent sequentially executes tasks from the checklist, reporting back progress.

### Live Preview Environment

Web preview integrations allow WhizCode to build React/Vite applications and instantly verify the visual output side-by-side with the editor.

### Context Menu Operations

Right-click in the file explorer for:

- Create new files/folders
- Rename and delete items
- Copy paths (absolute/relative)
- Reveal in system explorer
- Open terminal at location

### File Synchronization

- Automatic tab closure when files are deleted
- Tab name updates when files are renamed
- Real-time sync between explorer and editor
- External change detection

### Iteration Management

- Automatic continuation prompts when iteration limit reached
- User choice to extend or stop execution
- Dynamic threshold adjustment

### Permission System

- Automatic approval for file operations
- Manual approval required for terminal commands
- "Always Run" option for trusted commands
- Granular control over agent autonomy

## 📁 Project Structure

```
WhizCode/
├── src/                    # React frontend
│   ├── components/         # UI components
│   │   ├── Chat/          # Chat interface
│   │   ├── Editor/        # Code editor
│   │   ├── Explorer/      # File explorer
│   │   └── Terminal/      # Terminal pane
│   ├── types/             # TypeScript definitions
│   ├── lib/               # Utilities
│   │   └── tauri-api.ts   # Tauri API wrapper
│   └── main.tsx           # App entry point
├── src-tauri/             # Tauri backend (Rust)
│   ├── src/
│   │   ├── main.rs        # Tauri app entry
│   │   ├── commands/      # Tauri commands
│   │   │   ├── fs.rs      # File operations
│   │   │   ├── system.rs  # System info
│   │   │   └── ...
│   │   ├── state.rs       # App state
│   │   └── error.rs       # Error handling
│   └── tauri.conf.json    # Tauri config
├── docs/                  # Documentation
│   ├── SECURITY.md        # Security implementation
│   └── LICENSE_GUIDE.md   # License details
├── public/                # Static assets
└── package.json           # Dependencies & scripts
```

## 🛠️ Development

### Building for Production

```bash
npm run build
```

### Development Mode

```bash
npm run dev
```

### Linting

```bash
npm run lint
```

### Building Tauri App

```bash
# Build frontend
npm run vite:build

# Build Tauri app (includes frontend)
cargo build --release -C src-tauri
```

## � Security

WhizCode includes comprehensive security measures:

- **Path Traversal Prevention** - Validates all file operations stay within workspace
- **Command Injection Prevention** - Uses safe argument arrays for external commands
- **Input Validation** - Strict validation on all IPC handlers
- **Sensitive Data Encryption** - Azure tokens encrypted with AES-256-CBC
- **Secure IPC Communication** - Context isolation and preload validation

See [docs/SECURITY.md](docs/SECURITY.md) for detailed security implementation.

WhizCode is optimized for minimal size and maximum performance through Tauri's native backend and Vite's efficient bundling.

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)** with **Commons Clause** and **Trademark Protection**.

**What this means:**

- ✅ **Open Source** - Anyone can view, modify, and use the code
- ✅ **Community Contributions** - Improvements must be shared back
- ✅ **Free to Use** - No cost for personal or internal use
- ❌ **No Commercial Sale** - Cannot sell WhizCode or derivatives
- ❌ **No Rebranding** - Cannot rebrand as "WhizCode Pro" or similar
- ❌ **No Proprietary Forks** - Cannot create closed-source versions

**You can:**

- Use WhizCode for any purpose (free)
- Modify the code for your needs
- Use it in your projects (non-commercially)
- Distribute modified versions (non-commercially, with attribution)
- Contribute improvements back to the project

**You cannot:**

- Sell WhizCode or any derivative
- Offer WhizCode as a paid service
- Charge for hosting or support
- Rebrand it as your own product
- Create proprietary versions
- Remove attribution or license notices

See [LICENSE](LICENSE) file and [docs/LICENSE_GUIDE.md](docs/LICENSE_GUIDE.md) for full details.

## � Troubleshooting

### Common Issues

**"Ollama not detected"**

```bash
ollama serve
```

**"Model not found"**

```bash
ollama list  # Check installed models
ollama pull <model-name>
```

**Agent is slow**

- Use smaller models (3b or 7b)
- Use same model for both roles
- Check RAM usage

**Agent keeps repeating actions**

- Built-in loop detection will self-correct
- Click "Stop" and rephrase request if stuck

### Build Issues

- Clear `dist/` directory
- Run `npm install` to ensure dependencies are installed
- Check Node.js and Rust version compatibility

### Installer Issues

- Ensure you have the necessary Tauri build dependencies installed for your platform.

### Performance Optimization

**Hardware Requirements:**

- Minimum: 8GB RAM, use 3b models
- Recommended: 16GB RAM, use 7b-8b models
- Optimal: 32GB+ RAM, use 13b-16b models

**Response Times:**

- First request: 2-10s (model loading)
- Subsequent: 0.5-3s (model cached)
- Tool execution: 0.1-1s (file operations)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

When contributing, your code will be licensed under AGPL-3.0 + Commons Clause. See [docs/LICENSE_GUIDE.md](docs/LICENSE_GUIDE.md) for details.

## 🙏 Acknowledgments

- Built with [Ollama](https://ollama.ai/) for local LLM support
- Inspired by VS Code's interface design
- Uses [Monaco Editor](https://microsoft.github.io/monaco-editor/) for code editing
- Powered by [Tauri](https://tauri.app/) and [React](https://reactjs.org/)
- Backend built with [Rust](https://www.rust-lang.org/)

---

**Ready to code with AI?** Open a folder, ask the agent to help, and watch it work autonomously! 🚀
