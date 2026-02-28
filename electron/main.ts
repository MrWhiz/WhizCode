import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'
import { exec } from 'node:child_process'
import { promisify } from 'node:util'
import * as fs from 'node:fs/promises'
import fetch from 'cross-fetch'
import * as os from 'node:os'
import { createRequire } from 'node:module'

const _require = createRequire(import.meta.url)
const pty = _require('node' + '-pty')

const execAsync = promisify(exec)

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

process.env.APP_ROOT = join(__dirname, '..')

export const VITE_DEV_SERVER_URL = process.env['VITE_DEV_SERVER_URL']
export const MAIN_DIST = join(process.env.APP_ROOT, 'dist-electron')
export const RENDERER_DIST = join(process.env.APP_ROOT, 'dist')

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL ? join(process.env.APP_ROOT, 'public') : RENDERER_DIST

let win: BrowserWindow | null
let ptyProcess: any = null

// Ollama Configuration
const OLLAMA_URL = 'http://127.0.0.1:11434/api/chat';
const MODEL_NAME = 'llama3'; // User should change this to their local model if needed

function createWindow() {
  win = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    webPreferences: {
      preload: join(__dirname, 'preload.mjs'),
      nodeIntegration: false,
      contextIsolation: true,
    },
    titleBarStyle: 'hidden',
    titleBarOverlay: {
      color: '#1e1e1e', // VSCode Dark Theme 
      symbolColor: '#cccccc',
      height: 30
    },
    backgroundColor: '#1e1e1e'
  })

  // Open DevTools automatically if in dev environment
  if (VITE_DEV_SERVER_URL) {
    win.loadURL(VITE_DEV_SERVER_URL)
  } else {
    win.loadFile(join(RENDERER_DIST, 'index.html'))
  }
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
    win = null
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})

app.whenReady().then(createWindow)

// ====== AGENTIC CODING FRAMEWORK ======

// Directories and extensions to skip when scanning the workspace
const SKIP_DIRS = new Set(['node_modules', '.git', 'dist', 'dist-electron', '.next', '__pycache__', '.venv', 'venv', '.cache', 'coverage', '.idea', '.vscode']);
const BINARY_EXTS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.ico', '.webp', '.svg', '.woff', '.woff2', '.ttf', '.eot', '.mp3', '.mp4', '.zip', '.tar', '.gz', '.exe', '.dll', '.so', '.dylib', '.lock', '.pdf']);

async function readDirectoryRecursive(dirPath: string, maxFiles = 200): Promise<{ path: string, content: string }[]> {
  const results: { path: string, content: string }[] = [];
  async function walk(currentPath: string) {
    if (results.length >= maxFiles) return;
    try {
      const entries = await fs.readdir(currentPath, { withFileTypes: true });
      for (const entry of entries) {
        if (results.length >= maxFiles) break;
        const fullPath = join(currentPath, entry.name);
        if (entry.isDirectory()) {
          if (!SKIP_DIRS.has(entry.name) && !entry.name.startsWith('.')) {
            await walk(fullPath);
          }
        } else {
          const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
          if (!BINARY_EXTS.has(ext)) {
            try {
              const stat = await fs.stat(fullPath);
              if (stat.size < 100_000) {
                const content = await fs.readFile(fullPath, 'utf-8');
                results.push({ path: fullPath, content });
              }
            } catch { /* skip unreadable files */ }
          }
        }
      }
    } catch { /* skip unreadable dirs */ }
  }
  await walk(dirPath);
  return results;
}

// List directory contents (non-recursive, with metadata)
async function listDirectory(dirPath: string): Promise<string> {
  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });
    const lines: string[] = [];
    for (const entry of entries) {
      const fullPath = join(dirPath, entry.name);
      try {
        const stat = await fs.stat(fullPath);
        const type = entry.isDirectory() ? 'DIR ' : 'FILE';
        const size = entry.isDirectory() ? '' : ` (${stat.size} bytes)`;
        lines.push(`${type} ${entry.name}${size}`);
      } catch {
        lines.push(`???? ${entry.name}`);
      }
    }
    return lines.join('\n') || '(empty directory)';
  } catch (e: any) {
    return `Error listing directory: ${e.message}`;
  }
}

