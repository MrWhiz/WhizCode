import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'
import { exec, spawn } from 'node:child_process'
import { promisify } from 'node:util'
import * as fs from 'node:fs/promises'
// Remove cross-fetch as global fetch is available in modern Electron/Node
import * as os from 'node:os'
import { createRequire } from 'node:module'
import * as chokidar from 'chokidar'
import { IndexingService } from './indexService'
import { CodeGraphService } from './graphService'
import { DiffService, type FileChange } from './diffService'

const _require = createRequire(import.meta.url)
const pty = _require('node' + '-pty')

const execAsync = promisify(exec)

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

process.env.APP_ROOT = join(__dirname, '..')
// Suppress noisy deprecation warnings that distract from dev logs
process.env.NODE_NO_WARNINGS = '1';
app.commandLine.appendSwitch('no-warnings');

export const VITE_DEV_SERVER_URL = process.env['VITE_DEV_SERVER_URL']
export const MAIN_DIST = join(process.env.APP_ROOT, 'dist-electron')
export const RENDERER_DIST = join(process.env.APP_ROOT, 'dist')

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL ? join(process.env.APP_ROOT, 'public') : RENDERER_DIST

let win: BrowserWindow | null
let ptyProcess: any = null
let indexingService: IndexingService | null = null
let graphService: CodeGraphService | null = null
const diffService = new DiffService();
let workspaceWatcher: chokidar.FSWatcher | null = null;
let pendingPermissionResolver: ((decision: { approved: boolean }) => void) | null = null
let abortRequested = false;
// voyageKey removed as it was unused and causing lint error

// Ollama Configuration
const OLLAMA_URL = 'http://127.0.0.1:11434/api/chat';
const MODEL_NAME = 'deepseek-coder-v2:latest'; // Optimized for local coding

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
const SKIP_DIRS = new Set(['node_modules', '.git', 'dist', 'dist-electron', '.next', '__pycache__', '.venv', 'venv', '.cache', 'coverage', '.idea', '.vscode', 'build', 'out', 'bin', 'obj']);
const BINARY_EXTS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.ico', '.webp', '.svg', '.woff', '.woff2', '.ttf', '.eot', '.mp3', '.mp4', '.zip', '.tar', '.gz', '.exe', '.dll', '.so', '.dylib', '.lock', '.pdf', '.bin', '.pyc', '.node']);

/**
 * Checks if a file is likely binary by reading a small chunk and looking for NULL bytes
 */
async function isBinaryFile(filePath: string): Promise<boolean> {
  const fd = await fs.open(filePath, 'r');
  try {
    const buffer = Buffer.alloc(1024);
    const { bytesRead } = await fd.read(buffer, 0, 1024, 0);
    for (let i = 0; i < bytesRead; i++) {
      if (buffer[i] === 0) return true; // NULL byte found
    }
    return false;
  } finally {
    await fd.close();
  }
}

