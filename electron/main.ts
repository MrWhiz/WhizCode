import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron'
import { join, dirname, isAbsolute } from 'node:path'
import { fileURLToPath } from 'node:url'
import { exec, spawn } from 'node:child_process'
import { promisify } from 'node:util'
import * as fs from 'node:fs/promises'
import * as os from 'node:os'
// Remove cross-fetch as global fetch is available in modern Electron/Node
import { createRequire } from 'node:module'
import * as chokidar from 'chokidar'
import { IndexingService } from './indexService'
import { CodeGraphService } from './graphService'
import { DiffService, type FileChange } from './diffService'
import { getSubAgentConfig, listSubAgents } from './subAgents'
import { HooksManager } from './hooksSystem'
import { SteeringManager } from './steeringSystem'
import { SpecsManager } from './specsSystem'
import { MCPManager } from './mcpService'
import { MemoryManager } from './memoryService'

const _require = createRequire(import.meta.url)
const pty = _require('node' + '-pty')

import { HistoryManager } from './historyService'

const execAsync = promisify(exec)

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// Error analysis and guidance function
function analyzeCommandError(errorOutput: string, command: string): string {
  const error = errorOutput.toLowerCase();
  let guidance = '';

  if (error.includes('operation cancelled')) {
    guidance = 'The command was cancelled, likely because it tried to show an interactive prompt (like "Overwrite existing files?") and stdin was closed. ' +
               'If you are using create-vite or similar, ensure the target directory is empty or choose a new name. ' +
               'You can use "list_directory" to check if the folder already exists.';
    return `ERROR: Operation cancelled.\n${guidance}`;
  }

  // Common error patterns and guidance
  if (error.includes('enoent') || error.includes('no such file or directory')) {
    if (command.includes('npm create vite') || command.includes('npm create vite@latest')) {
      // Check for malformed paths in the error
      const pathMatch = error.match(/path: ['"]([^'"]+)['"]/i);
      if (pathMatch && pathMatch[1].includes('\\') && pathMatch[1].split('\\').length > 5) {
        guidance = 'The path appears malformed. The create-vite command might be interpreting the path incorrectly. Try using a simpler path without spaces or use quotes. Example: npm create vite@latest "my-app" --template react-ts';
      } else {
        guidance = 'The vite command may not be installed. Try: npx -y create-vite@latest my-app --template react-ts';
      }
    } else if (command.includes('npm install') || command.includes('npm i ')) {
      guidance = 'npm might not be installed or not in PATH. Check if npm is installed with: npm --version';
    } else if (command.includes('mkdir') || command.includes('cd ')) {
      guidance = 'The directory path might contain spaces or special characters. Try using quotes around paths with spaces.';
    }
  }

  if (error.includes('command not found') || error.includes('is not recognized')) {
    if (command.includes('npm ') && command.includes('create')) {
      guidance = 'npm create command not found. Try: npx create-vite@latest my-app --template react-ts';
    } else if (command.includes('npx ')) {
      guidance = 'npx might not be installed or in PATH. Try using npm directly or check npx installation.';
    }
  }

  if (error.includes('permission denied') || error.includes('eacces')) {
    guidance = 'Permission denied. Try running with administrator/sudo privileges or check file permissions.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Port is already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('eaddrinuse') || error.includes('already in use')) {
    guidance = 'Port is already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('enoent') && error.includes('fatal: not a git repository')) {
    guidance = 'Not a git repository. Initialize git with: git init';
  }

  if (error.includes('module not found') || error.includes('cannot find module')) {
    const moduleMatch = error.match(/cannot find module ['"]([^'"]+)['"]/i);
    if (moduleMatch) {
      guidance = `Module not found: ${moduleMatch[1]}. Try: npm install ${moduleMatch[1]}`;
    } else {
      guidance = 'Module not found. Try running: npm install';
    }
  }

  if (error.includes('syntax error') || error.includes('syntaxerror')) {
    guidance = 'Syntax error in command or script. Check for typos or missing arguments.';
  }

  if (error.includes('econnrefused') || error.includes('connection refused')) {
    guidance = 'Connection refused. Check if the server/port is running and accessible.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network connectivity or server status.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Address already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('eacces') || error.includes('permission denied')) {
    guidance = 'Permission denied. Try running with elevated privileges or check file permissions.';
  }

  if (error.includes('enoent') && error.includes('no such file or directory')) {
    if (command.includes('cd ') && command.includes('mkdir')) {
      guidance = 'Directory does not exist. Check the path or create parent directories first.';
    } else {
      guidance = 'File or directory not found. Check the path and try again.';
    }
  }

  if (error.includes('command not found') || error.includes('is not recognized')) {
    if (command.includes('npm ') || command.includes('npx ')) {
      guidance = 'npm/npx not found. Make sure Node.js and npm are installed and in PATH.';
    } else {
      guidance = 'Command not found. Check if the command exists and is in your PATH.';
    }
  }

  if (error.includes('eexist') || error.includes('file already exists')) {
    guidance = 'File or directory already exists. Use a different name or remove the existing one.';
  }

  if (error.includes('enotempty') || error.includes('directory not empty')) {
    guidance = 'Directory is not empty. Remove or move files first.';
  }

  if (error.includes('eisdir') || error.includes('is a directory')) {
    guidance = 'Expected a file but found a directory, or vice versa.';
  }

  if (error.includes('eacces') || error.includes('permission denied')) {
    guidance = 'Permission denied. Check file permissions or run with elevated privileges.';
  }

  if (error.includes('econnrefused') || error.includes('connection refused')) {
    guidance = 'Connection refused. Check if the server is running and accessible.';
  }

  if (error.includes('etimedout') || error.includes('timeout')) {
    guidance = 'Connection timed out. Check network connectivity or server status.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network or server.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Address already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('econnrefused') || error.includes('connection refused')) {
    guidance = 'Connection refused. Check if the server is running and accessible.';
  }

  if (error.includes('econnaborted') || error.includes('connection aborted')) {
    guidance = 'Connection was aborted. Check network or server.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network or server.';
  }

  if (error.includes('ehostunreach') || error.includes('host unreachable')) {
    guidance = 'Host unreachable. Check network or host availability.';
  }

  if (error.includes('enetunreach') || error.includes('network unreachable')) {
    guidance = 'Network unreachable. Check network connection.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Address already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Address already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('econnrefused') || error.includes('connection refused')) {
    guidance = 'Connection refused. Check if the server is running and accessible.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network or server.';
  }

  if (error.includes('econnaborted') || error.includes('connection aborted')) {
    guidance = 'Connection was aborted. Check network or server.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network or server.';
  }

  if (error.includes('ehostunreach') || error.includes('host unreachable')) {
    guidance = 'Host unreachable. Check network or host.';
  }

  if (error.includes('enetunreach') || error.includes('network unreachable')) {
    guidance = 'Network unreachable. Check network connection.';
  }

  if (error.includes('eaddrinuse') || error.includes('address already in use')) {
    guidance = 'Address already in use. Try a different port or kill the process using the port.';
  }

  if (error.includes('econnrefused') || error.includes('connection refused')) {
    guidance = 'Connection refused. Check if the server is running and accessible.';
  }

  if (error.includes('econnaborted') || error.includes('connection aborted')) {
    guidance = 'Connection was aborted. Check network or server.';
  }

  if (error.includes('econnreset') || error.includes('connection reset')) {
    guidance = 'Connection was reset. Check network or server.';
  }

  if (error.includes('ehostunreach') || error.includes('host unreachable')) {
    guidance = 'Host unreachable. Check network or host.';
  }

  if (error.includes('enetunreach') || error.includes('network unreachable')) {
    guidance = 'Network unreachable. Check network connection.';
  }

  // Add the original error and guidance
  let result = errorOutput;
  if (guidance) {
    result += `\n\n[ERROR ANALYSIS]\n${guidance}\n\nSuggested fix: ${guidance}`;
  }

  return result;
}

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
let agentAbortController: AbortController | null = null;
let currentActiveProcess: any | null = null;
let currentWorkspacePath: string | null = null;
let hooksManager: HooksManager | null = null;
let steeringManager: SteeringManager | null = null;
let specsManager: SpecsManager | null = null;
let mcpManager: MCPManager | null = null;
let memoryManager: MemoryManager | null = null;

let currentHistoryManager: HistoryManager | null = null;
let currentConversationId: string = Date.now().toString();

// Rolling buffer of PTY terminal output — fed into agent context on each request
const terminalOutputBuffer: string[] = [];

// Ollama Configuration
const OLLAMA_URL = 'http://127.0.0.1:11434/api/chat';
const MODEL_NAME = 'deepseek-coder-v2:latest'; // Optimized for local coding

// Workspace persistence
const WORKSPACE_STORAGE_FILE = join(app.getPath('userData'), 'last-workspace.json');
const WINDOW_BOUNDS_FILE = join(app.getPath('userData'), 'window-bounds.json');

async function saveLastWorkspace(workspacePath: string) {
  try {
    await fs.writeFile(WORKSPACE_STORAGE_FILE, JSON.stringify({ lastWorkspace: workspacePath }), 'utf-8');
  } catch (e) {
    console.error('Failed to save last workspace:', e);
  }
}

async function loadLastWorkspace(): Promise<string | null> {
  try {
    const data = await fs.readFile(WORKSPACE_STORAGE_FILE, 'utf-8');
    const parsed = JSON.parse(data);
    return parsed.lastWorkspace || null;
  } catch (e) {
    return null;
  }
}

async function saveWindowBounds() {
  try {
    if (!win) return;
    const bounds = win.getBounds();
    await fs.writeFile(WINDOW_BOUNDS_FILE, JSON.stringify(bounds), 'utf-8');
  } catch (e) {
    console.error('Failed to save window bounds:', e);
  }
}

async function loadWindowBounds(): Promise<{ x: number; y: number; width: number; height: number } | null> {
  try {
    const data = await fs.readFile(WINDOW_BOUNDS_FILE, 'utf-8');
    return JSON.parse(data);
  } catch (e) {
    return null;
  }
}

async function createWindow() {
  const savedBounds = await loadWindowBounds();

  win = new BrowserWindow({
    x: savedBounds?.x,
    y: savedBounds?.y,
    width: savedBounds?.width || 1200,
    height: savedBounds?.height || 800,
    minWidth: 800,
    minHeight: 600,
    show: false,
    webPreferences: {
      preload: join(__dirname, 'preload.mjs'),
      nodeIntegration: false,
      contextIsolation: true,
      webviewTag: true
    },
    titleBarStyle: 'hidden',
    titleBarOverlay: {
      color: '#1e1e1e', // VSCode Dark Theme 
      symbolColor: '#cccccc',
      height: 30
    },
    backgroundColor: '#1e1e1e'
  })

  // Save window bounds when moved or resized
  win.on('resize', () => saveWindowBounds());
  win.on('move', () => saveWindowBounds());

  // Show maximized once ready
  win.once('ready-to-show', () => {
    win?.maximize();
    win?.show();
  });

  // Open DevTools automatically if in dev environment
  if (VITE_DEV_SERVER_URL) {
    win.loadURL(VITE_DEV_SERVER_URL)
  } else {
    win.loadFile(join(RENDERER_DIST, 'index.html'))
  }

  // Load last workspace after window is ready
  win.webContents.on('did-finish-load', async () => {
    const lastWorkspace = await loadLastWorkspace();
    if (lastWorkspace) {
      try {
        // Check if the path still exists
        await fs.access(lastWorkspace);
        currentWorkspacePath = lastWorkspace;
        setupWorkspaceWatcher(lastWorkspace);
        win?.webContents.send('workspace:restored', lastWorkspace);
      } catch (e) {
        console.log('Last workspace no longer exists:', lastWorkspace);
      }
    }
  });
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

// ====== HELPER FUNCTIONS FOR NEW TOOLS ======

// Fast fuzzy file search
async function fuzzyFindFile(workspacePath: string, query: string, maxResults = 10): Promise<string> {
  const results: { path: string; score: number }[] = [];
  const queryLower = query.toLowerCase();

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
          const fileName = entry.name.toLowerCase();
          const relPath = fullPath.replace(workspacePath, '').replace(/^[\\/]/, '');

          // Calculate match score
          let score = 0;
          if (fileName === queryLower) score = 100; // Exact match
          else if (fileName.startsWith(queryLower)) score = 80; // Prefix match
          else if (fileName.includes(queryLower)) score = 50; // Contains match
          else if (relPath.toLowerCase().includes(queryLower)) score = 30; // Path contains match

          if (score > 0) {
            results.push({ path: relPath, score });
          }
        }
      }
    } catch { /* skip */ }
  }

  await walk(workspacePath);
  results.sort((a, b) => b.score - a.score);

  if (results.length === 0) return `No files found matching "${query}".`;
  return results.map(r => `- ${r.path} (score: ${r.score})`).join('\n');
}

