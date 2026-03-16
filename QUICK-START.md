# WhizCode Quick Start

## 🚀 Get Running in 3 Steps

### 1. Install Ollama Models
```bash
# For reasoning
ollama pull llama3:8b

# For coding  
ollama pull deepseek-coder-v2:16b
```

### 2. Start the App
```bash
npm install
npm run dev
```

### 3. Configure Models
- Click settings icon in chat
- Set Primary Model: `llama3:8b`
- Set Tool Model: `deepseek-coder-v2:16b`

## 💡 First Commands to Try

```
"Open a folder and show me the project structure"

"Read the package.json file"

"Add a new React component called UserCard"

"Fix any TypeScript errors in the project"

"Search for all TODO comments"
```

## 🎯 Key Differences from Before

| Old Behavior | New Behavior |
|-------------|--------------|
| Creates plan → waits for approval | Acts autonomously |
| Two separate phases | Single unified flow |
| Planner/Executor models | Primary/Tool models |
| Formal, structured | Natural, conversational |

## ⚙️ Model Roles Explained

**Primary Model** = The Brain 🧠
- Thinks and plans
- Makes decisions
- Understands context
- Example: llama3, mistral

**Tool Model** = The Hands 🛠️
- Writes code
- Executes tools
- Generates output
- Example: deepseek-coder, codellama

## 🔧 Common Issues

**"Ollama not detected"**
```bash
ollama serve
```

**"Model not found"**
```bash
ollama list  # Check installed models
ollama pull <model-name>
```

**"Agent is slow"**
- Use smaller models (3b or 7b)
- Use same model for both roles
- Check RAM usage

## 📚 More Help

- Full guide: [WHIZCODE-SETUP-GUIDE.md](WHIZCODE-SETUP-GUIDE.md)
- Changes: [.agents/CHANGES.md](.agents/CHANGES.md)
- Details: [.agents/implementation-summary.md](.agents/implementation-summary.md)

## 🎉 You're Ready!

Open a folder, ask the agent to help, and watch it work autonomously.