async function readDirectoryRecursive(dirPath: string, maxFiles = 2000): Promise<{ path: string }[]> {
  const results: { path: string }[] = [];
  async function walk(currentPath: string) {
    if (results.length >= maxFiles) return;
    try {
      const entries = await fs.readdir(currentPath, { withFileTypes: true });
      for (const entry of entries) {
        if (results.length >= maxFiles) break;
        const fullPath = join(currentPath, entry.name);
        if (entry.isDirectory()) {
          // Skip hidden and noise directories
          if (!SKIP_DIRS.has(entry.name) && !entry.name.startsWith('.')) {
            await walk(fullPath);
          }
        } else {
          const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
          if (!BINARY_EXTS.has(ext)) {
            results.push({ path: fullPath });
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

// ====== SYSTEM PROMPTS ======

const PLANNER_SYSTEM_PROMPT = `
<IDENTITY>
You are the "WHIZCODE ARCHITECT". Your role is to analyze user requests and project context to create a foolproof implementation plan.
</IDENTITY>

<PRIME_DIRECTIVE>
1. **STRATEGIZE**: Break down the task into logical, sequential steps.
2. **ENVIRONMENT**: Always start by checking the environment and prerequisites. Identify if dependencies (node_modules, venv, go.mod, etc.) are present and properly configured. Include setup steps if they are missing.
3. **CONTEXTUALIZE**: Identify which files need to be read, modified, or created.
4. **VALIDATE**: Ensure the plan includes testing and verification steps.
5. **NO TOOLS**: Do not use tools yourself. Output a clean, detailed Markdown plan.
</PRIME_DIRECTIVE>

<OUTPUT_FORMAT>
# IMPLEMENTATION PLAN
1. [Phase 1: Environment & Discovery] - Check for required runtimes/dependencies and list relevant files.
2. [Phase 2: Setup] - Install dependencies or create virtual environments if necessary.
3. [Phase 3: Implementation] - Perform the core task.
4. [Phase 4: Verification] - Run tests or validate project.
</OUTPUT_FORMAT>
`;

const EXECUTOR_SYSTEM_PROMPT = `
<IDENTITY>
You are the "WHIZCODE EXECUTOR", a high-speed autonomous coding engine. 
You possess full system access. You are a doer, a builder, and an execution machine.
</IDENTITY>

<PRIME_DIRECTIVE>
1. **ACT FIRST**: While the task is in progress, your response should be a tool call.
2. **FOLLOW THE PLAN**: Execute the plan perfectly. **NEVER skip environment checks** specified in the plan.
3. **COMPLETION**: Once ALL steps in the plan are finished, DO NOT call any more tools. Instead, provide a final technical summary of your work and state that the task is complete.
4. **ZERO QUESTIONS**: NEVER ask the user for permission or code. Use your tools.
</PRIME_DIRECTIVE>

<TOOL_HIERARCHY>
- **Discovery**: 'list_directory' (path), 'search_files' (pattern, include), 'semantic_search' (query)
- **Context**: 'read_file' (path) (MANDATORY before every edit). Use 'run_command' (command) to check versions or environment status.
- **Execution**: 
  - 'apply_diffs' (changes: [{path, diff}]): Preferred for multi-line edits.
  - 'write_file' (path, content): Best for new files.
  - 'run_command' (command): For terminal operations.
- **Validation**: 'validate_project' (MANDATORY after every change)
</TOOL_HIERARCHY>

<TOOL_SCHEMAS>
- read_file: {"tool": "read_file", "path": "string"}
- write_file: {"tool": "write_file", "path": "string", "content": "string"}
- edit_file: {"tool": "edit_file", "path": "string", "edits": [{"search": "string", "replace": "string"}]}
- list_directory: {"tool": "list_directory", "path": "string"}
- run_command: {"tool": "run_command", "command": "string"}
- apply_diffs: {"tool": "apply_diffs", "changes": [{"path": "string", "diff": "string"}]}
</TOOL_SCHEMAS>

<OUTPUT_FORMAT>
- **Thinking**: <THOUGHT> [What phase are you on? Are dependencies met? Current action?] </THOUGHT>
- **Action**: [Valid JSON Tool Call]
- **NO CHATTER**: No "I'd be happy to", "Here is", or "Could you".
</OUTPUT_FORMAT>
`;

let conversationHistory: any[] = [];
let workspaceManifest: string = '';

let workspaceContextLoaded = false;

// ====== LLM PROVIDER CALLS ======

async function callAI(messages: any[], modelConfig: { provider: string, model: string }, config: any) {
  try {
    let response: any;
    let data: any;
    const { provider, model } = modelConfig;

    if (provider === 'openai') {
      response = await fetch('https://api.openai.com/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${config.openaiKey}`
        },
        body: JSON.stringify({
          model: model || 'gpt-4o',
          messages: messages,
          temperature: 0.1
        })
      });
      if (!response.ok) throw new Error(`OpenAI HTTP Error: ${response.status} ${await response.text()}`);
      data = await response.json();
      return data.choices[0].message.content;
    } else if (provider === 'gemini') {
      const geminiMessages = messages.filter(m => m.role !== 'system').map(m => {
        return { role: m.role === 'assistant' ? 'model' : m.role, parts: [{ text: m.content }] };
      });
      const systemMsg = messages.find(m => m.role === 'system');

      const body: any = { contents: geminiMessages };
      if (systemMsg) {
        body.system_instruction = { parts: [{ text: systemMsg.content }] };
      }

      response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/${model || 'gemini-1.5-flash'}:generateContent?key=${config.geminiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      });
      if (!response.ok) throw new Error(`Gemini HTTP Error: ${response.status} ${await response.text()}`);
      data = await response.json();
      return data.candidates[0].content.parts[0].text;
    } else {
      response = await fetch(OLLAMA_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: model || MODEL_NAME,
          messages: messages,
          stream: false,
          options: {
            temperature: 0
          }
        })
      });
      if (!response.ok) throw new Error(`Ollama HTTP Error: ${response.status}`);
      data = await response.json();
      return data.message.content;
    }
  } catch (error: any) {
    console.error("AI Provider Error:", error);
    throw error;
  }
}