// Get TypeScript/ESLint diagnostics
async function getDiagnostics(filePath: string, workspacePath: string | null): Promise<string> {
  const resolvedPath = resolvePath(filePath, workspacePath);

  try {
    // Check if file exists
    await fs.access(resolvedPath);

    // Check for tsconfig.json
    const tsconfigPath = join(workspacePath || '.', 'tsconfig.json');
    try {
      await fs.access(tsconfigPath);

      // Run tsc to get diagnostics
      const { stdout, stderr } = await execAsync(`npx tsc --noEmit --pretty false "${resolvedPath}"`, {
        cwd: workspacePath || '.',
        maxBuffer: 1024 * 1024 * 10
      });

      if (stdout) return `TypeScript Diagnostics for ${filePath}:\n${stdout}`;
      if (stderr) return `TypeScript Diagnostics for ${filePath}:\n${stderr}`;
      return `✅ No TypeScript errors in ${filePath}`;
    } catch {
      // No tsconfig.json, try ESLint
      try {
        const { stdout, stderr } = await execAsync(`npx eslint "${resolvedPath}"`, {
          cwd: workspacePath || '.',
          maxBuffer: 1024 * 1024 * 10
        });

        if (stdout) return `ESLint Diagnostics for ${filePath}:\n${stdout}`;
        if (stderr) return `ESLint Diagnostics for ${filePath}:\n${stderr}`;
        return `✅ No ESLint errors in ${filePath}`;
      } catch {
        return `No configuration found (tsconfig.json or .eslintrc). Skipping diagnostics.`;
      }
    }
  } catch {
    return `❌ File not found: ${filePath}`;
  }
}

// Fast grep search with line numbers
async function grepSearch(workspacePath: string, pattern: string, includePattern?: string, maxResults = 50): Promise<string> {
  const results: string[] = [];
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
          if (includePattern) {
            const ext = entry.name.split('.').pop()?.toLowerCase();
            const filterExt = includePattern.replace('*.', '').toLowerCase();
            if (ext !== filterExt) continue;
          }

          const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
          if (BINARY_EXTS.has(ext)) continue;

          try {
            const stat = await fs.stat(fullPath);
            if (stat.size > 500_000) continue; // Skip very large files

            const content = await fs.readFile(fullPath, 'utf-8');
            const lines = content.split('\n');

            for (let i = 0; i < lines.length; i++) {
              if (regex.test(lines[i])) {
                const relPath = fullPath.replace(workspacePath, '').replace(/^[\\/]/, '');
                results.push(`${relPath}:${i + 1}: ${lines[i].trim()}`);
                if (results.length >= maxResults) break;
              }
              regex.lastIndex = 0;
            }
          } catch { /* skip */ }
        }
      }
    } catch { /* skip */ }
  }

  await walk(workspacePath || '.');
  return results.length > 0 ? results.join('\n') : `No matches found for "${pattern}".`;
}

// Read multiple files at once
async function readMultipleFiles(files: string[], workspacePath: string | null): Promise<string> {
  const results: { path: string; content: string; error?: string }[] = [];

  for (const filePath of files) {
    const resolvedPath = resolvePath(filePath, workspacePath);
    try {
      const isBinary = await isBinaryFile(resolvedPath);
      if (isBinary) {
        results.push({ path: filePath, content: '', error: 'Binary file' });
        continue;
      }

      const content = await fs.readFile(resolvedPath, 'utf-8');
      const lines = content.split('\n');
      results.push({
        path: filePath,
        content: lines.map((line, i) => `${i + 1}: ${line}`).join('\n')
      });
    } catch (e: any) {
      results.push({ path: filePath, content: '', error: e.message });
    }
  }

  return results.map(r =>
    r.error
      ? `❌ ${r.path}: ${r.error}`
      : `📄 ${r.path}:\n${r.content.substring(0, 5000)}${r.content.length > 5000 ? '\n... (truncated)' : ''}`
  ).join('\n\n' + '='.repeat(50) + '\n\n');
}

// Rename symbol with reference updates
async function semanticRename(
  _filePath: string,
  oldName: string,
  newName: string,
  workspacePath: string | null
): Promise<string> {
  try {
    // First, find all references
    const references: { path: string; line: number; content: string }[] = [];

    async function findReferences(currentPath: string) {
      try {
        const entries = await fs.readdir(currentPath, { withFileTypes: true });
        for (const entry of entries) {
          const fullPath = join(currentPath, entry.name);
          if (entry.isDirectory()) {
            if (!SKIP_DIRS.has(entry.name) && !entry.name.startsWith('.')) {
              await findReferences(fullPath);
            }
          } else {
            const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
            if (!['.ts', '.tsx', '.js', '.jsx'].includes(ext)) continue;

            try {
              const content = await fs.readFile(fullPath, 'utf-8');
              const lines = content.split('\n');
              const regex = new RegExp(`\\b${oldName}\\b`, 'g');

              for (let i = 0; i < lines.length; i++) {
                if (regex.test(lines[i])) {
                  references.push({
                    path: fullPath.replace(workspacePath || '', '').replace(/^[\\/]/, ''),
                    line: i + 1,
                    content: lines[i].trim()
                  });
                }
              }
            } catch { /* skip */ }
          }
        }
      } catch { /* skip */ }
    }

    await findReferences(workspacePath || '.');

    if (references.length === 0) {
      return `No references found for "${oldName}".`;
    }

    // Now rename in all files
    let renamedCount = 0;
    for (const ref of references) {
      const fullPath = join(workspacePath || '.', ref.path);
      try {
        let content = await fs.readFile(fullPath, 'utf-8');
        const newContent = content.replace(new RegExp(`\\b${oldName}\\b`, 'g'), newName);
        if (newContent !== content) {
          await fs.writeFile(fullPath, newContent, 'utf-8');
          renamedCount++;
        }
      } catch { /* skip */ }
    }

    return `✅ Renamed "${oldName}" to "${newName}" in ${renamedCount} locations.\n\nFound ${references.length} references total.`;
  } catch (e: any) {
    return `❌ Error renaming symbol: ${e.message}`;
  }
}

// Move file with import updates
async function smartRelocate(
  sourcePath: string,
  destinationPath: string,
  workspacePath: string | null
): Promise<string> {
  const resolvedSource = resolvePath(sourcePath, workspacePath);
  const resolvedDest = resolvePath(destinationPath, workspacePath);

  try {
    // Check source exists
    await fs.access(resolvedSource);

    // Create destination directory if needed
    const destDir = dirname(resolvedDest);
    await fs.mkdir(destDir, { recursive: true });

    // Read source content
    const content = await fs.readFile(resolvedSource, 'utf-8');

    // Update imports in the file being moved
    let newContent = content;
    const oldImportPath = sourcePath.replace(workspacePath || '', '').replace(/^[\\/]/, '').replace(/\\/g, '/');
    const newImportPath = destinationPath.replace(workspacePath || '', '').replace(/^[\\/]/, '').replace(/\\/g, '/');

    // Update relative imports in the moved file
    const relativeImportRegex = new RegExp(`(['"])\\.\\.?/${oldImportPath.replace('.', '\\.')}`, 'g');
    newContent = newContent.replace(relativeImportRegex, `$1../${newImportPath}`);

    // Write to new location
    await fs.writeFile(resolvedDest, newContent, 'utf-8');

    // Update imports in other files
    let importsUpdated = 0;
    async function updateImports(currentPath: string) {
      try {
        const entries = await fs.readdir(currentPath, { withFileTypes: true });
        for (const entry of entries) {
          const fullPath = join(currentPath, entry.name);
          if (entry.isDirectory()) {
            if (!SKIP_DIRS.has(entry.name) && !entry.name.startsWith('.')) {
              await updateImports(fullPath);
            }
          } else {
            const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
            if (!['.ts', '.tsx', '.js', '.jsx'].includes(ext)) continue;

            try {
              let fileContent = await fs.readFile(fullPath, 'utf-8');
              const importRegex = new RegExp(`(['"])\\.\\.?/${oldImportPath.replace('.', '\\.')}`, 'g');

              if (importRegex.test(fileContent)) {
                fileContent = fileContent.replace(importRegex, `$1../${newImportPath}`);
                await fs.writeFile(fullPath, fileContent, 'utf-8');
                importsUpdated++;
              }
            } catch { /* skip */ }
          }
        }
      } catch { /* skip */ }
    }

    await updateImports(workspacePath || '.');

    // Delete old file
    await fs.unlink(resolvedSource);

    return `✅ Moved ${sourcePath} to ${destinationPath}.\nUpdated ${importsUpdated} import statements.`;
  } catch (e: any) {
    return `❌ Error relocating file: ${e.message}`;
  }
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

const KIRO_SYSTEM_PROMPT = `
<identity>
You are WhizCode, an Autonomous Engineering Agent. 

Your goal is to build, debug, and maintain software by DIRECTLY using the tools provided to you. You are NOT a documentation assistant or a teacher. You are an OPERATOR.
</identity>

<capabilities>
- Direct access to the local file system (read/write/edit)
- Direct shell execution (run_command)
- Workspace indexing and semantic code search
- Structured project management via Specs
- Web access for research and documentation
</capabilities>

<rules>
- NEVER give the user steps to follow manually. If a file needs to be created, CREATE IT. If a command needs to be run, RUN IT.
- NEVER ask the user to "Run this command" or "Create this file." Use your tools immediately.
- Be extremely surgical. Read files before editing. Verify changes after writing.
- If you are stuck or a command fails, do NOT repeat the same mistake. Analyze the logs and change your strategy (e.g., read a different file, check a different directory).
- Talk like a senior engineer: concise, technically precise, and action-oriented.
- Use <THOUGHT> tags for your internal reasoning before outputting a tool call or final response.
</rules>

<response_style>
- No fluff. No "Certainly!", "I can help with that", or "Here are the steps."
- State what you are doing, then do it.
- After completing a task, provide a MINIMAL summary (2-3 sentences) of what was accomplished.
- When you are finished with the entire user request, include the phrase "TASK COMPLETE" in your final response.
</response_style>

<tool_usage_guidelines>
**Available Tools:**

**Reading & Discovery:**
- read_file: Read a single file
- readCode: Read code files with structure analysis
- readMultipleFiles: Read multiple files at once
- list_directory: List directory contents
- search_files: Search for files by pattern
- grepSearch: Search file contents with regex
- fileSearch: Fuzzy file name search

**Writing & Editing:**
- write_file: Create new files or overwrite existing ones
- edit_file: Make targeted edits to existing files
- editCode: AST-aware code editing
- strReplace: Simple string replacement
- delete_file: Remove files

**Execution:**
- run_command: Execute shell commands (requires user approval)
  - ALWAYS use the "cwd" parameter to run in a specific directory.
  - NEVER use standalone "cd" commands.
  - Paths in "command" AND "cwd" should be RELATIVE to the workspace root unless absolutely necessary.
  - For project creation: use "." (the current workspace) as the cwd if you want it in the root, or a subdirectory name if you want it nested.
  - IMPORTANT: CLI commands are run non-interactively. ALWAYS use flags to auto-accept prompts (e.g., -y, --yes), otherwise the command will hang indefinitely.

**Analysis:**
- getDiagnostics: Get TypeScript/ESLint errors
- semanticRename: Rename symbols with auto-update references
- smartRelocate: Move files with auto-update imports

**Sub-Agents:**
- invokeSubAgent: Delegate tasks to specialized sub-agents
- listSubAgents: List all available sub-agents

**Web Access:**
- web_search: Search the web for current information, documentation, and error solutions
- webFetch: Fetch full content from a URL (stripped of HTML tags)

**Spec / Feature Management:**
- createSpec: Initialize a feature development lifecycle
- readSpec: Get current state for a feature spec
- updateSpec: Modify specific spec documents
- listSpecs: View all active and pending feature specs
- completeTask: Mark a task as done
</tool_usage_guidelines>

<output_format>
All tool calls MUST be valid JSON on a single line.
{"tool": "tool_name", "param1": "value1"}

⚠️ CRITICAL: Output the JSON tool call as plain text ONLY.
- Do NOT wrap it in markdown code fences.
- Do NOT add any text before or after the JSON.
</output_format>




<system_context>
Operating System: ${process.platform}
Shell: ${process.platform === 'win32' ? 'Windows Command Prompt (cmd.exe via shell:true)' : 'bash'}
Current Date: ${new Date().toLocaleDateString()}
</system_context>

<windows_shell_rules>
CRITICAL - READ CAREFULLY:
- You are running on Windows using cmd.exe (shell:true). Use Windows-compatible commands.
- Do NOT use bash-only syntax like $(), source, export, etc.
- NEVER use "cd" as a standalone run_command. Each command runs in a fresh shell.
- To run a command in a specific directory, ALWAYS use the "cwd" parameter.
  WRONG: {"tool": "run_command", "command": "cd my-app && npm install"}
  RIGHT: {"tool": "run_command", "command": "npm install", "cwd": "my-app"}
- Windows paths with spaces MUST be quoted: "C:\\My Folder\\app"
- Use RELATIVE paths for all operations inside the workspace.
  Example: To edit "src/main.ts", just use "src/main.ts", NOT "C:\\Users\\...\\src\\main.ts".
- After creating a project folder (e.g. "my-app"), use "cwd": "my-app" for all subsequent commands inside it.
- If a command fails, DO NOT retry the same command. Analyze the error and change your approach.
</windows_shell_rules>
`;

let conversationHistory: any[] = [];
let workspaceManifest: string = '';

let workspaceContextLoaded = false;

// ====== LLM PROVIDER CALLS ======
async function callAI(messages: any[], modelConfig: { provider: string, model: string }, config: any, signal?: AbortSignal, temperature: number = 0.1) {
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
          temperature: temperature || 0.1
        }),
        signal
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
      body.generationConfig = {
        temperature: temperature || 0.1
      };

      response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/${model || 'gemini-1.5-flash'}:generateContent?key=${config.geminiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal
      });
      if (!response.ok) throw new Error(`Gemini HTTP Error: ${response.status} ${await response.text()}`);
      data = await response.json();
      
      const candidate = data.candidates?.[0];
      if (!candidate?.content?.parts) return '';

      // Handle multi-part responses (text + potential native tool calls)
      let combinedContent = '';
      for (const part of candidate.content.parts) {
        if (part.text) combinedContent += part.text;
        if (part.functionCall) {
          // Convert native functionCall to our JSON format so tryParseToolCall can find it
          combinedContent += `\n{"tool": "${part.functionCall.name}", ${JSON.stringify(part.functionCall.args).slice(1)}`;
        }
      }
      return combinedContent;
    } else {
      // ── Ollama — Streaming mode ─────────────────────────────────────
      response = await fetch(OLLAMA_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: model || MODEL_NAME,
          messages: messages,
          stream: true,
          options: {
            temperature: temperature || 0,
            num_ctx: 32768,   // CRITICAL: default is often 2048 — far too small
            repeat_penalty: 1.1
          }
        }),
        signal
      });
      if (!response.ok) throw new Error(`Ollama HTTP Error: ${response.status}`);

      // Read the NDJSON stream and accumulate content
      const reader = response.body?.getReader();
      if (!reader) throw new Error('Ollama stream body is null');
      const decoder = new TextDecoder();
      let fullContent = '';
      let streamBuffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        streamBuffer += decoder.decode(value, { stream: true });
        const lines = streamBuffer.split('\n');
        streamBuffer = lines.pop() || '';
        for (const line of lines) {
          if (!line.trim()) continue;
          try {
            const chunk = JSON.parse(line);
            const content = chunk.message?.content || '';
            if (content) {
              fullContent += content;
              win?.webContents.send('agent:stream', { token: content });
            }
            
            // Handle native tool calls (Ollama/DeepSeek native format support)
            if (chunk.message?.tool_calls && Array.isArray(chunk.message.tool_calls)) {
              for (const tc of chunk.message.tool_calls) {
                if (tc.function) {
                  const toolJson = `\n{"tool": "${tc.function.name}", ${JSON.stringify(tc.function.arguments || {}).slice(1)}`;
                  fullContent += toolJson;
                  win?.webContents.send('agent:stream', { token: toolJson });
                }
              }
            }
            
            if (chunk.done) break;
          } catch { /* skip malformed lines */ }
        }
      }
      return fullContent;
    }
  } catch (error: any) {
    if (error.name === 'AbortError') throw new Error('Task stopped by user');
    console.error("AI Provider Error:", error);
    throw error;
  }
}