// Search files for a pattern (grep-like)
async function searchFiles(rootDir: string, pattern: string, includeGlob?: string): Promise<string> {
  const results: string[] = [];
  const maxResults = 50;
  const regex = new RegExp(pattern, 'gi');

  async function walk(currentPath: string) {
    if (results.length >= maxResults) return;
    try {
      const entries = await fs.readdir(currentPath, { withFileTypes: true });
      for (const entry of entries) {
        if (results.length >= maxResults) break;
        const fullPath = join(currentPath, entry.name);
        if (entry.isDirectory()) {
          if (!SKIP_DIRS.has(entry.name) && !entry.name.startsWith('.')) {
            await walk(fullPath);
          }
        } else {
          // Apply include filter if specified
          if (includeGlob) {
            const ext = entry.name.split('.').pop()?.toLowerCase();
            const filterExt = includeGlob.replace('*.', '').toLowerCase();
            if (ext !== filterExt) continue;
          }
          const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
          if (BINARY_EXTS.has(ext)) continue;
          try {
            const stat = await fs.stat(fullPath);
            if (stat.size > 100_000) continue;
            const content = await fs.readFile(fullPath, 'utf-8');
            const lines = content.split('\n');
            for (let i = 0; i < lines.length; i++) {
              if (regex.test(lines[i])) {
                const relPath = fullPath.replace(rootDir, '').replace(/^[\\/]/, '');
                results.push(`${relPath}:${i + 1}: ${lines[i].trim()}`);
                if (results.length >= maxResults) break;
              }
              regex.lastIndex = 0; // reset regex state
            }
          } catch { /* skip */ }
        }
      }
    } catch { /* skip */ }
  }

  await walk(rootDir);
  return results.length > 0 ? results.join('\n') : `No matches found for "${pattern}".`;
}

// Resolve a path from the agent (could be relative) to an absolute path
function resolvePath(agentPath: string, workspacePath: string | null): string {
  if (!agentPath) return agentPath;
  // If already absolute, use as-is
  if (agentPath.match(/^[A-Za-z]:[\\/]/) || agentPath.startsWith('/')) {
    return agentPath;
  }
  // Otherwise resolve relative to workspace
  if (workspacePath) {
    return join(workspacePath, agentPath);
  }
  return agentPath;
}

// ====== SYSTEM PROMPT ======

const SYSTEM_PROMPT = `You are "Antigravity Agent", a high-performance autonomous software engineer.
Your primary directive is to ACT on the user's project, NOT give instructions.

## CORE DIRECTIVES

1. **AUTOMATE EVERYTHING**: If a user asks to "update", "fix", or "add" something, you MUST use tools to do it. NEVER tell the user how to do it themselves.
2. **DOER, NOT TEACHER**: Do not explain how code works unless specifically asked. Focus on execution.
3. **TOOL FIRST**: Every turnaround that involves a code change MUST contain a JSON tool call. 

## OPERATING MODE (THOUGHT -> ACTION -> OBSERVATION)

1. **Thought**: "I need to fix the bug in X. I will read the file, then apply the fix."
2. **Action**: {"tool": "read_file", "path": "src/App.tsx"}
3. **Observation**: (The result of the tool)
...Repeat until finished...

## AVAILABLE TOOLS (JSON ONLY)

### read_file
{"tool": "read_file", "path": "file.ts"}
Returns content with line numbers. Use this to prepare for line-based edits.

### replace_lines
{"tool": "replace_lines", "path": "file.ts", "startLine": 10, "endLine": 15, "content": "new code"}
Replaces lines [startLine, endLine] (inclusive) with the new content. This is the MOST RELIABLE way to edit.

### insert_code
{"tool": "insert_code", "path": "file.ts", "line": 20, "content": "code to insert"}
Inserts code AFTER the specified line number.

### edit_file
{"tool": "edit_file", "path": "file.ts", "edits": [{"search": "exact match", "replace": "replacement"}]}
Only use this for very simple, unique string replacements.

### write_file / run_command / list_directory / search_files
Use these as previously documented.

## CRITICAL: NO CHATTING DURING EDITS
When updating files, do not provide markdown code blocks in your thoughts. Put the code ONLY inside the tool JSON. 
If you give instructions instead of acting, you have FAILED.`;

let conversationHistory: any[] = [
  { role: 'system', content: SYSTEM_PROMPT }
];

let workspaceContextLoaded = false;

// ====== LLM PROVIDER CALLS ======