// ====== TOOL PARSER ======

function tryParseToolCall(response: string): any | null {
  if (!response) return null;
  const trimmed = response.trim();

  // 1. Find the largest JSON object in the string
  const firstBrace = trimmed.indexOf('{');
  const lastBrace = trimmed.lastIndexOf('}');

  if (firstBrace !== -1 && lastBrace !== -1 && lastBrace > firstBrace) {
    const jsonCandidate = trimmed.substring(firstBrace, lastBrace + 1);
    try {
      const parsed = JSON.parse(jsonCandidate);
      if (parsed.tool) return parsed;
    } catch {
      // If full wrap fails, try regex for the tool object specifically
      const toolRegex = /({\s*"tool"\s*:\s*"[^"]+"[\s\S]*?})/g; // Added 'g' flag for multiple matches
      let match;
      let bestMatch = null;
      while ((match = toolRegex.exec(trimmed)) !== null) {
        try {
          const innerParsed = JSON.parse(match[1]);
          if (innerParsed.tool) {
            // Prioritize the longest valid tool JSON found
            if (!bestMatch || match[1].length > bestMatch[1].length) {
              bestMatch = match;
            }
          }
        } catch { }
      }
      if (bestMatch) {
        try {
          const parsed = JSON.parse(bestMatch[1]);
          if (parsed.tool) return parsed;
        } catch { }
      }
    }
  }

  return null;
}

// ====== TOOL EXECUTOR ======

async function executeToolCall(toolData: any, workspacePath: string | null): Promise<{ result: string; logs?: string[]; abort?: boolean }> {
  const resolvedPath = toolData.path ? resolvePath(toolData.path, workspacePath) : '';
  console.log(`\n[TOOL] [${toolData.tool}] ${resolvedPath || toolData.command || toolData.pattern || ''}`);

  try {
    switch (toolData.tool) {
      case 'read_file': {
        if (!toolData.path) return { result: '❌ Error: Tool "read_file" requires a "path" parameter.' };
        const isBinary = await isBinaryFile(resolvedPath);
        if (isBinary) {
          return { result: `❌ Cannot read ${toolData.path}: This appears to be a binary file.` };
        }
        const content = await fs.readFile(resolvedPath, 'utf-8');
        const lines = content.split('\n');
        // Add line numbers for context
        return { result: lines.map((line, i) => `${i + 1}: ${line}`).join('\n') };
      }

      case 'write_file': {
        if (!toolData.path) {
          return { result: '❌ Error: Tool "write_file" requires a "path" parameter.' };
        }
        // Ensure parent directory exists
        const dir = dirname(resolvedPath);
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(resolvedPath, toolData.content, 'utf-8');
        const lineCount = toolData.content.split('\n').length;

        // Refresh manifest so agent sees new file immediately
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Successfully wrote ${lineCount} lines to ${toolData.path}` };
      }

      case 'edit_file': {
        if (!toolData.path) return { result: '❌ Error: Tool "edit_file" requires a "path" parameter.' };
        if (!toolData.edits) return { result: '❌ Error: Tool "edit_file" requires an "edits" parameter.' };
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
              return {
                result: `❌ edit_file failed for ${toolData.path}: The search string exists but whitespace/indentation did not match exactly. 
Please 'read_file' again to get the EXACT indentation or use the 'write_file' tool to overwrite the file if the edit is complex.
Searched for: "${edit.search.substring(0, 50)}..."`
              };
            } else {
              return { result: `❌ edit_file failed: could not find the following code block in ${toolData.path}:\n\n${edit.search}\n\nMake sure you have the latest content via 'read_file'.` };
            }
          }
        }
        await fs.writeFile(resolvedPath, content, 'utf-8');

        // Refresh manifest
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Applied ${editCount} edit(s) to ${toolData.path}` };
      }

      case 'list_directory': {
        return { result: await listDirectory(resolvedPath || workspacePath || '.') };
      }

      case 'search_files': {
        const searchRoot = workspacePath || '.';
        return { result: await searchFiles(searchRoot, toolData.pattern, toolData.include) };
      }

      case 'run_command': {
        if (!toolData.command) return { result: '❌ Error: Tool "run_command" requires a "command" parameter.' };
        const command = toolData.command;
        // Notify UI that we are waiting for permission
        win?.webContents.send('agent:step', {
          tool: 'run_command',
          status: 'awaiting_permission',
          summary: `Execute: ${command}`,
          command: command
        });

        const decision = await new Promise<{ approved: boolean }>(resolve => {
          pendingPermissionResolver = resolve;
        });
        pendingPermissionResolver = null;

        if (!decision.approved) {
          return { result: '❌ Command denied by user.', abort: true };
        }

        const cwd = workspacePath || process.cwd();
        const logs: string[] = [];

        // Transition to running if approved
        win?.webContents.send('agent:step', {
          tool: 'run_command',
          status: 'running',
          summary: `Executing: ${command}`,
          logs: logs
        });

        try {
          if (ptyProcess) {
            ptyProcess.write(`\r\n# Executing agent command: ${command}\r\n`);
          }

          const fullOutput = await new Promise<string>((resolve, reject) => {
            const shell = process.platform === 'win32' ? 'powershell.exe' : 'bash';
            const shellArgs = process.platform === 'win32' ? ['-Command', command] : ['-c', command];

            const child = spawn(shell, shellArgs, { cwd, shell: true });
            let output = '';

            const handleData = (data: any) => {
              const str = data.toString();
              output += str;
              if (ptyProcess) ptyProcess.write(str);

              const lines = str.split(/\r?\n/).filter((l: string) => l.trim().length > 0);
              if (lines.length > 0) {
                logs.push(...lines);
                // Only keep last 100 lines for the UI to avoid lag
                if (logs.length > 100) logs.splice(0, logs.length - 100);

                win?.webContents.send('agent:step', {
                  tool: 'run_command',
                  status: 'running',
                  summary: `Executing: ${command}`,
                  logs: [...logs]
                });
              }
            };

            child.stdout?.on('data', handleData);
            child.stderr?.on('data', handleData);

            child.on('close', (code) => {
              if (code === 0) {
                resolve(output.trim() || '(command completed with no output)');
              } else {
                resolve(`Command exited with code ${code}:\n${output}`.trim());
              }
            });

            child.on('error', (err) => {
              reject(err);
            });
          });

          // Refresh manifest after command execution (might have created files/dirs like venv)
          if (workspacePath) {
            const files = await readDirectoryRecursive(workspacePath, 3000);
            workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
              files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
          }

          return { result: fullOutput, logs };
        } catch (e: any) {
          const errOutput = `Command failed: ${e.message}`.trim();
          if (ptyProcess) ptyProcess.write(errOutput + '\r\n');
          return { result: errOutput, logs };
        }
      }

      case 'create_directory': {
        await fs.mkdir(resolvedPath, { recursive: true });

        // Refresh manifest
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Created directory: ${toolData.path}` };
      }

      case 'delete_file': {
        await fs.unlink(resolvedPath);

        // Refresh manifest
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Deleted: ${toolData.path}` };
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

        // Refresh manifest
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Replaced lines ${toolData.startLine}-${toolData.endLine} in ${toolData.path}` };
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

        // Refresh manifest
        if (workspacePath) {
          const files = await readDirectoryRecursive(workspacePath, 3000);
          workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
            files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        }

        return { result: `✅ Inserted code after line ${toolData.line} in ${toolData.path}` };
      }

      case 'apply_diffs': {
        const changes: FileChange[] = toolData.changes.map((c: any) => ({
          path: resolvePath(c.path, workspacePath),
          blocks: DiffService.parseDiffBlocks(c.diff)
        }));

        if (changes.some(c => c.blocks.length === 0)) {
          return { result: '❌ Failed to parse one or more diff blocks. Ensure you use the exact format:\n<<<< SEARCH\n...\n====\n...\n>>>> REPLACE' };
        }

        const result = await diffService.applyTransaction(changes);
        if (result.success) {
          // Refresh manifest after applying diffs
          if (workspacePath) {
            const files = await readDirectoryRecursive(workspacePath, 3000);
            workspaceManifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n` +
              files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
          }
          return { result: `✅ Successfully applied diffs to ${result.appliedCount} files.` };
        } else {
          return { result: `❌ Diff transaction failed: ${result.error}. No changes were saved (auto-rollback successful).` };
        }
      }

      case 'validate_project': {
        const cwd = workspacePath || process.cwd();
        try {
          // Check for tsconfig.json to see if we should run tsc
          await fs.access(join(cwd, 'tsconfig.json'));
          const { stdout } = await execAsync('npx tsc --noEmit', { cwd });
          return { result: `Validation (tsc) passed! No type errors found.\n${stdout}` };
        } catch (e: any) {
          if (e.code === 'ENOENT') return { result: 'No tsconfig.json found. Skipping tsc validation.' };
          return { result: `Validation failed with errors:\n${e.stdout || ''}\n${e.stderr || ''}` };
        }
      }

      case 'run_tests': {
        const cwd = workspacePath || process.cwd();
        try {
          const { stdout } = await execAsync('npm test', { cwd });
          return { result: `Tests passed!\n${stdout}` };
        } catch (e: any) {
          return { result: `Tests failed:\n${e.stdout || ''}\n${e.stderr || ''}` };
        }
      }

      case 'get_blast_radius': {
        if (!graphService) return { result: '❌ Code graph not initialized.' };
        const resolved = resolvePath(toolData.path, workspacePath);
        const affected = graphService.getBlastRadius(resolved);
        if (affected.length === 0) return { result: `No external files depend on ${toolData.path}.` };
        return { result: `Files affected by changing ${toolData.path}:\n` + affected.map(f => `- ${f.replace(workspacePath || '', '').replace(/^[\\/]/, '')}`).join('\n') };
      }

      case 'semantic_search': {
        if (!indexingService) return { result: '❌ Indexing service not initialized.' };
        const results = await indexingService.search(toolData.query);
        if (results.length === 0) return { result: 'No relevant code found.' };
        return { result: results.map((r: any) => `--- ${r.filePath}:${r.startLine}-${r.endLine} (Score: ${r._distance}) ---\n${r.content}`).join('\n\n') };
      }

      default:
        return { result: `❌ Unknown tool: "${toolData.tool}". Available tools: semantic_search, apply_diffs, validate_project, run_tests, get_blast_radius, read_file, replace_lines, insert_code, write_file, edit_file, list_directory, search_files, run_command` };
    }
  } catch (e: any) {
    return { result: `❌ Tool error (${toolData.tool}): ${e.message}` };
  }
}

