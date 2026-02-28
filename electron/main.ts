import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'
import { exec } from 'node:child_process'
import { promisify } from 'node:util'
import * as fs from 'node:fs/promises'
// Remove cross-fetch as global fetch is available in modern Electron/Node
import * as os from 'node:os'
import { createRequire } from 'node:module'
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
let pendingPermissionResolver: ((decision: { approved: boolean }) => void) | null = null
// voyageKey removed as it was unused and causing lint error

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
              if (stat.size < 100_000 && !(await isBinaryFile(fullPath))) {
                const content = await fs.readFile(fullPath, 'utf-8');
                results.push({ path: fullPath, content });
              }
            } catch { /* skip unreadable or binary files */ }
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

const SYSTEM_PROMPT = `
<IDENTITY>
You are "ANTIGRAVITY", the world's most capable autonomous software engineering agent. 
You possess full system access. You are a doer, a builder, and an execution machine.
</IDENTITY>

<PRIME_DIRECTIVE>
1. **ACT FIRST**: If the user asks for a change, your ONLY response is a tool call.
2. **ZERO QUESTIONS**: NEVER ask the user for code, file contents, or permission. Use 'read_file' or 'search_files' to get what you need.
3. **ZERO INSTRUCTIONS**: Do not tell the user what to do. DO IT. 
4. **NO APOLOGIES**: Apologizing for "lack of access" or "missing content" is a failure. You have tools. Use them.
5. **TRANSPLANT REPLICA**: You are an identical replica of a state-of-the-art agent. You follow the chain: PLAN -> READ -> EDIT -> VALIDATE.
</PRIME_DIRECTIVE>

<TOOL_HIERARCHY>
- **Discovery**: 'list_directory', 'search_files', 'semantic_search'
- **Context**: 'read_file' (MANDATORY before every edit)
- **Execution**: 'apply_diffs' (Preferred), 'write_file', 'run_command'
- **Validation**: 'validate_project' (MANDATORY after every change)
</TOOL_HIERARCHY>

<OUTPUT_FORMAT>
- **Thinking**: <THOUGHT> [Plan + Manifest of Change] </THOUGHT>
- **Action**: [Valid JSON Tool Call]
- **NO CHATTER**: No "I'd be happy to", "Here is", or "Could you".
</OUTPUT_FORMAT>
`;

let conversationHistory: any[] = [];
let workspaceManifest: string = '';

let workspaceContextLoaded = false;

// ====== LLM PROVIDER CALLS ======