async function callAI(messages: any[], provider: string, config: any) {
  try {
    if (provider === 'openai') {
      const response = await fetch('https://api.openai.com/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${config.openaiKey}`
        },
        body: JSON.stringify({
          model: 'gpt-4o-mini',
          messages: messages,
          temperature: 0.2
        })
      });
      if (!response.ok) throw new Error(`OpenAI HTTP Error: ${response.status} ${await response.text()}`);
      const data: any = await response.json();
      return data.choices[0].message.content;
    } else if (provider === 'gemini') {
      const geminiMessages = messages.map(m => {
        if (m.role === 'system') return { role: 'user', parts: [{ text: "SYSTEM INSTRUCTION: " + m.content }] };
        return { role: m.role === 'assistant' ? 'model' : m.role, parts: [{ text: m.content }] };
      });
      const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key=${config.geminiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ contents: geminiMessages })
      });
      if (!response.ok) throw new Error(`Gemini HTTP Error: ${response.status} ${await response.text()}`);
      const data: any = await response.json();
      return data.candidates[0].content.parts[0].text;
    } else {
      const response = await fetch(OLLAMA_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: config.ollamaModel || MODEL_NAME,
          messages: messages,
          stream: false
        })
      });
      if (!response.ok) throw new Error(`Ollama HTTP Error: ${response.status}`);
      const data: any = await response.json();
      return data.message.content;
    }
  } catch (error: any) {
    console.error("Error communicating with AI Provider:", error);
    throw error;
  }
}

// ====== TOOL PARSER ======