// ====== AGENT LOOP ======

const MAX_AGENT_ITERATIONS = 20;

async function runAgentLoop(
  userMessage: string,
  planner: { provider: string, model: string },
  executor: { provider: string, model: string },
  config: any,
  workspacePath: string | null,
  activeContext: { path: string, content: string } | null = null
): Promise<{ finalResponse: string; steps: any[] }> {
  const steps: any[] = [];
  abortRequested = false;

  // 1. Build project context
  let projectStatus = `<PROJECT_STATUS>\n`;
  if (workspaceManifest) {
    projectStatus += `Project Indexed. Files found: ${workspaceManifest.split('\n').filter(l => l.startsWith('-')).length}\n${workspaceManifest}`;
  } else {
    projectStatus += `Project not indexed yet. Root: ${workspacePath}\n`;
  }
  if (activeContext) {
    projectStatus += `\n\n### ACTIVE FILE (CURRENTLY OPEN IN EDITOR):\nPath: ${activeContext.path}\nContent:\n${activeContext.content}\n`;
  }
  projectStatus += `\n</PROJECT_STATUS>`;

  // 2. PHASE 1: PLANNING
  win?.webContents.send('agent:step', { tool: 'planning', status: 'running', summary: 'Generating implementation plan...' });

  const plannerMessages = [
    { role: 'system', content: `${PLANNER_SYSTEM_PROMPT}\n\n${projectStatus}` },
    ...conversationHistory,
    { role: 'user', content: userMessage }
  ];

  const plan = await callAI(plannerMessages, planner, config);
  console.log('[PLAN]\n', plan);

  const planStep: any = { tool: 'planning', status: 'awaiting_permission', summary: 'Architectural Plan Ready', result: plan };
  win?.webContents.send('agent:step', planStep);

  const decision = await new Promise<{ approved: boolean }>(resolve => {
    pendingPermissionResolver = resolve;
  });
  pendingPermissionResolver = null;

  if (!decision.approved) {
    planStep.status = 'done';
    planStep.summary = 'Plan Rejected';
    win?.webContents.send('agent:step', planStep);
    steps.push(planStep);
    const cancelMsg = "⚠️ Plan rejected by user. Stopping task.";
    conversationHistory.push({ role: 'assistant', content: cancelMsg });
    return { finalResponse: cancelMsg, steps };
  }

  planStep.status = 'done';
  planStep.summary = 'Plan Approved';
  win?.webContents.send('agent:step', planStep);
  steps.push(planStep);

  // 3. PHASE 2: EXECUTION
  const executorInstructions = `${EXECUTOR_SYSTEM_PROMPT}\n\n${projectStatus}\n\n<PLAN>\n${plan}\n</PLAN>\n\n[ACT NOW: Start from the first step of the plan. Output a tool call.]`;

  const currentMessages = [
    { role: 'system', content: executorInstructions },
    ...conversationHistory,
    { role: 'user', content: `Task: ${userMessage}` }
  ];

  for (let iteration = 0; iteration < MAX_AGENT_ITERATIONS; iteration++) {
    if (abortRequested) {
      console.log("[ABORT] Agent loop stopped by user.");
      const abortMsg = "⚠️ Task cancelled by user.";
      conversationHistory.push({ role: 'assistant', content: abortMsg });
      return { finalResponse: abortMsg, steps };
    }
    console.log(`[ITERATION ${iteration + 1}/${MAX_AGENT_ITERATIONS}]`);

    // Call Executor
    const aiResponse = await callAI(currentMessages, executor, config);
    let toolCall = tryParseToolCall(aiResponse);

    if (!toolCall) {
      // Stalling check
      if (aiResponse.length > 5 && (aiResponse.includes('```') || aiResponse.includes('?'))) {
        console.log("[WAIT] Stalling detected. Forcing action...");
        currentMessages.push({ role: 'assistant', content: aiResponse });
        currentMessages.push({ role: 'user', content: 'STRICT RULE: YOUR RESPONSE MUST BE A JSON TOOL CALL. DO NOT TALK. ACTION ONLY.' });
        continue;
      }

      // Final response
      conversationHistory.push({ role: 'user', content: userMessage });
      conversationHistory.push({ role: 'assistant', content: aiResponse });
      return { finalResponse: aiResponse, steps };
    }

    // It's a tool call — execute it
    const toolName = toolCall.tool;
    const toolSummary = getToolSummary(toolCall);

    const stepData = { tool: toolName, status: 'running', summary: toolSummary, iteration: iteration + 1 };
    win?.webContents.send('agent:step', stepData);
    steps.push(stepData);

    currentMessages.push({ role: 'assistant', content: aiResponse });
    const { result: toolResult, logs, abort } = await executeToolCall(toolCall, workspacePath);

    const truncatedResult = toolResult.length > 15000
      ? toolResult.substring(0, 15000) + '\n... (truncated)'
      : toolResult;

    steps[steps.length - 1].status = 'done';
    steps[steps.length - 1].result = truncatedResult.substring(0, 500);
    if (logs) steps[steps.length - 1].logs = logs;

    win?.webContents.send('agent:step', { ...steps[steps.length - 1], status: 'done' });

    if (abort) {
      console.log(`[ABORT] Stopping loop due to tool abort signal (e.g. denial)`);
      const finalMsg = `The user denied the command: ${toolCall.command || toolName}. Stopping task.`;
      conversationHistory.push({ role: 'assistant', content: finalMsg });
      return { finalResponse: finalMsg, steps };
    }

    currentMessages.push({
      role: 'user',
      content: `[Result: ${toolName}]\n${truncatedResult}\n\n[NEXT STEP: If the implementation plan is fully executed and verified, provide your FINAL RESPONSE now. Otherwise, continue with the next tool call.]`
    });
  }

  const finalAI = currentMessages[currentMessages.length - 1].content;
  conversationHistory.push({ role: 'user', content: userMessage });
  conversationHistory.push({ role: 'assistant', content: finalAI });

  return { finalResponse: finalAI, steps };
}

