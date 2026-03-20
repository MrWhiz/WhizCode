# WhizCode - AI-Powered Code Editor

WhizCode is a modern, autonomous AI coding assistant with a VS Code-inspired interface, powered by local LLMs through Ollama. Built with React, TypeScript, Vite, and Electron.

## ✨ Features

### 🤖 Autonomous AI Agent
- **Strategic Planning** - Intelligent task decomposition and execution planning
- **Context Memory** - Persistent learning from past interactions and patterns
- **Adaptive Learning** - Behavioral adaptation based on success patterns
- **Code Intelligence** - Deep semantic code analysis and understanding
- **Enhanced MCP Integration** - Comprehensive powers marketplace and tool ecosystem
- **Vector Search Engine** - Semantic code search and contextual recommendations
- **Advanced Error Recovery** - Multi-layered error diagnosis and recovery strategies
- **Tool Result Caching** - Intelligent caching for performance optimization
- **Autonomous behavior** - Acts autonomously without forced approval steps
- **Single model simplicity** - One model handles all reasoning and coding tasks
- **Visible thinking process** - See the agent's reasoning when helpful
- **Smart loop prevention** - Automatic detection and correction of repetitive actions
- **Natural conversation flow** - Minimal, conversational responses

### 🛠️ Comprehensive Tool Suite
- **File Operations**: Read, write, edit, delete files with exact indentation matching
- **Code Analysis**: Semantic search, dependency analysis, TypeScript validation
- **Search & Discovery**: Pattern search, fuzzy file finding, grep-style search
- **Terminal Integration**: Run commands with approval system
- **Refactoring Tools**: Semantic rename, smart file relocation, multi-file edits
- **Project Management**: Test execution, validation, blast radius analysis

### 🎨 Professional IDE Experience
- **VS Code-inspired UI** - Familiar dark theme interface with activity bar
- **File Explorer** - Full-featured file tree with context menus
- **Integrated Terminal** - Built-in terminal with command execution
- **Multi-tab Editor** - Monaco editor with syntax highlighting
- **Source Control** - Git integration with status and diff views
- **Search Panel** - Project-wide search with results preview

### 🔧 Advanced Capabilities
- **Strategic Planning System** - Intelligent task decomposition with parallel execution
- **Context Memory & Learning** - Persistent memory of patterns, preferences, and strategies
- **Code Intelligence Engine** - Deep semantic analysis with refactoring suggestions
- **Adaptive Behavior** - AI learns from interactions and adapts approach
- **Error Recovery System** - Multi-layered fallback strategies for robust execution
- **Context Awareness** - Automatic diagnostics and git diff inclusion
- **Autopilot/Supervised Modes** - User control over file operation autonomy
- **Sub-Agent System** - Specialized agents for different tasks
- **Hooks System** - Event-driven automation and workflows
- **Steering Files** - Custom instructions and coding guidelines
- **File Synchronization** - Real-time sync between explorer and editor

## 🚀 Quick Start

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or newer)
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

### Single Model System
WhizCode uses a unified approach with one model handling all tasks:

**Unified Model (Brain + Hands 🧠🛠️)**
- Analyzes requests and makes decisions
- Plans approach and strategy
- Generates code and executes tools
- Handles precise syntax and formatting
- Understands context and requirements
- Best: Versatile models (qwen2.5-coder, deepseek-coder, gpt-4o, claude-3.5-sonnet)

This simplified approach reduces complexity while maintaining high performance.

### Core Components