function tryParseToolCall(response: string): any | null {
  if (!response) return null;
  const trimmed = response.trim();

  // Try direct JSON parse
  try {
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      const parsed = JSON.parse(trimmed);
      if (parsed.tool) return parsed;
    }
  } catch { /* continue to other strategies */ }

  // Try extracting JSON from markdown code blocks
  const codeBlockMatch = trimmed.match(/```(?:json)?\s*\n?({[\s\S]*?})\s*\n?```/);
  if (codeBlockMatch) {
    try {
      const parsed = JSON.parse(codeBlockMatch[1]);
      if (parsed.tool) return parsed;
    } catch { /* not valid JSON */ }
  }

  // Try finding a JSON object anywhere in the response
  const jsonMatch = trimmed.match(/({\s*"tool"\s*:\s*"[^"]+"[\s\S]*?})/);
  if (jsonMatch) {
    try {
      const parsed = JSON.parse(jsonMatch[1]);
      if (parsed.tool) return parsed;
    } catch { /* not valid JSON */ }
  }

  return null;
}

// ====== TOOL EXECUTOR ======

async function executeToolCall(toolData: any, workspacePath: string | null): Promise<string> {
  const resolvedPath = toolData.path ? resolvePath(toolData.path, workspacePath) : '';
  console.log(`\n🛠️  [${toolData.tool}] ${resolvedPath || toolData.command || toolData.pattern || ''}`);

  try {
    switch (toolData.tool) {
      case 'read_file': {
        const content = await fs.readFile(resolvedPath, 'utf-8');
        const lines = content.split('\n');
        // Add line numbers for context
        return lines.map((line, i) => `${i + 1}: ${line}`).join('\n');
      }

      case 'write_file': {
        // Ensure parent directory exists
        const dir = dirname(resolvedPath);
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(resolvedPath, toolData.content, 'utf-8');
        const lineCount = toolData.content.split('\n').length;
        return `✅ Successfully wrote ${lineCount} lines to ${toolData.path}`;
      }

      case 'edit_file': {
        let content = await fs.readFile(resolvedPath, 'utf-8');
        const edits = toolData.edits || [];
        let editCount = 0;

        for (const edit of edits) {
          // Check for exact match
          if (content.includes(edit.search)) {
            content = content.replace(edit.search, edit.replace);
            editCount++;
          } else {
            // Provide helpful feedback for nearly-matching strings
            const searchTrimmed = edit.search.trim();
            const contentTrimmed = content.replace(/\s+/g, ' ');

            if (contentTrimmed.includes(searchTrimmed.replace(/\s+/g, ' '))) {
              return `❌ edit_file failed for ${toolData.path}: The search string exists but whitespace/indentation did not match exactly. 
Please 'read_file' again to get the EXACT indentation or use the 'write_file' tool to overwrite the file if the edit is complex.
Searched for: "${edit.search.substring(0, 50)}..."`;
            } else {
              return `❌ edit_file failed: could not find the following code block in ${toolData.path}:\n\n${edit.search}\n\nMake sure you have the latest content via 'read_file'.`;
            }
          }
        }
        await fs.writeFile(resolvedPath, content, 'utf-8');
        return `✅ Applied ${editCount} edit(s) to ${toolData.path}`;
      }

      case 'list_directory': {
        return await listDirectory(resolvedPath || workspacePath || '.');
      }

      case 'search_files': {
        const searchRoot = workspacePath || '.';
        return await searchFiles(searchRoot, toolData.pattern, toolData.include);
      }

      case 'run_command': {
        const cwd = workspacePath || process.cwd();
        try {
          const { stdout, stderr } = await execAsync(toolData.command, {
            cwd,
            timeout: 30000, // 30s timeout
            maxBuffer: 1024 * 1024 // 1MB
          });
          const output = (stdout + (stderr ? '\nSTDERR: ' + stderr : '')).trim();
          return output || '(command completed with no output)';
        } catch (e: any) {
          // execAsync throws on non-zero exit codes — still return output
          return `Command exited with error:\n${e.stdout || ''}\n${e.stderr || ''}\n${e.message}`.trim();
        }
      }

      case 'create_directory': {
        await fs.mkdir(resolvedPath, { recursive: true });
        return `✅ Created directory: ${toolData.path}`;
      }

      case 'delete_file': {
        await fs.unlink(resolvedPath);
        return `✅ Deleted: ${toolData.path}`;
      }

      case 'replace_lines': {
        const content = await fs.readFile(resolvedPath, 'utf-8');
        const lines = content.split('\n');
        const startIdx = Math.max(0, toolData.startLine - 1);
        const endIdx = Math.min(lines.length, toolData.endLine);

        const newLines = [
          ...lines.slice(0, startIdx),
          toolData.content,
          ...lines.slice(endIdx)
        ];

        await fs.writeFile(resolvedPath, newLines.join('\n'), 'utf-8');
        return `✅ Replaced lines ${toolData.startLine}-${toolData.endLine} in ${toolData.path}`;
      }

      case 'insert_code': {
        const content = await fs.readFile(resolvedPath, 'utf-8');
        const lines = content.split('\n');
        const insertIdx = Math.min(lines.length, toolData.line);

        const newLines = [
          ...lines.slice(0, insertIdx),
          toolData.content,
          ...lines.slice(insertIdx)
        ];

        await fs.writeFile(resolvedPath, newLines.join('\n'), 'utf-8');
        return `✅ Inserted code after line ${toolData.line} in ${toolData.path}`;
      }

      default:
        return `❌ Unknown tool: "${toolData.tool}". Available tools: read_file, replace_lines, insert_code, write_file, edit_file, list_directory, search_files, run_command`;
    }
  } catch (e: any) {
    return `❌ Tool error (${toolData.tool}): ${e.message}`;
  }
}

// ====== AGENT LOOP ======

const MAX_AGENT_ITERATIONS = 20;

async function runAgentLoop(
  userMessage: string,
  provider: string,
  config: any,
  workspacePath: string | null
): Promise<{ finalResponse: string; steps: any[] }> {
  const steps: any[] = [];

  // Inject workspace context on first real user message
  if (workspacePath && !workspaceContextLoaded) {
    workspaceContextLoaded = true;
    console.log('📂 Reading workspace for context:', workspacePath);
    win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'running', summary: 'Reading project files...' });

    const files = await readDirectoryRecursive(workspacePath, 200); // Increased limit
    if (files.length > 0) {
      let manifest = `\n\n## CORE PROJECT CONTEXT\n\n### Project Root: ${workspacePath}\n\n#### File List:\n`;
      const fileList: string[] = [];
      for (const f of files) {
        const rel = f.path.replace(workspacePath, '').replace(/^[\\/]/, '');
        fileList.push(rel);
      }
      manifest += fileList.map(f => `- ${f}`).join('\n');
      manifest += '\n\n#### Critical File Contents:\n';
      for (const f of files) {
        const rel = f.path.replace(workspacePath, '').replace(/^[\\/]/, '');
        // Only include contents of smaller files or important ones (js/ts/css/html/json)
        const ext = rel.split('.').pop()?.toLowerCase();
        const importantExts = ['ts', 'tsx', 'js', 'jsx', 'json', 'css', 'html', 'md'];
        if (importantExts.includes(ext || '') || f.content.length < 5000) {
          manifest += `\n--- FILE: ${rel} ---\n${f.content}\n`;
        }
      }
      conversationHistory.splice(1, 0, {
        role: 'system',
        content: `I have indexed the project. Use this information as your starting context. If you need more details from a file, use 'read_file'.\n${manifest}`
      });
    }
    win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'done', summary: `Indexed ${files.length} files` });
    steps.push({ tool: 'indexing_workspace', status: 'done', summary: `Indexed ${files.length} files` });
  }

  // Add user message
  conversationHistory.push({ role: 'user', content: userMessage });

  for (let iteration = 0; iteration < MAX_AGENT_ITERATIONS; iteration++) {
    console.log(`\n🔄 Agent iteration ${iteration + 1}/${MAX_AGENT_ITERATIONS}`);

    // Call the LLM
    const aiResponse = await callAI(conversationHistory, provider, config);

    // Try to parse as a tool call
    let toolCall = tryParseToolCall(aiResponse);

    // Automation Guard: If the model provided code blocks but no tool call, 
    // it's trying to give "instructions" instead of acting.
    if (!toolCall && (aiResponse.includes('```') || aiResponse.includes('update') || aiResponse.includes('fix'))) {
      // We only do this if it's an assistant message that looks like it's dodging responsibility
      if (aiResponse.length > 50 && (aiResponse.toLowerCase().includes('you should') || aiResponse.toLowerCase().includes('try to'))) {
        console.log("⚠️ Agent is being lazy. Forcing tool usage.");
        const correction = `[System Message] You provided code or instructions but did not use a tool. 
I am an autonomous system; I cannot follow instructions. You must use write_file, edit_file, or replace_lines to apply the changes yourself.
DO NOT tell me what to do. DO IT.`;
        conversationHistory.push({ role: 'assistant', content: aiResponse });
        conversationHistory.push({ role: 'user', content: correction });
        continue;
      }
    }

    if (!toolCall) {
      // Not a tool call — this is the final text response
      conversationHistory.push({ role: 'assistant', content: aiResponse });
      console.log('✅ Agent finished with text response');
      return { finalResponse: aiResponse, steps };
    }

    // It's a tool call — execute it
    const toolName = toolCall.tool;
    const toolSummary = getToolSummary(toolCall);

    // Send step start to frontend
    const stepData = { tool: toolName, status: 'running', summary: toolSummary, iteration: iteration + 1 };
    win?.webContents.send('agent:step', stepData);
    steps.push(stepData);

    // Add assistant's tool request to history
    conversationHistory.push({ role: 'assistant', content: aiResponse });

    // Execute the tool
    const toolResult = await executeToolCall(toolCall, workspacePath);

    // Truncate very long results to avoid context overflow
    const truncatedResult = toolResult.length > 15000
      ? toolResult.substring(0, 15000) + '\n... (truncated, ' + toolResult.length + ' chars total)'
      : toolResult;

    // Update step with result
    steps[steps.length - 1].status = 'done';
    steps[steps.length - 1].result = truncatedResult.substring(0, 500); // short preview for frontend
    win?.webContents.send('agent:step', {
      ...steps[steps.length - 1],
      status: 'done'
    });

    // Add tool result to conversation — use 'user' role since some LLMs don't support 'system' mid-convo well
    conversationHistory.push({
      role: 'user',
      content: `[Tool Result: ${toolName}]\n${truncatedResult}\n\n[Continue your task. Use another tool if needed, or provide your final text response when done.]`
    });
  }

  // Max iterations reached
  const maxMsg = 'I\'ve reached the maximum number of steps (20). Here is a summary of what I\'ve done so far. You may need to continue manually or ask me to proceed.';
  conversationHistory.push({ role: 'assistant', content: maxMsg });
  return { finalResponse: maxMsg, steps };
}