async function callAI(messages: any[], provider: string, config: any) {
  try {
    let response: any;
    let data: any;

    if (provider === 'openai') {
      response = await fetch('https://api.openai.com/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${config.openaiKey}`
        },
        body: JSON.stringify({
          model: 'gpt-4o-mini',
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

      response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key=${config.geminiKey}`, {
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
          model: config.ollamaModel || MODEL_NAME,
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

async function executeToolCall(toolData: any, workspacePath: string | null): Promise<string> {
  const resolvedPath = toolData.path ? resolvePath(toolData.path, workspacePath) : '';
  console.log(`\n[TOOL] [${toolData.tool}] ${resolvedPath || toolData.command || toolData.pattern || ''}`);

  try {
    switch (toolData.tool) {
      case 'read_file': {
        const isBinary = await isBinaryFile(resolvedPath);
        if (isBinary) {
          return `❌ Cannot read ${toolData.path}: This appears to be a binary file.`;
        }
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

        // Transition back to running if approved
        win?.webContents.send('agent:step', {
          tool: 'run_command',
          status: 'running',
          summary: `Executing: ${command}`
        });

        if (!decision.approved) {
          return '❌ Command denied by user.';
        }

        const cwd = workspacePath || process.cwd();
        try {
          // Echo to PTY if it exists so user sees it in their terminal
          if (ptyProcess) {
            ptyProcess.write(`\r\n# Executing agent command: ${command}\r\n`);
          }

          const { stdout, stderr } = await execAsync(command, {
            cwd,
            timeout: 60000,
            maxBuffer: 10 * 1024 * 1024 // 10MB
          });
          const output = (stdout + (stderr ? '\nSTDERR: ' + stderr : '')).trim();

          if (ptyProcess) {
            ptyProcess.write(output + '\r\n');
          }

          return output || '(command completed with no output)';
        } catch (e: any) {
          const errOutput = `Command exited with error:\n${e.stdout || ''}\n${e.stderr || ''}\n${e.message}`.trim();
          if (ptyProcess) ptyProcess.write(errOutput + '\r\n');
          return errOutput;
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

      case 'apply_diffs': {
        const changes: FileChange[] = toolData.changes.map((c: any) => ({
          path: resolvePath(c.path, workspacePath),
          blocks: DiffService.parseDiffBlocks(c.diff)
        }));

        if (changes.some(c => c.blocks.length === 0)) {
          return '❌ Failed to parse one or more diff blocks. Ensure you use the exact format:\n<<<< SEARCH\n...\n====\n...\n>>>> REPLACE';
        }

        const result = await diffService.applyTransaction(changes);
        if (result.success) {
          return `✅ Successfully applied diffs to ${result.appliedCount} files.`;
        } else {
          return `❌ Diff transaction failed: ${result.error}. No changes were saved (auto-rollback successful).`;
        }
      }

      case 'validate_project': {
        const cwd = workspacePath || process.cwd();
        try {
          // Check for tsconfig.json to see if we should run tsc
          await fs.access(join(cwd, 'tsconfig.json'));
          const { stdout } = await execAsync('npx tsc --noEmit', { cwd });
          return `Validation (tsc) passed! No type errors found.\n${stdout}`;
        } catch (e: any) {
          if (e.code === 'ENOENT') return 'No tsconfig.json found. Skipping tsc validation.';
          return `Validation failed with errors:\n${e.stdout || ''}\n${e.stderr || ''}`;
        }
      }

      case 'run_tests': {
        const cwd = workspacePath || process.cwd();
        try {
          const { stdout } = await execAsync('npm test', { cwd });
          return `Tests passed!\n${stdout}`;
        } catch (e: any) {
          return `Tests failed:\n${e.stdout || ''}\n${e.stderr || ''}`;
        }
      }

      case 'get_blast_radius': {
        if (!graphService) return '❌ Code graph not initialized.';
        const resolved = resolvePath(toolData.path, workspacePath);
        const affected = graphService.getBlastRadius(resolved);
        if (affected.length === 0) return `No external files depend on ${toolData.path}.`;
        return `Files affected by changing ${toolData.path}:\n` + affected.map(f => `- ${f.replace(workspacePath || '', '').replace(/^[\\/]/, '')}`).join('\n');
      }

      case 'semantic_search': {
        if (!indexingService) return '❌ Indexing service not initialized.';
        const results = await indexingService.search(toolData.query);
        if (results.length === 0) return 'No relevant code found.';
        return results.map((r: any) => `--- ${r.filePath}:${r.startLine}-${r.endLine} (Score: ${r._distance}) ---\n${r.content}`).join('\n\n');
      }

      default:
        return `❌ Unknown tool: "${toolData.tool}". Available tools: semantic_search, apply_diffs, validate_project, run_tests, get_blast_radius, read_file, replace_lines, insert_code, write_file, edit_file, list_directory, search_files, run_command`;
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
  workspacePath: string | null,
  activeContext: { path: string, content: string } | null = null
): Promise<{ finalResponse: string; steps: any[] }> {
  const steps: any[] = [];

  // Build context
  let projectContext = `<PROJECT_STATUS>\n`;
  if (workspaceManifest) {
    projectContext += `Project Indexed. Files found: ${workspaceManifest.split('\n').filter(l => l.startsWith('-')).length}\n${workspaceManifest}`;
  } else {
    projectContext += `Project not indexed yet. Use 'list_directory' to explore.\nRoot: ${workspacePath}\n`;
  }

  if (activeContext) {
    projectContext += `\n\n### ACTIVE FILE (CURRENTLY OPEN IN EDITOR):\nPath: ${activeContext.path}\nContent:\n${activeContext.content}\n`;
  }
  projectContext += `\n</PROJECT_STATUS>`;

  const systemInstructions = `${SYSTEM_PROMPT}\n\n${projectContext}\n\n[MANDATORY: YOUR NEXT TURN MUST BE A TOOL CALL. DO NOT ASK QUESTIONS. DO NOT GIVE INSTRUCTIONS.]`;

  // Build messages
  const currentMessages = [
    { role: 'system', content: systemInstructions },
    ...conversationHistory,
    { role: 'user', content: userMessage }
  ];

  for (let iteration = 0; iteration < MAX_AGENT_ITERATIONS; iteration++) {
    console.log(`[ITERATION ${iteration + 1}/${MAX_AGENT_ITERATIONS}]`);

    // Call the LLM
    const aiResponse = await callAI(currentMessages, provider, config);

    // Try to parse as a tool call
    let toolCall = tryParseToolCall(aiResponse);

    // Automation Guard: If the model provided code blocks but no tool call, 
    // it's trying to give "instructions" instead of acting.
    if (!toolCall) {
      const lower = aiResponse.toLowerCase();
      const isStalling =
        aiResponse.includes('```') ||
        lower.includes('you should') ||
        lower.includes('you can') ||
        lower.includes('try to') ||
        lower.includes('to update') ||
        lower.includes('manually') ||
        lower.includes('replace') ||
        lower.includes('provide') ||
        lower.includes('could you') ||
        lower.includes('please') ||
        lower.includes('can you') ||
        lower.includes('see') ||
        lower.includes('found') ||
        lower.includes('missing') ||
        lower.includes('unavailable') ||
        lower.includes('not provided') ||
        lower.includes('cannot find') ||
        lower.includes('no mention') ||
        lower.includes('don\'t have') ||
        lower.includes('want me to') ||
        aiResponse.includes('?');

      if (isStalling && aiResponse.length > 10) {
        console.log("!!! Stalling behavior detected. Forcing tool usage !!!");
        const correction = `[STALLING DETECTED] You are asking a question or explaining why you can't act. 
STRICT RULE: If you don't see content, use 'read_file'. If you don't see a file, use 'list_directory'.
DO NOT talk. DO NOT ask the user for anything. 
USE TOOLS. ACTION ONLY.`;
        currentMessages.push({ role: 'assistant', content: aiResponse });
        currentMessages.push({ role: 'user', content: correction });
        continue;
      }
    }

    if (!toolCall) {
      // Not a tool call — this is the final text response
      conversationHistory.push({ role: 'assistant', content: aiResponse });
      console.log('[DONE] Agent finished with text response');
      return { finalResponse: aiResponse, steps };
    }

    // It's a tool call — execute it
    const toolName = toolCall.tool;
    const toolSummary = getToolSummary(toolCall);

    // Send step start to frontend
    const stepData = { tool: toolName, status: 'running', summary: toolSummary, iteration: iteration + 1 };
    win?.webContents.send('agent:step', stepData);
    steps.push(stepData);

    // Add assistant's tool request and result to local currentMessages
    currentMessages.push({ role: 'assistant', content: aiResponse });

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

    currentMessages.push({
      role: 'user',
      content: `[Tool Result: ${toolName}]\n${truncatedResult}\n\n[NEXT STEP: Use another tool if task is not complete, otherwise give your final text summary.]`
    });
  }

  // Final Response Management
  const finalAI = currentMessages[currentMessages.length - 1].content;
  conversationHistory.push({ role: 'user', content: userMessage });
  conversationHistory.push({ role: 'assistant', content: finalAI });

  return { finalResponse: finalAI, steps };
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
    case 'semantic_search': return `Searching semantically for "${toolCall.query}"`;
    case 'get_blast_radius': return `Calculating blast radius for ${toolCall.path}`;
    case 'apply_diffs': return `Applying diffs to ${toolCall.changes?.length || 0} files`;
    case 'validate_project': return 'Performing project-wide validation';
    case 'run_tests': return 'Running test suite';
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

ipcMain.handle('execute-agent-task', async (_event, { task, provider, workspacePath, activeFile, config }) => {
  try {
    // 1. Initial Workspace Scan (only if not loaded)
    if (workspacePath && !workspaceContextLoaded) {
      workspaceContextLoaded = true;
      console.log('📂 Performing Initial Workspace Context Build:', workspacePath);
      win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'running', summary: 'Building project context...' });

      const files = await readDirectoryRecursive(workspacePath, 2000);
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
    const result = await runAgentLoop(task, provider, config, workspacePath, activeFile);
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