// ====== TOOL PARSER ======

function tryParseAllToolCalls(response: string): any[] {
  if (!response) return [];
  const trimmed = response.trim();
  const toolCalls: any[] = [];

  const findJsonBlocksDetailed = (text: string) => {
    const blocks: { text: string; start: number; end: number }[] = [];
    let start = -1;
    let balance = 0;
    let inQuote = false;

    for (let i = 0; i < text.length; i++) {
      const char = text[i];
      if (char === '"' && text[i - 1] !== '\\') {
        inQuote = !inQuote;
      }
      if (!inQuote) {
        if (char === '{') {
          if (balance === 0) start = i;
          balance++;
        } else if (char === '}') {
          balance--;
          if (balance === 0 && start !== -1) {
            blocks.push({ text: text.substring(start, i + 1), start, end: i + 1 });
            start = -1;
          }
        }
      }
    }
    return blocks;
  };

  const blocks = findJsonBlocksDetailed(trimmed);
  const toolWords = ['run_command', 'write_file', 'read_file', 'edit_file', 'list_directory', 'search_files', 'delete_file', 'grepSearch', 'fileSearch', 'readCode', 'getDiagnostics', 'semanticRename', 'smartRelocate', 'createSpec', 'readSpec', 'updateSpec', 'listSpecs', 'completeTask', 'web_search', 'webFetch', 'mcp_call', 'learn_fact', 'replace_lines', 'insert_code'];

  for (const blockData of blocks) {
    const block = blockData.text;
    try {
      const attemptParse = (text: string) => {
        try {
          const p = JSON.parse(text);
          // Standard JSON format
          if (p.tool) return p;
          // Format from some OpenAI proxies
          if (p.tool_calls && Array.isArray(p.tool_calls) && p.tool_calls[0]?.tool) return p.tool_calls[0];
          
          // DeepSeek Fallback: the tool name is in the text context immediately before the block
          const contextText = response.substring(Math.max(0, blockData.start - 120), blockData.start);
          
          // Pattern: < | tool_sep | > tool_name
          const dsMatch = contextText.match(/tool_sep\s*[\|>]\s*>\s*(\w+)/) || contextText.match(/<\|tool_sep\|>\s*(\w+)/);
          if (dsMatch && toolWords.includes(dsMatch[1])) {
            p.tool = dsMatch[1];
            return p;
          }

          // More general: scan for tool name in proximity
          let bestTool = '';
          let lastToolIdx = -1;
          for (const word of toolWords) {
            const idx = contextText.lastIndexOf(word);
            if (idx > lastToolIdx) {
               bestTool = word;
               lastToolIdx = idx;
            }
          }
          if (bestTool) {
             p.tool = bestTool;
             return p;
          }
          return null;
        } catch { return null; }
      };

      // 1. Try straight parse
      let parsed = attemptParse(block);
      if (parsed) {
        toolCalls.push(parsed);
        continue;
      }

      // 2. Proactive JSON Fixer (Ultimate fix for smaller models/Ollama)
      let currentTry = block;
      
      // Fix unescaped quotes in known params
      currentTry = currentTry.replace(/"(content|command|search|replace|path|cwd|reasoning|template)"\s*:\s*"([\s\S]*?)"(?=\s*[,}\]])/g, (_, key, val) => {
        const escapedVal = val.replace(/(?<!\\)"/g, '\\"');
        return `"${key}": "${escapedVal}"`;
      });

      // Fix unescaped backslashes (Windows paths)
      currentTry = currentTry.replace(/"(path|cwd|cwd_override)"\s*:\s*"([\s\S]*?)"/g, (_, key, val) => {
          const escapedVal = val.replace(/\\(?![\\\/bfnrtu"'])/g, '\\\\');
          return `"${key}": "${escapedVal}"`;
      });
      
      // Clear out illegal control characters inside strings
      currentTry = currentTry.replace(/[\x00-\x1F\x7F-\x9F]/g, " ");

      parsed = attemptParse(currentTry);
      if (parsed) {
        toolCalls.push(parsed);
        continue;
      }

      // 3. Fallback: Aggressive quote cleaning
      const superFixed = currentTry.replace(/\\+"/g, '\\"');
      parsed = attemptParse(superFixed);
      if (parsed) {
        toolCalls.push(parsed);
      }

    } catch (err) {
      console.warn(`[TOOL] Block exploration failed for: ${block.substring(0, 30)}...`, err);
    }
  }

  // Regex fallback ONLY if no blocks found
  if (toolCalls.length === 0) {
    const toolMatch = /"tool"\s*:\s*"([^"]+)"/.exec(trimmed);
    if (toolMatch) {
      const tool = toolMatch[1];
      const pathMatch = /"path"\s*:\s*"([^"]+)"/.exec(trimmed);
      const commandMatch = /"command"\s*:\s*"([\s\S]*?)"(?=\s*[,}])/.exec(trimmed);
      if (tool === 'run_command' && commandMatch) toolCalls.push({ tool, command: commandMatch[1].replace(/\\"/g, '"') });
      else if (tool === 'read_file' && pathMatch) toolCalls.push({ tool, path: pathMatch[1] });
    }
  }

  return toolCalls;
}

// Deprecated — use tryParseAllToolCalls
function tryParseToolCall(response: string): any | null {
  const all = tryParseAllToolCalls(response);
  return all.length > 0 ? all[0] : null;
}

// ====== TOOL EXECUTOR ======


// ====== TOOL EXECUTOR ======

// Patterns considered destructive — blocked in autopilot mode
const DANGEROUS_COMMAND_PATTERNS = [
  /rm\s+-rf?\s+[\/~C-Z:\\]/i,
  /del\s+\/[sf]/i,
  /rmdir\s+\/s/i,
  /format\s+[a-z]:/i,
  /rd\s+\/s/i,
  /DROP\s+TABLE/i,
  /DROP\s+DATABASE/i,
  /:\s*\(\s*\)\s*\{.*fork bomb/i,
];

async function executeToolCall(toolData: any, workspacePath: string | null, iteration?: number, isAutopilotMode: boolean = false, toolModel?: { provider: string, model: string }, config?: any): Promise<{ result: string; logs?: string[]; abort?: boolean; data?: any }> {
  const resolvedPath = toolData.path ? resolvePath(toolData.path, workspacePath) : '';
  console.log(`\n[TOOL] [${toolData.tool}] ${resolvedPath || toolData.command || toolData.pattern || ''}`);

  const requestApproval = async (summary: string) => {
    if (isAutopilotMode) return true;
    console.log(`[APPROVAL] Requesting permission for: ${summary}`);
    win?.webContents.send('agent:step', {
      tool: toolData.tool,
      status: 'awaiting_permission',
      summary,
      iteration: iteration
    });
    const decision = await new Promise<{ approved: boolean }>(resolve => {
      pendingPermissionResolver = resolve;
    });
    pendingPermissionResolver = null;
    console.log(`[APPROVAL] Decision received: ${decision.approved}`);
    return decision.approved;
  };

  // Fire preToolUse hooks
  if (hooksManager) {
    try {
      const preHooks = await hooksManager.triggerToolEvent('preToolUse', toolData.tool);
      for (const hook of preHooks) {
        if (hook.action === 'runCommand' && hook.command) {
          console.log(`[HOOK] preToolUse → runCommand: ${hook.command}`);
          await execAsync(hook.command, { cwd: workspacePath || '.', timeout: (hook.timeout || 30) * 1000 }).catch(e => {
            console.warn(`[HOOK] preToolUse command failed: ${e.message}`);
          });
        }
      }
    } catch (e) {
      console.warn('[HOOK] preToolUse error:', e);
    }
  }

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

        // Request approval
        if (!(await requestApproval(`Write file: ${toolData.path}`))) {
          return { result: '❌ File write denied by user.', abort: true };
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

        // NOTE: Auto-diagnostics removed — too slow (5-15s per write with tsc).
        // The agent can explicitly call getDiagnostics when needed.
        return { result: `✅ Successfully wrote ${lineCount} lines to ${toolData.path}` };
      }

      case 'edit_file': {
        if (!toolData.path) return { result: '❌ Error: Tool "edit_file" requires a "path" parameter.' };
        if (!toolData.edits) return { result: '❌ Error: Tool "edit_file" requires an "edits" parameter.' };

        // Request approval
        if (!(await requestApproval(`Edit file: ${toolData.path} (${toolData.edits?.length || 0} edits)`))) {
          return { result: '❌ File edit denied by user.', abort: true };
        }

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

        // NOTE: Auto-diagnostics removed — too slow (5-15s per edit with tsc).
        // The agent can explicitly call getDiagnostics when needed.
        return {
          result: `✅ Applied ${editCount} edit(s) to ${toolData.path}`,
          data: { path: toolData.path, edits: toolData.edits }
        };
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

        // Intercept "cd ... &&" patterns — they are often signs of the agent ignoring the "cwd" parameter
        const cdMatch = command.match(/^"?cd\s+([^&"|;]+)"?\s*(&&|;|\|)/i);
        if (cdMatch) {
          const targetDir = cdMatch[1].trim();
          return {
            result: `❌ ERROR: Use the "cwd" parameter instead of "cd" within the command string.\n` +
              `Your command tried to 'cd' into: ${targetDir}\n` +
              `Please resubmit using: {"tool": "run_command", "command": "${command.replace(cdMatch[0], '').trim()}", "cwd": "${targetDir}"}`
          };
        }

        // Intercept bare "cd" commands
        if (/^"?cd\s+/i.test(command.trim()) && !command.includes('&&') && !command.includes(';')) {
          return {
            result: `❌ ERROR: "cd" has no effect as a standalone command — each run_command spawns a fresh shell.\n` +
              `Use the "cwd" parameter instead to set the working directory.\n` +
              `Example: {"tool": "run_command", "command": "npm install", "cwd": "${command.replace(/^"?cd\s+/i, '').replace(/"/g, '').trim()}"}`
          };
        }

        // Autopilot safety: block dangerous/destructive commands
        if (isAutopilotMode) {
          const isDangerous = DANGEROUS_COMMAND_PATTERNS.some(p => p.test(command));
          if (isDangerous) {
            console.warn(`[AUTOPILOT SAFETY] Blocked destructive command: ${command}`);
            return {
              result: `❌ Autopilot safety blocked this command as potentially destructive: ${command}\nRequires manual approval.`,
              abort: true
            };
          }
        }

        // Support optional cwd parameter for running commands in subdirectories
        // Resolve cwd: if agent gives an absolute path, use it directly; otherwise join with workspace
        const rawCwd = toolData.cwd || '';
        const isAbsoluteCwd = /^[A-Za-z]:[\\/]/.test(rawCwd) || rawCwd.startsWith('/');
        const commandCwd = rawCwd
          ? (isAbsoluteCwd ? rawCwd : join(workspacePath!, rawCwd))
          : workspacePath;

        // Request approval
        if (!(await requestApproval(`Execute: ${command}${toolData.cwd ? ` (in ${toolData.cwd})` : ''}`))) {
          return { result: '❌ Command denied by user.', abort: true };
        }

        const logs: string[] = [];

        // Transition to running if approved
        win?.webContents.send('agent:step', {
          tool: 'run_command',
          status: 'running',
          summary: `Executing: ${command}${toolData.cwd ? ` (in ${toolData.cwd})` : ''}`,
          logs: logs,
          iteration: iteration
        });

        try {
          if (ptyProcess) {
            ptyProcess.write(`\r\n# Executing agent command: ${command}\r\n`);
          }

          // Pre-check for directory creation commands (common source of "Operation cancelled" if dir exists)
          if ((command.includes('create-vite') || command.includes('create vite')) && command.includes('--template')) {
            // Try to find the project name - usually the first non-flag argument after create-vite
            const parts = command.split(' ').filter((p: string) => p.trim());
            let targetDirName = '';
            for (let i = 0; i < parts.length; i++) {
              if (parts[i].includes('create-vite') || (parts[i] === 'create' && parts[i+1] === 'vite')) {
                // Skip the next part if it was 'vite'
                const nextIdx = parts[i] === 'create' ? i + 2 : i + 1;
                // Find next non-flag argument
                for (let j = nextIdx; j < parts.length; j++) {
                  if (!parts[j].startsWith('-')) {
                    targetDirName = parts[j];
                    break;
                  }
                }
                break;
              }
            }

            if (targetDirName) {
              const fullTargetDir = isAbsolute(targetDirName) ? targetDirName : join(commandCwd!, targetDirName);
              try {
                await fs.access(fullTargetDir);
                const files = await fs.readdir(fullTargetDir);
                if (files.length > 0) {
                  logs.push(`âš ï¸ Warning: Target directory is not empty: ${targetDirName}`);
                  logs.push(`âš ï¸ This will likely cause an "Operation cancelled" error if not handled.`);
                }
              } catch {
                // Directory doesn't exist, perfect
              }
            }
          }

          const fullOutput = await new Promise<string>((resolve, reject) => {
            // Spawn with CI=true + piped stdin so non-interactive CLIs don't hang
            const spawnEnv = { ...process.env, CI: '1', NO_COLOR: '1', FORCE_COLOR: '0' };
            const child = spawn(command, [], { cwd: commandCwd, shell: true, stdio: ['pipe', 'pipe', 'pipe'], env: spawnEnv });
            // Close stdin immediately to signal EOF (prevents interactive prompts from blocking)
            child.stdin?.end();
            currentActiveProcess = child;
            let output = '';
            let isResolved = false;

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
                  logs: [...logs],
                  iteration: iteration
                });

                // Early success detection: If it looks like a server started, 
                // return early so the agent doesn't stall.
                const successMarkers = [
                  'ready in', 'started on', 'local:', 'network:',
                  'listening on', 'compiled successfully', 'available at',
                  'vitedevserver', 'server running', 'ready on', 'successfully compiled',
                  'process started', 'server started'
                ];
                const lowerStr = str.toLowerCase();
                if (successMarkers.some(marker => lowerStr.includes(marker))) {
                  if (!isResolved) {
                    setTimeout(() => { // Small delay to catch a few more lines
                      if (isResolved) return;
                      isResolved = true;
                      clearTimeout(timeout);
                      currentActiveProcess = null;
                      resolve(`${output.trim()}\n\n[INFO]: Server/Process detected as READY and running in background.`);
                    }, 1000);
                  }
                }
              }
            };

            child.stdout?.on('data', handleData);
            child.stderr?.on('data', handleData);

            // Safety timeout: Only kick in for truly stuck/infinite processes (10 min).
            // Normal commands (npm install, npx create, etc.) must complete fully.
            // Server processes are handled early via successMarkers above.
            const timeout = setTimeout(() => {
              if (isResolved) return;
              isResolved = true;
              currentActiveProcess = null;
              const resultMsg = output.trim() || '(No output yet)';
              resolve(`${resultMsg}\n\n[INFO]: Command timed out after 10 minutes. It may still be running in background.`);
            }, 600000); // 10 minute hard safety timeout

            child.on('close', (code) => {
              if (isResolved) return;
              clearTimeout(timeout);
              isResolved = true;
              currentActiveProcess = null;
              
              const trimmedOutput = output.trim();
              if (code === 0) {
                // Some tools like create-vite exit with 0 even when cancelled/interrupted
                if (trimmedOutput.toLowerCase().includes('operation cancelled')) {
                  resolve(`Error: Operation cancelled\n${trimmedOutput}`);
                } else {
                  resolve(trimmedOutput || '(command completed with no output)');
                }
              } else {
                resolve(`Command exited with code ${code}:\n${trimmedOutput}`);
              }
            });

            child.on('error', (err) => {
              if (isResolved) return;
              clearTimeout(timeout);
              isResolved = true;
              currentActiveProcess = null;
              reject(err);
            });
          });

          if (abortRequested) return { result: '⚠️ Task stopped by user.', abort: true };

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
        // Request approval if not in autopilot mode
        if (!isAutopilotMode) {
          win?.webContents.send('agent:step', {
            tool: 'delete_file',
            status: 'awaiting_permission',
            summary: `Delete file: ${toolData.path}`,
            iteration: iteration
          });

          const decision = await new Promise<{ approved: boolean }>(resolve => {
            pendingPermissionResolver = resolve;
          });
          pendingPermissionResolver = null;

          if (!decision.approved) {
            return { result: '❌ File deletion denied by user.', abort: true };
          }
        }

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
          return {
            result: `✅ Successfully applied diffs to ${result.appliedCount} files.`,
            data: { changes: toolData.changes }
          };
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

      case 'readCode': {
        if (!toolData.path) return { result: '❌ Error: Tool "readCode" requires a "path" parameter.' };
        const resolvedPath = resolvePath(toolData.path, workspacePath);
        const isBinary = await isBinaryFile(resolvedPath);
        if (isBinary) return { result: `❌ Cannot read ${toolData.path}: This appears to be a binary file.` };

        try {
          const content = await fs.readFile(resolvedPath, 'utf-8');
          // Parse AST to extract structure
          const lines = content.split('\n');
          const structure: { type: string; name: string; startLine: number; endLine: number }[] = [];

          // Simple AST-like extraction for common patterns
          const classRegex = /^export?\s*class\s+(\w+)/m;
          const funcRegex = /^export?\s*(async\s+)?function\s+(\w+)/m;
          const arrowRegex = /^export?\s*const\s+(\w+)\s*=\s*(async\s+)?\(/m;

          for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const classMatch = line.match(classRegex);
            if (classMatch) structure.push({ type: 'class', name: classMatch[1], startLine: i + 1, endLine: i + 1 });

            const funcMatch = line.match(funcRegex);
            if (funcMatch) structure.push({ type: 'function', name: funcMatch[2], startLine: i + 1, endLine: i + 1 });

            const arrowMatch = line.match(arrowRegex);
            if (arrowMatch) structure.push({ type: 'arrow', name: arrowMatch[1], startLine: i + 1, endLine: i + 1 });
          }

          return {
            result: `File: ${toolData.path}\n\n${lines.map((line, i) => `${i + 1}: ${line}`).join('\n')}\n\n--- STRUCTURE ---\n${structure.map(s => `${s.type}: ${s.name} (lines ${s.startLine}-${s.endLine})`).join('\n')}`,
            data: { structure }
          };
        } catch (e: any) {
          return { result: `❌ Error reading file: ${e.message}` };
        }
      }

      case 'editCode': {
        if (!toolData.path) return { result: '❌ Error: Tool "editCode" requires a "path" parameter.' };
        if (!toolData.search) return { result: '❌ Error: Tool "editCode" requires a "search" parameter.' };
        if (!toolData.replace) return { result: '❌ Error: Tool "editCode" requires a "replace" parameter.' };

        const resolvedPath = resolvePath(toolData.path, workspacePath);
        const isBinary = await isBinaryFile(resolvedPath);
        if (isBinary) return { result: `❌ Cannot edit ${toolData.path}: This appears to be a binary file.` };

        try {
          let content = await fs.readFile(resolvedPath, 'utf-8');

          // Use AST-aware replacement if possible
          const searchRegex = new RegExp(`\\b${toolData.search}\\b`, 'g');
          if (searchRegex.test(content)) {
            const newContent = content.replace(searchRegex, toolData.replace);
            await fs.writeFile(resolvedPath, newContent, 'utf-8');
            return {
              result: `✅ Successfully replaced "${toolData.search}" with "${toolData.replace}" in ${toolData.path}`,
              data: { path: toolData.path, search: toolData.search, replace: toolData.replace }
            };
          }

          return { result: `❌ Could not find "${toolData.search}" in ${toolData.path}. Use readCode first to see the exact content.` };
        } catch (e: any) {
          return { result: `❌ Error editing file: ${e.message}` };
        }
      }

      case 'getDiagnostics': {
        if (!toolData.path) return { result: '❌ Error: Tool "getDiagnostics" requires a "path" parameter.' };
        return { result: await getDiagnostics(toolData.path, workspacePath) };
      }

      case 'grepSearch': {
        if (!toolData.pattern) return { result: '❌ Error: Tool "grepSearch" requires a "pattern" parameter.' };
        const searchRoot = workspacePath || '.';
        return { result: await grepSearch(searchRoot, toolData.pattern, toolData.include, toolData.maxResults || 50) };
      }

      case 'fileSearch': {
        if (!toolData.query) return { result: '❌ Error: Tool "fileSearch" requires a "query" parameter.' };
        const searchRoot = workspacePath || '.';
        return { result: await fuzzyFindFile(searchRoot, toolData.query, toolData.maxResults || 10) };
      }

      case 'readMultipleFiles': {
        if (!toolData.files || !Array.isArray(toolData.files) || toolData.files.length === 0) {
          return { result: '❌ Error: Tool "readMultipleFiles" requires a "files" array parameter.' };
        }
        return { result: await readMultipleFiles(toolData.files, workspacePath) };
      }

      case 'semanticRename': {
        if (!toolData.path) return { result: '❌ Error: Tool "semanticRename" requires a "path" parameter.' };
        if (!toolData.oldName) return { result: '❌ Error: Tool "semanticRename" requires an "oldName" parameter.' };
        if (!toolData.newName) return { result: '❌ Error: Tool "semanticRename" requires a "newName" parameter.' };
        return { result: await semanticRename(toolData.path, toolData.oldName, toolData.newName, workspacePath) };
      }

      case 'smartRelocate': {
        if (!toolData.sourcePath) return { result: '❌ Error: Tool "smartRelocate" requires a "sourcePath" parameter.' };
        if (!toolData.destinationPath) return { result: '❌ Error: Tool "smartRelocate" requires a "destinationPath" parameter.' };
        return { result: await smartRelocate(toolData.sourcePath, toolData.destinationPath, workspacePath) };
      }

      case 'strReplace': {
        if (!toolData.path) return { result: '❌ Error: Tool "strReplace" requires a "path" parameter.' };
        if (!toolData.oldStr) return { result: '❌ Error: Tool "strReplace" requires an "oldStr" parameter.' };
        if (!toolData.newStr) return { result: '❌ Error: Tool "strReplace" requires a "newStr" parameter.' };

        const resolvedPath = resolvePath(toolData.path, workspacePath);
        const isBinary = await isBinaryFile(resolvedPath);
        if (isBinary) return { result: `❌ Cannot replace in ${toolData.path}: This appears to be a binary file.` };

        try {
          let content = await fs.readFile(resolvedPath, 'utf-8');
          if (content.includes(toolData.oldStr)) {
            const newContent = content.replace(toolData.oldStr, toolData.newStr);
            await fs.writeFile(resolvedPath, newContent, 'utf-8');
            return {
              result: `✅ Successfully replaced in ${toolData.path}`,
              data: { path: toolData.path, oldStr: toolData.oldStr, newStr: toolData.newStr }
            };
          }
          return { result: `❌ Could not find the exact string in ${toolData.path}. Use read_file first.` };
        } catch (e: any) {
          return { result: `❌ Error: ${e.message}` };
        }
      }

      case 'invokeSubAgent': {
        if (!toolData.agentName) return { result: '❌ Error: Tool "invokeSubAgent" requires an "agentName" parameter.' };
        if (!toolData.task) return { result: '❌ Error: Tool "invokeSubAgent" requires a "task" parameter.' };

        const agentConfig = getSubAgentConfig(toolData.agentName);
        if (!agentConfig) {
          const available = listSubAgents().map(a => a.name).join(', ');
          return { result: `❌ Unknown sub-agent: "${toolData.agentName}". Available: ${available}` };
        }

        console.log(`\n[SUB-AGENT] Invoking ${toolData.agentName} for task: ${toolData.task.substring(0, 100)}...`);

        // Run the sub-agent with its own system prompt and iteration limit
        const subAgentResult = await runSubAgent(
          toolData.task,
          agentConfig,
          toolModel || { provider: 'ollama', model: 'qwen2.5-coder:latest' },
          config || { openaiKey: '', geminiKey: '' },
          workspacePath,
          isAutopilotMode
        );

        return { result: subAgentResult.finalResponse };
      }

      case 'listSubAgents': {
        const agents = listSubAgents();
        const agentList = agents.map(a => `- ${a.name}: ${a.description}`).join('\n');
        return { result: `Available sub-agents:\n${agentList}` };
      }

      // ====== TIER 1: WEB TOOLS ======

      case 'webFetch': {
        if (!toolData.url) return { result: '❌ Error: Tool "webFetch" requires a "url" parameter.' };
        try {
          const controller = new AbortController();
          const timeout = setTimeout(() => controller.abort(), 15000);
          const res = await fetch(toolData.url, {
            headers: { 'User-Agent': 'Mozilla/5.0 WhizCode/1.0' },
            signal: controller.signal
          });
          clearTimeout(timeout);
          const html = await res.text();
          // Strip HTML tags, collapse whitespace, limit size
          const text = html
            .replace(/<style[\s\S]*?<\/style>/gi, '')
            .replace(/<script[\s\S]*?<\/script>/gi, '')
            .replace(/<[^>]+>/g, ' ')
            .replace(/\s+/g, ' ')
            .trim()
            .slice(0, 24000);
          return { result: `📄 Content from ${toolData.url}:\n\n${text}` };
        } catch (e: any) {
          return { result: `❌ webFetch failed: ${e.message}` };
        }
      }

      case 'web_search': {
        if (!toolData.query) return { result: '❌ Error: Tool "web_search" requires a "query" parameter.' };
        try {
          // DuckDuckGo HTML search — no API key required
          const searchUrl = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(toolData.query)}`;
          const controller = new AbortController();
          const timeout = setTimeout(() => controller.abort(), 15000);
          const res = await fetch(searchUrl, {
            headers: { 'User-Agent': 'Mozilla/5.0 WhizCode/1.0' },
            signal: controller.signal
          });
          clearTimeout(timeout);
          const html = await res.text();
          // Extract result snippets from DuckDuckGo HTML
          const resultRegex = /<a class="result__a"[^>]+href="([^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]*?<a class="result__snippet"[^>]*>([\s\S]*?)<\/a>/g;
          const results: string[] = [];
          let m;
          let count = 0;
          while ((m = resultRegex.exec(html)) !== null && count < 8) {
            const url = m[1].replace(/\/\/duckduckgo\.com\/l\/\?uddg=/, '').split('&')[0];
            const title = m[2].replace(/<[^>]+>/g, '').trim();
            const snippet = m[3].replace(/<[^>]+>/g, '').trim();
            results.push(`**${title}**\n${decodeURIComponent(url)}\n${snippet}`);
            count++;
          }
          if (results.length === 0) {
            // Fallback: just strip HTML
            const text = html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 8000);
            return { result: `🔍 Search results for "${toolData.query}":\n\n${text}` };
          }
          return { result: `🔍 Search results for "${toolData.query}":\n\n${results.join('\n\n')}` };
        } catch (e: any) {
          return { result: `❌ web_search failed: ${e.message}` };
        }
      }

      // ====== TIER 1: MCP TOOL CALL ======

      case 'mcp_call': {
        if (!toolData.toolName) return { result: '❌ Error: Tool "mcp_call" requires a "toolName" parameter.' };
        if (!mcpManager) return { result: '❌ MCP manager not initialized. Open a workspace first.' };
        const connectedServers = mcpManager.getConnectedServers();
        if (connectedServers.length === 0) {
          return { result: '❌ No MCP servers connected. Create .kiro/mcp-servers.json to configure servers.' };
        }
        try {
          const result = await mcpManager.callTool(toolData.toolName, toolData.args || {});
          return { result: `✅ MCP [${toolData.toolName}]:\n${result}` };
        } catch (e: any) {
          return { result: `❌ MCP tool "${toolData.toolName}" failed: ${e.message}` };
        }
      }

      // ====== TIER 1: SPECS SYSTEM ======

      case 'createSpec': {
        if (!toolData.name) return { result: '❌ Error: Tool "createSpec" requires a "name" parameter.' };
        if (!specsManager) return { result: '❌ Specs manager not initialized. Open a workspace first.' };
        try {
          const spec = await specsManager.createSpec(toolData.name, toolData.requirements || '');
          return { result: `✅ Created spec "${spec.name}" (slug: ${spec.slug})\nPath: ${spec.path}\n\nThe spec has 3 documents:\n- requirements.md — What to build\n- design.md — How to build it\n- tasks.md — Implementation checklist\n\nEdit these files or use updateSpec to fill them in.` };
        } catch (e: any) {
          return { result: `❌ createSpec failed: ${e.message}` };
        }
      }

      case 'readSpec': {
        if (!toolData.slug) return { result: '❌ Error: Tool "readSpec" requires a "slug" parameter.' };
        if (!specsManager) return { result: '❌ Specs manager not initialized.' };
        const spec = await specsManager.getSpec(toolData.slug);
        if (!spec) return { result: `❌ Spec "${toolData.slug}" not found. Use listSpecs to see available specs.` };
        return { result: specsManager.buildSpecContext(spec) };
      }

      case 'updateSpec': {
        if (!toolData.slug) return { result: '❌ Error: Tool "updateSpec" requires a "slug" parameter.' };
        if (!toolData.docType) return { result: '❌ Error: Tool "updateSpec" requires a "docType" parameter (requirements|design|tasks).' };
        if (!toolData.content) return { result: '❌ Error: Tool "updateSpec" requires a "content" parameter.' };
        if (!specsManager) return { result: '❌ Specs manager not initialized.' };
        const ok = await specsManager.updateSpecDocument(toolData.slug, toolData.docType, toolData.content);
        return { result: ok ? `✅ Updated ${toolData.docType}.md for spec "${toolData.slug}"` : `❌ Failed to update spec "${toolData.slug}"` };
      }

      case 'listSpecs': {
        if (!specsManager) return { result: '❌ Specs manager not initialized.' };
        const specs = await specsManager.listSpecs();
        if (specs.length === 0) return { result: 'No specs found. Use createSpec to create one.' };
        const list = specs.map(s =>
          `- **${s.name}** (${s.slug}) — ${s.completedTasks}/${s.totalTasks} tasks (${s.progress}%)`
        ).join('\n');
        return { result: `📋 Specs:\n${list}` };
      }

      case 'completeTask': {
        if (!toolData.slug) return { result: '❌ Error: Tool "completeTask" requires a "slug" parameter.' };
        if (!toolData.taskDescription) return { result: '❌ Error: Tool "completeTask" requires a "taskDescription" parameter.' };
        if (!specsManager) return { result: '❌ Specs manager not initialized.' };
        const res = await specsManager.completeTask(toolData.slug, toolData.taskDescription);
        return { result: res.message };
      }

      case 'learn_fact': {
        if (!toolData.topic) return { result: '❌ Error: Tool "learn_fact" requires a "topic" parameter.' };
        if (!toolData.content) return { result: '❌ Error: Tool "learn_fact" requires a "content" parameter.' };
        if (!memoryManager) return { result: '❌ Memory manager not initialized.' };
        const ok = await memoryManager.learnFact(toolData.topic, toolData.content);
        return { result: ok ? `✅ Learned fact and saved to memory: ${toolData.topic}` : `❌ Failed to save to memory` };
      }

      default:
        return { result: `❌ Unknown tool: "${toolData.tool}". Available tools: semantic_search, apply_diffs, validate_project, run_tests, get_blast_radius, read_file, replace_lines, insert_code, write_file, edit_file, list_directory, search_files, run_command, readCode, editCode, getDiagnostics, grepSearch, fileSearch, readMultipleFiles, semanticRename, smartRelocate, strReplace, invokeSubAgent, listSubAgents, webFetch, web_search, mcp_call, createSpec, readSpec, updateSpec, listSpecs, completeTask` };
    }

    // Unreachable — all cases return above; this satisfies the compiler
    return { result: '❌ Internal error: unhandled tool case' };
  } catch (e: any) {
    return { result: `❌ Tool error (${toolData.tool}): ${e.message}` };
  } finally {
    // Fire postToolUse hooks (best-effort, after tool result is already determined)
    if (hooksManager) {
      try {
        const postHooks = await hooksManager.triggerToolEvent('postToolUse', toolData.tool);
        for (const hook of postHooks) {
          if (hook.action === 'runCommand' && hook.command) {
            console.log(`[HOOK] postToolUse → runCommand: ${hook.command}`);
            await execAsync(hook.command, { cwd: workspacePath || '.', timeout: (hook.timeout || 30) * 1000 }).catch(e => {
              console.warn(`[HOOK] postToolUse command failed: ${e.message}`);
            });
          } else if (hook.action === 'askAgent' && hook.prompt) {
            console.log(`[HOOK] postToolUse → askAgent: ${hook.prompt}`);
            // Prevent infinite recursion by not allowing hooks during hook execution
            // (Simplified version: assume one level deep is okay for now)
            const fallbackModel = { provider: 'ollama', model: 'qwen2.5-coder:latest' };
            await runAgentLoop(
              `[AUTOMATED HOOK: ${hook.id}]\n${hook.prompt}`,
              toolModel || fallbackModel, // Use tool model for hooks for efficiency
              toolModel || fallbackModel,
              config,
              workspacePath,
              null,
              isAutopilotMode
            ).catch(e => console.warn(`[HOOK] askAgent failed: ${e.message}`));
          }
        }
      } catch (e) {
        console.warn('[HOOK] postToolUse error:', e);
      }
    }
  }
}

// ====== AGENT LOOP ======

const MAX_AGENT_ITERATIONS = 20;

// Sub-agent execution function
async function runSubAgent(
  task: string,
  agentConfig: any,
  model: { provider: string, model: string },
  config: any,
  workspacePath: string | null,
  isAutopilotMode: boolean = false
): Promise<{ finalResponse: string; steps: any[] }> {
  const steps: any[] = [];
  const subAgentMessages = [
    { role: 'system', content: agentConfig.systemPrompt },
    { role: 'user', content: task }
  ];

  const maxIterations = agentConfig.maxIterations || 10;
  let previousToolCallStr = '';
  let repeatCount = 0;

  for (let iteration = 0; iteration < maxIterations; iteration++) {
    if (abortRequested) {
      return { finalResponse: '⚠️ Sub-agent cancelled by user.', steps };
    }

    console.log(`[SUB-AGENT ${agentConfig.name}] Iteration ${iteration + 1}/${maxIterations}`);

    const aiResponse = await callAI(subAgentMessages, model, config);
    const toolCall = tryParseToolCall(aiResponse);

    if (!toolCall) {
      // Sub-agent finished
      return { finalResponse: aiResponse, steps };
    }

    // Detect repetition
    const currentToolCallStr = JSON.stringify({ tool: toolCall.tool, path: toolCall.path, command: toolCall.command });
    if (currentToolCallStr === previousToolCallStr) {
      repeatCount++;
      if (repeatCount >= 2) {
        return { finalResponse: `⚠️ Sub-agent got stuck in a loop repeating the same tool call. Task aborted.`, steps };
      }
      subAgentMessages.push({ role: 'assistant', content: aiResponse });
      subAgentMessages.push({ role: 'user', content: 'You repeated the same tool call. Try a different approach or provide your final response.' });
      continue;
    } else {
      repeatCount = 0;
      previousToolCallStr = currentToolCallStr;
    }

    // Execute tool
    const toolName = toolCall.tool;
    const stepData: any = { tool: toolName, status: 'running', summary: getToolSummary(toolCall) };
    steps.push(stepData);

    subAgentMessages.push({ role: 'assistant', content: aiResponse });

    // Don't allow sub-agents to invoke other sub-agents (prevent recursion)
    if (toolName === 'invokeSubAgent') {
      subAgentMessages.push({ role: 'user', content: '❌ Sub-agents cannot invoke other sub-agents. Complete the task yourself or provide your findings.' });
      continue;
    }

    const { result: toolResult } = await executeToolCall(toolCall, workspacePath, iteration, isAutopilotMode, model, config);

    const truncatedResult = toolResult.length > 10000
      ? toolResult.substring(0, 10000) + '\n... (truncated)'
      : toolResult;

    stepData.status = 'done';
    stepData.result = truncatedResult.substring(0, 500);

    subAgentMessages.push({
      role: 'user',
      content: `[Tool Result: ${toolName}]\n${truncatedResult}\n\n[Continue or provide final response]`
    });
  }

  return {
    finalResponse: `Sub-agent ${agentConfig.name} reached max iterations (${maxIterations}). Task may be incomplete.`,
    steps
  };
}

async function runAgentLoop(
  userMessage: string,
  primaryModel: { provider: string, model: string },
  toolModel: { provider: string, model: string },
  config: any,
  workspacePath: string | null,
  activeContext: { path: string, content: string } | null = null,
  isAutopilotMode: boolean = false
): Promise<{ finalResponse: string; steps: any[] }> {
  const steps: any[] = [];
  abortRequested = false;

  // Persist user message to history immediately
  conversationHistory.push({ role: 'user', content: userMessage });

  // Build enhanced project context
  let projectContext = `\n\n<project_context>\n`;

  // #File and #Folder injection
  const fileRefs = userMessage.match(/#File:([^\s]+)/g);
  if (fileRefs && workspacePath) {
    for (const ref of fileRefs) {
      const fileName = ref.replace('#File:', '');
      try {
        const fullPath = join(workspacePath, fileName);
        const content = await fs.readFile(fullPath, 'utf-8');
        projectContext += `<injected_file path="${fileName}">\n${content}\n</injected_file>\n`;
      } catch (e) {
        projectContext += `<error>Could not inject #File:${fileName}: file not found or inaccessible</error>\n`;
      }
    }
  }

  const folderRefs = userMessage.match(/#Folder:([^\s]+)/g);
  if (folderRefs && workspacePath) {
    for (const ref of folderRefs) {
      const folderName = ref.replace('#Folder:', '');
      try {
        const fullPath = join(workspacePath, folderName);
        const files = await readDirectoryRecursive(fullPath, 50);
        projectContext += `<injected_folder path="${folderName}">\n`;
        for (const f of files) {
          try {
            const content = await fs.readFile(f.path, 'utf-8');
            projectContext += `<file path="${f.path.replace(workspacePath, '')}">\n${content}\n</file>\n`;
          } catch {}
        }
        projectContext += `</injected_folder>\n`;
      } catch (e) {
        projectContext += `<error>Could not inject #Folder:${folderName}: folder not found or inaccessible</error>\n`;
      }
    }
  }

  // ── Context Window Guard ─────────────────────────────────────────────
  // Ollama models have limited context windows (8k-32k tokens).
  // Cap manifest lines and active file size to prevent overflow.
  const MAX_MANIFEST_LINES = 200;
  const MAX_ACTIVE_FILE_LINES = 300;
  const MAX_GIT_DIFF_CHARS = 3000;

  // Workspace and file tree
  if (workspaceManifest) {
    const manifestLines = workspaceManifest.split('\n');
    const cappedManifest = manifestLines.length > MAX_MANIFEST_LINES
      ? manifestLines.slice(0, MAX_MANIFEST_LINES).join('\n') + `\n... (${manifestLines.length - MAX_MANIFEST_LINES} more files — use list_directory to explore)`
      : workspaceManifest;
    projectContext += `<workspace_root>${workspacePath}</workspace_root>\n\n`;
    projectContext += `<file_tree>\n${cappedManifest}\n</file_tree>\n`;
  } else {
    projectContext += `<workspace_root>${workspacePath || 'No workspace open'}</workspace_root>\n`;
    projectContext += `<note>Project not indexed yet. Use list_directory to explore.</note>\n`;
  }

  // Active editor file (capped to avoid flooding context)
  if (activeContext) {
    const fileLines = activeContext.content.split('\n');
    const displayedContent = fileLines.length > MAX_ACTIVE_FILE_LINES
      ? fileLines.slice(0, MAX_ACTIVE_FILE_LINES).map((l, i) => `${i + 1}: ${l}`).join('\n') +
        `\n... (${fileLines.length - MAX_ACTIVE_FILE_LINES} more lines — use read_file to see full content)`
      : fileLines.map((l, i) => `${i + 1}: ${l}`).join('\n');
    projectContext += `\n<active_editor_file>\n<path>${activeContext.path}</path>\n<content>\n${displayedContent}\n</content>\n</active_editor_file>\n`;
  }

  // Add git diff if available (capped)
  if (workspacePath) {
    try {
      const { stdout: gitDiff } = await execAsync('git diff HEAD', {
        cwd: workspacePath,
        maxBuffer: 1024 * 50 // 50KB max
      });
      if (gitDiff && gitDiff.trim().length > 0) {
        const truncatedDiff = gitDiff.length > MAX_GIT_DIFF_CHARS
          ? gitDiff.substring(0, MAX_GIT_DIFF_CHARS) + '\n... (truncated)'
          : gitDiff;
        projectContext += `\n<git_diff>\n${truncatedDiff}\n</git_diff>\n`;
      }
    } catch (e) {
      // Silently skip if git is not available or no changes
    }
  }

  // Add terminal output if available
  if (terminalOutputBuffer.length > 0) {
    const lastLines = terminalOutputBuffer.slice(-50).join('\n');
    projectContext += `\n<terminal_output>\n${lastLines}\n</terminal_output>\n`;
  }

  projectContext += `</project_context>\n`;

  // Add steering instructions
  if (steeringManager) {
    const steeringContext = await steeringManager.buildSteeringContext(activeContext?.path);
    if (steeringContext) {
      projectContext += steeringContext;
    }
  }

  // Add specs summary
  if (specsManager) {
    const specsSummary = await specsManager.getSpecsSummaryText();
    if (specsSummary) projectContext += specsSummary;
  }

  // Add MCP tool list
  if (mcpManager) {
    const mcpPrompt = mcpManager.buildToolPrompt();
    if (mcpPrompt) projectContext += mcpPrompt;
  }

  // Add memory context
  if (memoryManager) {
    const memoryContext = await memoryManager.buildMemoryContext();
    if (memoryContext) projectContext += memoryContext;
  }

  // ── Sliding Window History ───────────────────────────────────────────
  // Keep only the last 10 conversation turns (20 messages = 10 user+assistant pairs)
  // to prevent context window overflow on long sessions.
  const MAX_HISTORY_MESSAGES = 20;
  const recentHistory = conversationHistory.length > MAX_HISTORY_MESSAGES + 1
    ? conversationHistory.slice(-(MAX_HISTORY_MESSAGES + 1))
    : conversationHistory;

  // Initialize conversation with system prompt and context
  const dynamicPrompt = KIRO_SYSTEM_PROMPT.replace(
    '</system_context>',
    `Current Workspace: ${workspacePath}\n</system_context>`
  );
  const currentMessages = [
    { role: 'system', content: dynamicPrompt },
    ...recentHistory.slice(0, -1), // All recent history except the last user message
    { role: 'user', content: `${userMessage}${projectContext}` }
  ];

  let previousToolCallStr = '';
  let toolHistory: string[] = [];
  let repeatCount = 0;
  let consecutiveThinkingCount = 0;

  for (let iteration = 0; iteration < MAX_AGENT_ITERATIONS; iteration++) {
    if (abortRequested) {
      console.log("[ABORT] Agent loop stopped by user.");
      const abortMsg = "⚠️ Task cancelled by user.";
      conversationHistory.push({ role: 'assistant', content: abortMsg });
      return { finalResponse: abortMsg, steps };
    }

    console.log(`\n[ITERATION ${iteration + 1}/${MAX_AGENT_ITERATIONS}]`);

    // Decide which model to use:
    // - Iteration 0: primary model (reasoning/planning — figures out what to do first)
    // - Iteration 1+: tool model (execution — keeps calling tools until done)
    // NOTE: Previously used toolHistory.length check which was WRONG — it would skip
    // the tool model on iter 1 if iter 0 only produced a <THOUGHT>.
    const useToolModel = iteration > 0;
    const selectedModel = useToolModel ? toolModel : primaryModel;

    console.log(`[MODEL] Using ${selectedModel.provider}/${selectedModel.model} (${useToolModel ? 'tool' : 'primary'} model)`);

    // Call AI
    const aiResponse = await callAI(
      currentMessages,
      selectedModel,
      config,
      agentAbortController?.signal,
      config.temperature || 0.1
    );
    const toolCalls = tryParseAllToolCalls(aiResponse);

    if (toolCalls.length === 0) {
      // Check if this is just thinking/reasoning without action
      if (aiResponse.includes('<THOUGHT>') && aiResponse.length < 500 && !aiResponse.includes('```')) {
        consecutiveThinkingCount++;
        if (consecutiveThinkingCount >= 2) {
          // Agent is stuck in thinking loop, push it to act
          console.log("[NUDGE] Agent thinking too much, pushing to action...");
          currentMessages.push({ role: 'assistant', content: aiResponse });
          currentMessages.push({ role: 'user', content: 'Take action now. Output a JSON tool call or provide your final response.' });
          consecutiveThinkingCount = 0;
          continue;
        }
        currentMessages.push({ role: 'assistant', content: aiResponse });
        continue;
      }

      consecutiveThinkingCount = 0;

      // Improved stalling detection
      const hasCodeBlock = aiResponse.includes('```');
      const hasInstructionalPhrases =
        aiResponse.toLowerCase().includes('you should run') ||
        aiResponse.toLowerCase().includes('you need to run') ||
        aiResponse.toLowerCase().includes('please run') ||
        aiResponse.toLowerCase().includes('next, run') ||
        aiResponse.toLowerCase().includes('then run') ||
        aiResponse.toLowerCase().includes('step-by-step') ||
        aiResponse.toLowerCase().includes('here is the code') ||
        aiResponse.toLowerCase().includes('manually');

      const isTalkingInsteadOfActing = (hasCodeBlock && !aiResponse.includes('"tool"')) || hasInstructionalPhrases;
      const looksLikeCompletion = aiResponse.toUpperCase().includes('TASK COMPLETE') || (aiResponse.length < 300 && iteration > 1);
      const containsJsonButFailed = (aiResponse.includes('{') && (aiResponse.includes('"tool"') || aiResponse.includes('"tool_calls"')));

      // Nudge if talking instead of acting OR if JSON parsing failed but intent was clear
      if (containsJsonButFailed) {
        console.log("[NUDGE] AI intent was clear (found JSON keywords) but all tool call parsing attempts failed.");
        currentMessages.push({ role: 'assistant', content: aiResponse });
        currentMessages.push({ role: 'user', content: 'ATTENTION: You provided tool-calling JSON but it could not be processed properly (likely due to unescaped characters or syntax errors). Please output ONLY the valid JSON tool call for the NEXT step, ensuring all quotes and backslashes are properly escaped.' });
        continue;
      }

      if (isTalkingInsteadOfActing && !looksLikeCompletion) {
        console.log("[NUDGE] Agent providing instructions instead of using tools");
        currentMessages.push({ role: 'assistant', content: aiResponse });
        currentMessages.push({ role: 'user', content: 'ACTION REQUIRED: Do NOT explain or provide manual steps. You are an autonomous agent. Use your tools (write_file, edit_file, run_command) to execute the solution directly. Output a JSON tool call NOW.' });
        continue;
      }

      // This is the final response
      conversationHistory.push({ role: 'assistant', content: aiResponse });
      
      // Save to chat history
      if (currentHistoryManager) {
        const title = conversationHistory.find(m => m.role === 'user')?.content.substring(0, 40) || 'Untitled Chat';
        await currentHistoryManager.saveThread(currentConversationId, title, conversationHistory);
      }

      return { finalResponse: aiResponse, steps };
    }

    // Standardize iterative thinking
    consecutiveThinkingCount = 0;
    
    // Process all tools found in the message
    let turnResults: string[] = [];
    let shouldAbort = false;
    let finalMsg = '';

    for (const toolCall of toolCalls) {
      if (abortRequested) {
        shouldAbort = true;
        finalMsg = 'Task stopped by user.';
        break;
      }

      const currentToolCallStr = JSON.stringify({ tool: toolCall.tool, path: toolCall.path, command: toolCall.command });

      // Detect direct repetition
      if (currentToolCallStr === previousToolCallStr) {
        repeatCount++;
        if (repeatCount >= 3) {
          finalMsg = '⚠️ Agent got stuck in a repetitive loop. Task aborted.';
          shouldAbort = true;
          break;
        }
        turnResults.push('[SYSTEM] You repeated the same tool call. Change your strategy.');
        continue;
      } else {
        repeatCount = 0;
        previousToolCallStr = currentToolCallStr;
      }

      // Detect ping-pong loops
      toolHistory.push(currentToolCallStr);
      if (toolHistory.length > 4) toolHistory.shift();

      const isPingPong = toolHistory.length === 4 &&
        toolHistory[0] === toolHistory[2] &&
        toolHistory[1] === toolHistory[3];

      if (isPingPong) {
        turnResults.push(`[SYSTEM] Loop detected. Strategy change required.`);
        toolHistory = [];
        continue;
      }

      // Execute the tool
      const toolName = toolCall.tool;
      const toolSummary = getToolSummary(toolCall);
      const stepData = { tool: toolName, status: 'running', summary: toolSummary, iteration: iteration + 1 };
      
      win?.webContents.send('agent:step', stepData);
      const stepIndex = steps.push(stepData) - 1;

      console.log(`[LOOP] Executing: ${toolName}`);
      const execution = await executeToolCall(toolCall, workspacePath, iteration + 1, isAutopilotMode, toolModel, config);

      const truncatedResult = execution.result.length > 15000
        ? execution.result.substring(0, 15000) + '\n... (truncated)'
        : execution.result;

      steps[stepIndex].status = 'done';
      steps[stepIndex].result = truncatedResult.substring(0, 500);
      if (execution.logs) steps[stepIndex].logs = execution.logs;
      if (execution.data) steps[stepIndex].data = execution.data;
      win?.webContents.send('agent:step', { ...steps[stepIndex], status: 'done' });

      // Analyze errors and provide guidance
      let enhancedResult = truncatedResult;
      if (toolName === 'run_command' && (truncatedResult.toLowerCase().includes('operation cancelled') || truncatedResult.includes('Error:') || truncatedResult.includes('ENOENT'))) {
        enhancedResult = analyzeCommandError(truncatedResult, toolCall.command || '');
      }
      
      turnResults.push(`[${toolName} Result]\n${enhancedResult}`);

      if (execution.abort) {
        shouldAbort = true;
        finalMsg = `Task stopped: User denied operation or requested abort during ${toolName}.`;
        break;
      }
    }

    currentMessages.push({ role: 'assistant', content: aiResponse });
    currentMessages.push({ role: 'user', content: turnResults.join('\n\n') + '\n\n[Identify next actions or provide final response]' });

    if (shouldAbort) {
      if (!finalMsg) finalMsg = 'Task stopped.';
      conversationHistory.push({ role: 'assistant', content: finalMsg });
      if (currentHistoryManager) {
        const title = conversationHistory.find(m => m.role === 'user')?.content.substring(0, 40) || 'Untitled Chat';
        await currentHistoryManager.saveThread(currentConversationId, title, conversationHistory);
      }
      return { finalResponse: finalMsg, steps };
    }
  }

  // Max iterations reached
  const finalMsg = `Reached maximum iterations (${MAX_AGENT_ITERATIONS}). Task may be incomplete.`;
  conversationHistory.push({ role: 'assistant', content: finalMsg });
  if (currentHistoryManager) {
    const title = conversationHistory.find(m => m.role === 'user')?.content.substring(0, 40) || 'Untitled Chat';
    await currentHistoryManager.saveThread(currentConversationId, title, conversationHistory);
  }
  return { finalResponse: finalMsg, steps };
}