// Human-readable summary for a tool call
function getToolSummary(toolCall: any): string {
  const path = toolCall.path || '(missing path)';
  switch (toolCall.tool) {
    case 'read_file': return `Reading ${path}`;
    case 'write_file': return `Writing ${path}`;
    case 'edit_file': return `Editing ${path} (${toolCall.edits?.length || 0} edits)`;
    case 'list_directory': return `Listing ${path}`;
    case 'search_files': return `Searching for "${toolCall.pattern || '?'}"${toolCall.include ? ` in ${toolCall.include}` : ''}`;
    case 'run_command': return `Running: ${toolCall.command || '(missing command)'}`;
    case 'create_directory': return `Creating directory ${path}`;
    case 'delete_file': return `Deleting ${path}`;
    case 'semantic_search': return `Searching semantically for "${toolCall.query || '?'}"`;
    case 'get_blast_radius': return `Calculating blast radius for ${path}`;
    case 'apply_diffs': return `Applying diffs to ${toolCall.changes?.length || 0} files`;
    case 'validate_project': return 'Performing project-wide validation';
    case 'run_tests': return 'Running test suite';
    default: return toolCall.tool;
  }
}

// ------ WORKSPACE FILE WATCHER ------
function setupWorkspaceWatcher(watchPath: string) {
  if (workspaceWatcher) {
    workspaceWatcher.close();
    workspaceWatcher = null;
  }

  workspaceWatcher = chokidar.watch(watchPath, {
    ignored: /(^|[\\/\\])(node_modules|\.git|dist|dist-electron|build|\.next|__pycache__|\.venv|venv|\.cache|coverage)([\\/\\]|$)/,
    persistent: true,
    ignoreInitial: true,
    depth: 20,
    awaitWriteFinish: {
      stabilityThreshold: 300,
      pollInterval: 100
    }
  });

  const notifyRenderer = (type: string, filePath: string) => {
    win?.webContents.send('fs:directoryChanged', { type, filePath });
  };

  workspaceWatcher.on('add', (path: string) => notifyRenderer('add', path));
  workspaceWatcher.on('addDir', (path: string) => notifyRenderer('addDir', path));
  workspaceWatcher.on('unlink', (path: string) => notifyRenderer('unlink', path));
  workspaceWatcher.on('unlinkDir', (path: string) => notifyRenderer('unlinkDir', path));

  console.log(`[WATCHER] Watching workspace: ${watchPath}`);
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
  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory']
  });
  // Start watching the selected folder
  if (!result.canceled && result.filePaths?.length > 0) {
    setupWorkspaceWatcher(result.filePaths[0]);
  }
  return result;
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