// Human-readable summary for a tool call
function getToolSummary(toolCall: any): string {
  switch (toolCall.tool) {
    case 'read_file': return `Reading ${toolCall.path}`;
    case 'write_file': return `Writing ${toolCall.path}`;
    case 'edit_file': return `Editing ${toolCall.path} (${toolCall.edits?.length || 0} edits)`;
    case 'list_directory': return `Listing ${toolCall.path || 'project root'}`;
    case 'search_files': return `Searching for "${toolCall.pattern}"${toolCall.include ? ` in ${toolCall.include}` : ''}`;
    case 'run_command': return `Running: ${toolCall.command}`;
    case 'create_directory': return `Creating directory ${toolCall.path}`;
    case 'delete_file': return `Deleting ${toolCall.path}`;
    default: return toolCall.tool;
  }
}

// ------ MENU HANDLERS ------
ipcMain.handle('dialog:openFile', async () => {
  if (!win) return { canceled: true };
  return await dialog.showOpenDialog(win, {
    properties: ['openFile']
  });
});

ipcMain.handle('dialog:openFolder', async () => {
  if (!win) return { canceled: true };
  return await dialog.showOpenDialog(win, {
    properties: ['openDirectory']
  });
});

ipcMain.handle('dialog:saveFile', async (_event, content) => {
  if (!win) return { canceled: true };
  const result = await dialog.showSaveDialog(win, {
    defaultPath: 'untitled.txt'
  });
  if (!result.canceled && result.filePath && content) {
    await fs.writeFile(result.filePath, content, 'utf-8');
  }
  return result;
});