async function refreshManifest(workspacePath: string) {
  const files = await readDirectoryRecursive(workspacePath, 3000);
  if (files.length > 0) {
    let manifest = `## PROJECT MANIFEST\n\n### Root: ${workspacePath}\n\n#### Directory Structure (File List):\n`;
    manifest += files.map(f => `- ${f.path.replace(workspacePath, '').replace(/^[\\/]/, '')}`).join('\n');
    manifest += '\n\n#### Critical Metadata:\n(Use read_file to access full contents)\n';
    workspaceManifest = manifest;
  }
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
    case 'readCode': return `Reading code structure from ${path}`;
    case 'editCode': return `Editing code in ${path}`;
    case 'getDiagnostics': return `Getting diagnostics for ${path}`;
    case 'grepSearch': return `Grep searching for "${toolCall.pattern || '?'}"${toolCall.include ? ` in ${toolCall.include}` : ''}`;
    case 'fileSearch': return `Fuzzy finding "${toolCall.query || '?'}"`;
    case 'readMultipleFiles': return `Reading ${toolCall.files?.length || 0} files`;
    case 'semanticRename': return `Renaming "${toolCall.oldName}" to "${toolCall.newName}"`;
    case 'smartRelocate': return `Moving ${toolCall.sourcePath} to ${toolCall.destinationPath}`;
    case 'strReplace': return `Replacing string in ${path}`;
    case 'invokeSubAgent': return `Delegating to ${toolCall.agentName}: ${toolCall.task?.substring(0, 50) || ''}...`;
    case 'listSubAgents': return `Listing available sub-agents`;
    // Tier 1 tools
    case 'webFetch': return `Fetching URL: ${toolCall.url}`;
    case 'web_search': return `Web search: "${toolCall.query}"`;
    case 'mcp_call': return `MCP tool: ${toolCall.toolName}`;
    case 'createSpec': return `Creating spec: ${toolCall.name}`;
    case 'readSpec': return `Reading spec: ${toolCall.slug}`;
    case 'updateSpec': return `Updating spec ${toolCall.slug}/${toolCall.docType}`;
    case 'listSpecs': return `Listing all specs`;
    case 'completeTask': return `Completing task in ${toolCall.slug}: ${toolCall.taskDescription?.substring(0, 40)}`;
    case 'learn_fact': return `Learning fact about: ${toolCall.topic}`;
    default: return toolCall.tool;
  }
}