ipcMain.handle('execute-agent-task', async (_event, { task, planner, executor, workspacePath, activeFile, config }) => {
  try {
    // 1. Initial Workspace Scan (only if not loaded)
    if (workspacePath && !workspaceContextLoaded) {
      workspaceContextLoaded = true;
      console.log('[INDEXING] Building project manifest:', workspacePath);
      win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'running', summary: 'Building project context...' });

      // Start watching workspace for file changes
      setupWorkspaceWatcher(workspacePath);

      const files = await readDirectoryRecursive(workspacePath, 3000); // Increased limit but lightweight
      if (files.length > 0) {
        let manifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n`;
        manifest += files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
        manifest += '\n\n#### Critical Metadata:\n(Use read_file to access full contents)\n';
        workspaceManifest = manifest;
      }

      // Initialize services
      if (!graphService) {
        graphService = new CodeGraphService();
        await graphService.initialize(workspacePath);
      }
      if (config.voyageKey && !indexingService) {
        indexingService = new IndexingService(config.voyageKey, (p) => graphService?.updateFile(p));
        await indexingService.initialize(workspacePath);
        await indexingService.indexWorkspace();
      }

      win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'done', summary: `Indexed ${files.length} files` });
    }

    // 2. Run the agent loop
    const result = await runAgentLoop(task, planner, executor, config, workspacePath, activeFile);
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

ipcMain.handle('agent:stop', () => {
  abortRequested = true;
  if (pendingPermissionResolver) {
    pendingPermissionResolver({ approved: false });
  }
  return true;
});

ipcMain.handle('agent:permission-response', (_event, decision) => {
  if (pendingPermissionResolver) {
    pendingPermissionResolver(decision);
  }
  return true;
});

// Reset conversation when workspace changes
ipcMain.handle('agent:reset', async () => {
  conversationHistory = [];
  workspaceContextLoaded = false;
  workspaceManifest = '';
  console.log('🔄 Agent conversation reset');
  return true;
});