ipcMain.handle('fs:readFile', async (_event, filePath) => {
  try {
    return await fs.readFile(filePath, 'utf-8');
  } catch (e: any) {
    console.error(e);
    return null;
  }
});

ipcMain.handle('fs:writeFile', async (_event, filePath, content) => {
  try {
    await fs.writeFile(filePath, content, 'utf-8');
    return true;
  } catch (e: any) {
    console.error(e);
    return false;
  }
});

ipcMain.handle('fs:readDirectory', async (_event, dirPath) => {
  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });
    return entries.map(entry => ({
      name: entry.name,
      isDirectory: entry.isDirectory(),
      path: join(dirPath, entry.name)
    })).sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });
  } catch (e: any) {
    console.error(e);
    return [];
  }
});

ipcMain.on('app:exit', () => app.quit());

// ------ TERMINAL HANDLERS ------
ipcMain.on('terminal:spawn', () => {
  if (ptyProcess) return;
  const shell = os.platform() === 'win32' ? 'powershell.exe' : 'bash';
  ptyProcess = pty.spawn(shell, [], {
    name: 'xterm-color',
    cols: 80,
    rows: 30,
    cwd: process.env.APP_ROOT,
    env: process.env as any
  });

  ptyProcess.onData((data: string) => {
    win?.webContents.send('terminal:incomingData', data);
  });

  ptyProcess.onExit(() => {
    ptyProcess = null;
  });
});

ipcMain.on('terminal:keystroke', (_event, key) => {
  ptyProcess?.write(key);
});

ipcMain.on('terminal:resize', (_event, cols, rows) => {
  if (ptyProcess && cols > 0 && rows > 0) {
    try {
      ptyProcess.resize(cols, rows);
    } catch (e) { }
  }
});

// ------ AI INFRASTRUCTURE ------

ipcMain.handle('ollama:getModels', async () => {
  try {
    const res = await fetch('http://127.0.0.1:11434/api/tags');
    if (!res.ok) throw new Error('Ollama not responding');
    const data: any = await res.json();
    return data.models.map((m: any) => m.name);
  } catch (e: any) {
    return { error: e.message };
  }
});

ipcMain.handle('fs:readDirectoryRecursive', async (_event, dirPath: string) => {
  try {
    return await readDirectoryRecursive(dirPath);
  } catch (e: any) {
    console.error(e);
    return [];
  }
});

ipcMain.handle('execute-agent-task', async (_event, args) => {
  try {
    const userMessage = args.task;
    const provider = args.provider || 'ollama';
    const config = args.config || {};
    const workspacePath = args.workspacePath || null;

    // Run the full agentic loop
    const result = await runAgentLoop(userMessage, provider, config, workspacePath);

    return {
      response: result.finalResponse,
      steps: result.steps
    };

  } catch (err: any) {
    console.error('Agent error:', err);
    return {
      response: `Error: ${err.message}. Check your AI provider settings.`,
      steps: []
    };
  }
});

// Reset conversation when workspace changes  
ipcMain.handle('agent:reset', async () => {
  conversationHistory = [{ role: 'system', content: SYSTEM_PROMPT }];
  workspaceContextLoaded = false;
  console.log('🔄 Agent conversation reset');
  return true;
})