```
┌─────────────────────────────────────────┐
│              React Frontend             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│  │Explorer │ │ Editor  │ │  Chat   │   │
│  │AI Insights│        │         │   │
│  └─────────┘ └─────────┘ └─────────┘   │
└─────────────────┬───────────────────────┘
                  │ IPC
┌─────────────────▼───────────────────────┐
│           Electron Main Process         │
│  ┌─────────────────────────────────┐   │
│  │    Enhanced Agent Loop          │   │
│  │  ┌──────────┐ ┌──────────────┐ │   │
│  │  │ Primary  │ │ Tool Model   │ │   │
│  │  │  Model   │ │   (Coding)   │ │   │
│  │  └──────────┘ └──────────────┘ │   │
│  │  ┌──────────┐ ┌──────────────┐ │   │
│  │  │Strategic │ │ Context      │ │   │
│  │  │Planner   │ │ Memory       │ │   │
│  │  └──────────┘ └──────────────┘ │   │
│  │  ┌──────────┐ ┌──────────────┐ │   │
│  │  │Learning  │ │ Code         │ │   │
│  │  │System    │ │ Intelligence │ │   │
│  │  └──────────┘ └──────────────┘ │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │           Services              │   │
│  │ • File Operations               │   │
│  │ • Code Analysis                 │   │
│  │ • Terminal Integration          │   │
│  │ • Sub-Agent System              │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### Key Services
- **Strategic Planner**: Intelligent task decomposition with parallel execution optimization
- **Context Memory**: Persistent learning from interactions, patterns, and user preferences  
- **Learning System**: Adaptive behavior based on success patterns and error recovery
- **Code Intelligence**: Deep semantic analysis with refactoring suggestions and metrics
- **IndexingService**: Semantic code search with vector embeddings
- **CodeGraphService**: Dependency analysis with Tree-sitter
- **DiffService**: Transactional multi-file changes with rollback
- **HooksSystem**: Event-driven automation and workflows
- **SteeringSystem**: Custom instructions and guidelines
- **SubAgentSystem**: Specialized agents for different tasks

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

### AI Insights Panel
Access advanced AI capabilities through the new AI Insights panel in the activity bar:

- **Learning Insights**: View patterns, preferences, strategies, and error analysis
- **Learning Metrics**: Track success rates, response times, and improvement trends  
- **Code Quality**: Monitor complexity, maintainability, coupling, and technical debt
- **Memory Statistics**: See accumulated knowledge including patterns and strategies

The AI continuously learns from your interactions and adapts its behavior for better performance over time.

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
│   └── main.tsx           # App entry point
├── electron/              # Electron main process
│   ├── main.ts            # Main process & agent loop
│   ├── subAgents.ts       # Sub-agent system
│   ├── hooksSystem.ts     # Event automation
│   └── steeringSystem.ts  # Custom instructions
├── docs/                  # Documentation
│   ├── SECURITY.md        # Security implementation
│   ├── BUILD_OPTIMIZATION.md
│   ├── DISTRIBUTION_GUIDE.md
│   └── FINAL_SIZE_REPORT.md
├── scripts/               # Utility scripts
│   ├── split-installer.ps1
│   ├── join-installer.ps1
│   └── analyze-bundle.js
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

### Packaging & Distribution

**Build installer:**
```bash
npm run package
```

**Split for email distribution (20MB limit):**
```bash
.\scripts\split-installer.ps1
```

**Rejoin split parts:**
```bash
.\scripts\join-installer.ps1
```

See [docs/DISTRIBUTION_GUIDE.md](docs/DISTRIBUTION_GUIDE.md) for detailed instructions.

**Build Information:**
- Installer Size: 99MB (compressed)
- Installed Size: ~390MB
- Build Time: ~2-3 minutes
- Supported OS: Windows 10+

## 🔍 Troubleshooting

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

See [LICENSE](LICENSE) file for full details.

## 🙏 Acknowledgments

- Built with [Ollama](https://ollama.ai/) for local LLM support
- Inspired by VS Code's interface design
- Uses [Monaco Editor](https://microsoft.github.io/monaco-editor/) for code editing
- Powered by [Electron](https://www.electronjs.org/) and [React](https://reactjs.org/)

---

**Ready to code with AI?** Open a folder, ask the agent to help, and watch it work autonomously! 🚀