// ------ WORKSPACE FILE WATCHER ------
function setupWorkspaceWatcher(watchPath: string) {
  if (workspaceWatcher) {
    workspaceWatcher.close();
    workspaceWatcher = null;
  }

  // Initialize hooks manager for this workspace
  hooksManager = new HooksManager(watchPath);
  hooksManager.initialize().catch(console.error);

  // Initialize steering manager for this workspace
  steeringManager = new SteeringManager(watchPath);
  steeringManager.initialize().catch(console.error);

  // Initialize specs manager
  specsManager = new SpecsManager(watchPath);
  specsManager.initialize().catch(console.error);

  // Initialize MCP manager (connects to servers defined in .kiro/mcp-servers.json)
  if (mcpManager) mcpManager.shutdown();
  mcpManager = new MCPManager(watchPath);
  mcpManager.initialize().catch(console.error);

  // Initialize memory manager
  memoryManager = new MemoryManager(watchPath);
  memoryManager.initialize().catch(console.error);

  // Initialize history manager
  currentHistoryManager = new HistoryManager(watchPath);
  currentHistoryManager.initialize().catch(console.error);
  // Reset conversation ID for new workspace or session
  currentConversationId = Date.now().toString();

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

  const notifyRenderer = async (type: string, filePath: string) => {
    win?.webContents.send('fs:directoryChanged', { type, filePath });

    // Trigger file hooks
    if (hooksManager) {
      let eventType: 'fileEdited' | 'fileCreated' | 'fileDeleted' | null = null;
      if (type === 'add') eventType = 'fileCreated';
      else if (type === 'unlink') eventType = 'fileDeleted';
      else if (type === 'change') eventType = 'fileEdited';

      if (eventType) {
        const triggeredHooks = await hooksManager.triggerFileEvent(eventType, filePath);
        if (triggeredHooks.length > 0) {
          console.log(`[HOOKS] Triggered ${triggeredHooks.length} hooks for ${eventType} on ${filePath}`);
          for (const hook of triggeredHooks) {
            if (hook.action === 'runCommand' && hook.command) {
              await execAsync(hook.command.replace('${filePath}', filePath), { cwd: watchPath }).catch(e => {
                console.warn(`[HOOK] File hook command failed: ${e.message}`);
              });
            } else if (hook.action === 'askAgent' && hook.prompt) {
              const fallbackModel = { provider: 'ollama', model: 'qwen2.5-coder:latest' };
              await runAgentLoop(
                `[AUTOMATED FILE HOOK: ${hook.id}]\nFile: ${filePath}\nEvent: ${eventType}\n\n${hook.prompt.replace('${filePath}', filePath)}`,
                fallbackModel, // Use defaults for background hooks
                fallbackModel,
                { openaiKey: '', geminiKey: '' },
                watchPath,
                null,
                false
              ).catch(e => console.warn(`[HOOK] askAgent failed: ${e.message}`));
            }
          }
        }
      }
    }

    // Refresh manifest on file changes to keep agent context up-to-date
    if (currentWorkspacePath) {
      refreshManifest(currentWorkspacePath).catch(console.error);
    }
  };

  workspaceWatcher.on('add', (path: string) => notifyRenderer('add', path));
  workspaceWatcher.on('change', (path: string) => notifyRenderer('change', path));
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
  // Start watching the selected folder and update current workspace
  if (!result.canceled && result.filePaths?.length > 0) {
    currentWorkspacePath = result.filePaths[0];
    setupWorkspaceWatcher(result.filePaths[0]);
    await saveLastWorkspace(result.filePaths[0]);
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

// ------ SPEC IPC HANDLERS ------
ipcMain.handle('specs:list', async () => {
  if (!specsManager) return [];
  return await specsManager.listSpecs();
});

ipcMain.handle('specs:get', async (_event, slug: string) => {
  if (!specsManager) return null;
  return await specsManager.getSpec(slug);
});

ipcMain.handle('specs:create', async (_event, name: string, requirements?: string) => {
  if (!specsManager) return { error: 'No workspace open' };
  return await specsManager.createSpec(name, requirements);
});

ipcMain.handle('specs:update', async (_event, slug: string, docType: string, content: string) => {
  if (!specsManager) return false;
  return await specsManager.updateSpecDocument(slug, docType as any, content);
});

ipcMain.handle('specs:completeTask', async (_event, slug: string, taskDescription: string) => {
  if (!specsManager) return { success: false, message: 'No workspace open' };
  return await specsManager.completeTask(slug, taskDescription);
});

ipcMain.handle('specs:delete', async (_event, slug: string) => {
  if (!specsManager) return false;
  return await specsManager.deleteSpec(slug);
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

ipcMain.handle('agent:permission-response', async (_event, decision) => {
  console.log(`[IPC] agent:permission-response:`, decision);
  if (pendingPermissionResolver) {
    pendingPermissionResolver(decision);
    pendingPermissionResolver = null;
    return { success: true };
  }
  console.warn('[IPC] No pending permission resolver found!');
  return { success: false, error: 'No pending permission' };
});

ipcMain.on('app:exit', () => app.quit());
ipcMain.handle('app:open-external', (_event, url) => shell.openExternal(url));

// ------ TERMINAL HANDLERS ------
ipcMain.on('terminal:spawn', (_event, terminalId = 'default') => {
  if (ptyProcess) return;

  // Use workspace path if available, otherwise use home directory
  const cwd = currentWorkspacePath || os.homedir();

  // Detect shell - prefer bash if available on Windows (Git Bash/WSL)
  let shell: string;
  if (os.platform() === 'win32') {
    // Try to use bash if available (Git Bash), otherwise PowerShell
    try {
      const { execSync } = require('child_process');
      execSync('bash --version', { stdio: 'ignore' });
      shell = 'bash.exe';
    } catch {
      shell = 'powershell.exe';
    }
  } else {
    shell = process.env.SHELL || 'bash';
  }

  console.log(`[TERMINAL] Spawning shell: ${shell} in ${cwd}`);

  ptyProcess = pty.spawn(shell, [], {
    name: 'xterm-color',
    cols: 80,
    rows: 30,
    cwd: cwd,
    env: process.env as any
  });

  ptyProcess.onData((data: string) => {
    win?.webContents.send('terminal:incomingData', data, terminalId);
    // Buffer terminal output for agent context (last 200 lines max)
    const lines = data.split(/\r?\n/);
    terminalOutputBuffer.push(...lines.filter(l => l.trim()));
    if (terminalOutputBuffer.length > 200) {
      terminalOutputBuffer.splice(0, terminalOutputBuffer.length - 200);
    }
  });

  ptyProcess.onExit(() => {
    console.log('[TERMINAL] Process exited');
    ptyProcess = null;
  });
});

ipcMain.on('terminal:keystroke', (_event, key, _terminalId = 'default') => {
  ptyProcess?.write(key);
});

ipcMain.on('terminal:resize', (_event, cols, rows, _terminalId = 'default') => {
  if (ptyProcess && cols > 0 && rows > 0) {
    try {
      ptyProcess.resize(cols, rows);
    } catch (e) { }
  }
});

ipcMain.handle('terminal:reset', () => {
  if (ptyProcess) {
    try {
      ptyProcess.kill();
    } catch (e) { }
    ptyProcess = null;
  }
  return true;
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

ipcMain.handle('execute-agent-task', async (_event, { task, primaryModel, toolModel, workspacePath, activeFile, config, isAutopilotMode }) => {
  if (!workspacePath) {
    return {
      response: "I'm ready to help, but I need you to open a folder first so I have a place to work. Please use the 'Open Folder' button in the Title Bar or File menu.",
      steps: []
    };
  }
  try {
    abortRequested = false;
    agentAbortController = new AbortController();

    // 1. Initial Workspace Scan or Path Change
    if (workspacePath && (workspacePath !== currentWorkspacePath || !workspaceContextLoaded)) {
      const isNewWorkspace = workspacePath !== currentWorkspacePath;
      currentWorkspacePath = workspacePath;
      workspaceContextLoaded = true;

      console.log('[INDEXING] Building project manifest:', workspacePath);
      win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'running', summary: `Indexing: ${workspacePath}` });

      // Start watching workspace for file changes
      setupWorkspaceWatcher(workspacePath);

      await refreshManifest(workspacePath);

      // Initialize or reset services
      if (isNewWorkspace || !graphService) {
        graphService = new CodeGraphService();
        await graphService.initialize(workspacePath);
      }
      if (config.voyageKey && (isNewWorkspace || !indexingService)) {
        indexingService = new IndexingService(config.voyageKey, (p) => graphService?.updateFile(p));
        await indexingService.initialize(workspacePath);
        await indexingService.indexWorkspace();
      }

      win?.webContents.send('agent:step', { tool: 'indexing_workspace', status: 'done', summary: `Indexed workspace context` });
    } else if (workspacePath) {
      // Background refresh only if needed (don't show UI step for this to avoid annoyance)
      refreshManifest(workspacePath).catch(() => { });
    }

    // 2. Run the agent loop
    const result = await runAgentLoop(task, primaryModel, toolModel, config, workspacePath, activeFile, isAutopilotMode);
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
  if (agentAbortController) {
    agentAbortController.abort();
    agentAbortController = null;
  }
  if (pendingPermissionResolver) {
    pendingPermissionResolver({ approved: false });
  }
  if (currentActiveProcess) {
    try {
      currentActiveProcess.kill();
    } catch (e) {
      console.error('Failed to kill active process:', e);
    }
    currentActiveProcess = null;
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

// Hooks management
ipcMain.handle('hooks:list', async () => {
  if (!hooksManager) return [];
  return hooksManager.getAllHooks();
});

ipcMain.handle('hooks:get', async (_event, hookId: string) => {
  if (!hooksManager) return null;
  return hooksManager.getHook(hookId);
});

ipcMain.handle('hooks:save', async (_event, hook: any) => {
  if (!hooksManager) throw new Error('No workspace open');
  await hooksManager.saveHook(hook);
  return { success: true };
});

ipcMain.handle('hooks:delete', async (_event, hookId: string) => {
  if (!hooksManager) throw new Error('No workspace open');
  await hooksManager.deleteHook(hookId);
  return { success: true };
});

ipcMain.handle('hooks:reload', async () => {
  if (!hooksManager) throw new Error('No workspace open');
  await hooksManager.loadHooks();
  return { success: true };
});

// Steering management
ipcMain.handle('steering:list', async () => {
  if (!steeringManager) return [];
  return steeringManager.getAllSteeringFiles();
});

ipcMain.handle('steering:get', async (_event, steeringId: string) => {
  if (!steeringManager) return null;
  return steeringManager.getSteeringFile(steeringId);
});

ipcMain.handle('steering:save', async (_event, steering: any) => {
  if (!steeringManager) throw new Error('No workspace open');
  await steeringManager.saveSteeringFile(steering);
  return { success: true };
});

ipcMain.handle('steering:delete', async (_event, steeringId: string) => {
  if (!steeringManager) throw new Error('No workspace open');
  await steeringManager.deleteSteeringFile(steeringId);
  return { success: true };
});

ipcMain.handle('steering:reload', async () => {
  if (!steeringManager) throw new Error('No workspace open');
  await steeringManager.loadSteeringFiles();
  return { success: true };
});


// ------ SEARCH HANDLERS ------
ipcMain.handle('search:files', async (_event, { path, query, include, exclude }) => {
  try {
    const results: Array<{ file: string, line: number, content: string }> = [];

    async function searchInFile(filePath: string) {
      try {
        const content = await fs.readFile(filePath, 'utf-8');
        const lines = content.split('\n');
        const regex = new RegExp(query, 'gi');

        lines.forEach((line, idx) => {
          if (regex.test(line)) {
            results.push({
              file: filePath,
              line: idx + 1,
              content: line.trim()
            });
          }
        });
      } catch (err) {
        // Skip files that can't be read
      }
    }

    async function searchDirectory(dirPath: string) {
      const entries = await fs.readdir(dirPath, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = join(dirPath, entry.name);

        if (entry.isDirectory()) {
          if (!SKIP_DIRS.has(entry.name)) {
            await searchDirectory(fullPath);
          }
        } else if (entry.isFile()) {
          const ext = entry.name.substring(entry.name.lastIndexOf('.'));
          if (!BINARY_EXTS.has(ext)) {
            // Apply include/exclude patterns
            if (include && !entry.name.includes(include)) continue;
            if (exclude && entry.name.includes(exclude)) continue;

            await searchInFile(fullPath);
          }
        }
      }
    }

    await searchDirectory(path);
    return results.slice(0, 500); // Limit results
  } catch (err) {
    console.error('Search error:', err);
    return [];
  }
});

// ------ GIT HANDLERS ------
ipcMain.handle('git:status', async (_event, workspacePath) => {
  try {
    // Check if git is available
    const { stdout: branchOut } = await execAsync('git rev-parse --abbrev-ref HEAD', { cwd: workspacePath });
    const branch = branchOut.trim();

    // Get status
    const { stdout: statusOut } = await execAsync('git status --porcelain', { cwd: workspacePath });
    const changes = statusOut.split('\n')
      .filter(line => line.trim())
      .map(line => {
        const status = line.substring(0, 2).trim();
        const file = line.substring(3);
        return { file, status: status || 'M' };
      });

    return { branch, changes };
  } catch (err) {
    console.error('Git status error:', err);
    return null;
  }
});

ipcMain.handle('git:stage', async (_event, { path, file }) => {
  try {
    await execAsync(`git add "${file}"`, { cwd: path });
    return true;
  } catch (err) {
    console.error('Git stage error:', err);
    return false;
  }
});

ipcMain.handle('git:commit', async (_event, { path, message }) => {
  try {
    await execAsync(`git commit -m "${message.replace(/"/g, '\\"')}"`, { cwd: path });
    return true;
  } catch (err) {
    console.error('Git commit error:', err);
    throw err;
  }
});
