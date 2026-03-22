// ====== MCP SERVICE ======
// Model Context Protocol client — connects to external tool servers
// and exposes their tools dynamically to the WhizCode agent.
//
// Config file: {workspace}/.whizcode/mcp-servers.json
// Example config:
// {
//   "servers": [
//     { "name": "filesystem", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "."] },
//     { "name": "github",     "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"],
//       "env": { "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_..." } },
//     { "name": "puppeteer",  "command": "npx", "args": ["-y", "@modelcontextprotocol/server-puppeteer"] }
//   ]
// }

import { spawn, type ChildProcess } from 'node:child_process';
import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export interface MCPServerConfig {
  name: string;
  command: string;
  args: string[];
  env?: Record<string, string>;
  enabled?: boolean;
}

export interface MCPToolDefinition {
  name: string;
  description: string;
  inputSchema: any;
  serverName: string;
}

interface MCPRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params?: any;
}

interface MCPResponse {
  jsonrpc: '2.0';
  id: number;
  result?: any;
  error?: { code: number; message: string; data?: any };
}

class MCPClient {
  private process: ChildProcess;
  private pendingRequests: Map<number, { resolve: (v: any) => void; reject: (e: any) => void }> = new Map();
  private msgId = 1;
  private buffer = '';
  public tools: MCPToolDefinition[] = [];
  public ready = false;
  public name: string;

  constructor(name: string, config: MCPServerConfig) {
    this.name = name;
    const env = { ...process.env, ...(config.env || {}) };
    this.process = spawn(config.command, config.args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      env,
      shell: false
    });

    this.process.stdout?.on('data', (data: Buffer) => {
      this.buffer += data.toString();
      const lines = this.buffer.split('\n');
      this.buffer = lines.pop() || '';
      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const msg: MCPResponse = JSON.parse(line);
          const pending = this.pendingRequests.get(msg.id);
          if (pending) {
            this.pendingRequests.delete(msg.id);
            if (msg.error) pending.reject(new Error(msg.error.message));
            else pending.resolve(msg.result);
          }
        } catch { /* ignore malformed lines */ }
      }
    });

    this.process.stderr?.on('data', (data: Buffer) => {
      console.warn(`[MCP:${name}] stderr:`, data.toString().trim());
    });

    this.process.on('exit', (code: number | null) => {
      console.log(`[MCP:${name}] process exited with code ${code}`);
      this.ready = false;
      // Reject any pending requests
      for (const [, pending] of this.pendingRequests) {
        pending.reject(new Error(`MCP server ${name} exited`));
      }
      this.pendingRequests.clear();
    });
  }

  private send(method: string, params?: any): Promise<any> {
    return new Promise((resolve, reject) => {
      const id = this.msgId++;
      const request: MCPRequest = { jsonrpc: '2.0', id, method, params };
      this.pendingRequests.set(id, { resolve, reject });

      const line = JSON.stringify(request) + '\n';
      this.process.stdin?.write(line);

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error(`MCP request ${method} timed out after 30s`));
        }
      }, 30000);
    });
  }

  async initialize(): Promise<void> {
    try {
      await this.send('initialize', {
        protocolVersion: '2024-11-05',
        capabilities: { tools: {} },
        clientInfo: { name: 'WhizCode', version: '1.0.0' }
      });
      await this.send('notifications/initialized');
      await this.loadTools();
      this.ready = true;
      console.log(`[MCP:${this.name}] Ready with ${this.tools.length} tools`);
    } catch (e) {
      console.error(`[MCP:${this.name}] Failed to initialize:`, e);
    }
  }

  private async loadTools(): Promise<void> {
    try {
      const result = await this.send('tools/list', {});
      this.tools = (result?.tools || []).map((t: any) => ({
        name: t.name,
        description: t.description || `Tool from MCP server: ${this.name}`,
        inputSchema: t.inputSchema || {},
        serverName: this.name
      }));
    } catch (e) {
      console.warn(`[MCP:${this.name}] Failed to list tools:`, e);
    }
  }

  async callTool(toolName: string, args: Record<string, any>): Promise<string> {
    if (!this.ready) throw new Error(`MCP server ${this.name} is not ready`);
    const result = await this.send('tools/call', { name: toolName, arguments: args });
    // MCP tools/call result is an array of content blocks
    const content = result?.content || [];
    return content
      .filter((c: any) => c.type === 'text')
      .map((c: any) => c.text)
      .join('\n') || JSON.stringify(result);
  }

  shutdown(): void {
    try {
      this.process.kill();
    } catch { /* ignore */ }
  }
}

export class MCPManager {
  private clients: Map<string, MCPClient> = new Map();
  private configPath: string;

  constructor(workspaceRoot: string) {
    this.configPath = join(workspaceRoot, '.whizcode', 'mcp-servers.json');
  }

  async initialize(): Promise<void> {
    const configs = await this.loadConfig();
    if (configs.length === 0) {
      console.log('[MCP] No server configs found. Create .whizcode/mcp-servers.json to add servers.');
      return;
    }

    console.log(`[MCP] Starting ${configs.length} server(s)...`);
    await Promise.all(configs.map(c => this.startServer(c)));
  }

  private async loadConfig(): Promise<MCPServerConfig[]> {
    try {
      const raw = await fs.readFile(this.configPath, 'utf-8');
      const parsed = JSON.parse(raw);
      const servers: MCPServerConfig[] = parsed.servers || [];
      return servers.filter(s => s.enabled !== false);
    } catch {
      return [];
    }
  }

  private async startServer(config: MCPServerConfig): Promise<void> {
    try {
      const client = new MCPClient(config.name, config);
      await client.initialize();
      this.clients.set(config.name, client);
    } catch (e) {
      console.error(`[MCP] Failed to start server "${config.name}":`, e);
    }
  }

  /** Returns all tools from all connected MCP servers */
  getAllTools(): MCPToolDefinition[] {
    const tools: MCPToolDefinition[] = [];
    for (const client of this.clients.values()) {
      tools.push(...client.tools);
    }
    return tools;
  }

  /** Calls the correct MCP server for a given tool */
  async callTool(toolName: string, args: Record<string, any>): Promise<string> {
    for (const client of this.clients.values()) {
      const tool = client.tools.find(t => t.name === toolName);
      if (tool) {
        return await client.callTool(toolName, args);
      }
    }
    throw new Error(`No MCP server has a tool named "${toolName}"`);
  }

  /** Generates the system prompt section describing available MCP tools */
  buildToolPrompt(): string {
    const tools = this.getAllTools();
    if (tools.length === 0) return '';

    const toolList = tools.map(t => `- ${t.name} (${t.serverName}): ${t.description}`).join('\n');
    return `\n<mcp_tools>\nThe following additional tools are available via connected MCP servers:\n${toolList}\n\nTo call an MCP tool, use:\n{"tool": "mcp_call", "toolName": "<tool_name>", "args": {...}}\n</mcp_tools>\n`;
  }

  getConnectedServers(): string[] {
    return Array.from(this.clients.entries())
      .filter(([, c]) => c.ready)
      .map(([name]) => name);
  }

  shutdown(): void {
    for (const client of this.clients.values()) {
      client.shutdown();
    }
    this.clients.clear();
  }
}
