// Enhanced MCP System for WhizCode
// Implements comprehensive MCP server management and powers marketplace

import * as fs from 'fs';
import * as path from 'path';
import { app } from 'electron';
import { spawn, ChildProcess } from 'child_process';

export interface MCPPower {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  category: 'database' | 'cloud' | 'api' | 'development' | 'productivity' | 'analysis';
  keywords: string[];
  installCommand: string;
  configSchema?: any;
  tools: MCPTool[];
  dependencies?: string[];
  installed: boolean;
  enabled: boolean;
  lastUpdated: Date;
}

export interface MCPTool {
  name: string;
  description: string;
  inputSchema: any;
  outputSchema?: any;
  category: string;
  examples?: any[];
}

export interface MCPServer {
  id: string;
  powerId: string;
  name: string;
  command: string;
  args: string[];
  env?: Record<string, string>;
  process?: ChildProcess;
  status: 'stopped' | 'starting' | 'running' | 'error';
  lastError?: string;
  tools: MCPTool[];
  autoRestart: boolean;
}

export interface MCPMarketplace {
  powers: MCPPower[];
  categories: string[];
  featured: string[];
  trending: string[];
  lastUpdated: Date;
}

export class EnhancedMCPSystem {
  private configPath: string;
  private marketplacePath: string;
  private installedPowers: Map<string, MCPPower> = new Map();
  private runningServers: Map<string, MCPServer> = new Map();
  private marketplace: MCPMarketplace | null = null;
  private toolCache: Map<string, any> = new Map();

  constructor(workspacePath?: string) {
    const baseDir = workspacePath ? path.join(workspacePath, '.whizcode') : path.join(app.getPath('userData'), 'mcp');
    this.configPath = path.join(baseDir, 'mcp-config.json');
    this.marketplacePath = path.join(baseDir, 'marketplace.json');
    
    this.ensureDirectories();
    this.loadConfiguration();
    this.loadMarketplace();
  }

  private ensureDirectories() {
    const dirs = [
      path.dirname(this.configPath),
      path.dirname(this.marketplacePath)
    ];
    
    dirs.forEach(dir => {
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
    });
  }

  private loadConfiguration() {
    try {
      if (fs.existsSync(this.configPath)) {
        const config = JSON.parse(fs.readFileSync(this.configPath, 'utf8'));
        
        if (config.powers) {
          config.powers.forEach((power: any) => {
            power.lastUpdated = new Date(power.lastUpdated);
            this.installedPowers.set(power.id, power);
          });
        }
        
        if (config.servers) {
          config.servers.forEach((server: any) => {
            this.runningServers.set(server.id, {
              ...server,
              status: 'stopped', // Reset status on startup
              process: undefined
            });
          });
        }
      }
    } catch (error) {
      console.warn('[MCP] Failed to load configuration:', error);
    }
  }

  private loadMarketplace() {
    try {
      if (fs.existsSync(this.marketplacePath)) {
        const marketplace = JSON.parse(fs.readFileSync(this.marketplacePath, 'utf8'));
        marketplace.lastUpdated = new Date(marketplace.lastUpdated);
        this.marketplace = marketplace;
      } else {
        // Initialize with default marketplace
        this.marketplace = this.getDefaultMarketplace();
        this.saveMarketplace();
      }
    } catch (error) {
      console.warn('[MCP] Failed to load marketplace:', error);
      this.marketplace = this.getDefaultMarketplace();
    }
  }

  private getDefaultMarketplace(): MCPMarketplace {
    return {
      powers: [
        {
          id: 'filesystem-power',
          name: 'File System Power',
          description: 'Advanced file system operations and monitoring',
          version: '1.0.0',
          author: 'WhizCode Team',
          category: 'development',
          keywords: ['files', 'filesystem', 'monitoring'],
          installCommand: 'uvx filesystem-mcp-server@latest',
          tools: [
            {
              name: 'watch_directory',
              description: 'Watch directory for changes',
              inputSchema: { type: 'object', properties: { path: { type: 'string' } } },
              category: 'filesystem'
            },
            {
              name: 'bulk_operations',
              description: 'Perform bulk file operations',
              inputSchema: { type: 'object', properties: { operation: { type: 'string' }, files: { type: 'array' } } },
              category: 'filesystem'
            }
          ],
          installed: false,
          enabled: false,
          lastUpdated: new Date()
        },
        {
          id: 'database-power',
          name: 'Database Power',
          description: 'Connect and query various databases',
          version: '1.2.0',
          author: 'Database Team',
          category: 'database',
          keywords: ['sql', 'database', 'query', 'mysql', 'postgres'],
          installCommand: 'uvx database-mcp-server@latest',
          tools: [
            {
              name: 'execute_query',
              description: 'Execute SQL queries',
              inputSchema: { type: 'object', properties: { query: { type: 'string' }, database: { type: 'string' } } },
              category: 'database'
            },
            {
              name: 'describe_schema',
              description: 'Describe database schema',
              inputSchema: { type: 'object', properties: { database: { type: 'string' } } },
              category: 'database'
            }
          ],
          installed: false,
          enabled: false,
          lastUpdated: new Date()
        },
        {
          id: 'web-scraper-power',
          name: 'Web Scraper Power',
          description: 'Extract data from websites and APIs',
          version: '2.1.0',
          author: 'Web Team',
          category: 'api',
          keywords: ['web', 'scraping', 'api', 'http', 'data'],
          installCommand: 'uvx web-scraper-mcp@latest',
          tools: [
            {
              name: 'scrape_page',
              description: 'Scrape content from web pages',
              inputSchema: { type: 'object', properties: { url: { type: 'string' }, selector: { type: 'string' } } },
              category: 'web'
            },
            {
              name: 'api_request',
              description: 'Make HTTP API requests',
              inputSchema: { type: 'object', properties: { url: { type: 'string' }, method: { type: 'string' }, headers: { type: 'object' } } },
              category: 'api'
            }
          ],
          installed: false,
          enabled: false,
          lastUpdated: new Date()
        },
        {
          id: 'aws-power',
          name: 'AWS Cloud Power',
          description: 'Manage AWS resources and services',
          version: '1.5.0',
          author: 'Cloud Team',
          category: 'cloud',
          keywords: ['aws', 'cloud', 'ec2', 's3', 'lambda'],
          installCommand: 'uvx aws-mcp-server@latest',
          configSchema: {
            type: 'object',
            properties: {
              accessKeyId: { type: 'string', description: 'AWS Access Key ID' },
              secretAccessKey: { type: 'string', description: 'AWS Secret Access Key' },
              region: { type: 'string', description: 'AWS Region', default: 'us-east-1' }
            },
            required: ['accessKeyId', 'secretAccessKey']
          },
          tools: [
            {
              name: 'list_ec2_instances',
              description: 'List EC2 instances',
              inputSchema: { type: 'object', properties: { region: { type: 'string' } } },
              category: 'cloud'
            },
            {
              name: 'upload_to_s3',
              description: 'Upload files to S3',
              inputSchema: { type: 'object', properties: { bucket: { type: 'string' }, key: { type: 'string' }, file: { type: 'string' } } },
              category: 'cloud'
            }
          ],
          installed: false,
          enabled: false,
          lastUpdated: new Date()
        },
        {
          id: 'code-analysis-power',
          name: 'Code Analysis Power',
          description: 'Advanced code analysis and metrics',
          version: '1.3.0',
          author: 'Analysis Team',
          category: 'analysis',
          keywords: ['code', 'analysis', 'metrics', 'quality', 'security'],
          installCommand: 'uvx code-analysis-mcp@latest',
          tools: [
            {
              name: 'analyze_security',
              description: 'Analyze code for security vulnerabilities',
              inputSchema: { type: 'object', properties: { path: { type: 'string' }, language: { type: 'string' } } },
              category: 'security'
            },
            {
              name: 'calculate_metrics',
              description: 'Calculate advanced code metrics',
              inputSchema: { type: 'object', properties: { path: { type: 'string' } } },
              category: 'metrics'
            }
          ],
          installed: false,
          enabled: false,
          lastUpdated: new Date()
        }
      ],
      categories: ['database', 'cloud', 'api', 'development', 'productivity', 'analysis'],
      featured: ['database-power', 'aws-power'],
      trending: ['web-scraper-power', 'code-analysis-power'],
      lastUpdated: new Date()
    };
  }

  private saveConfiguration() {
    try {
      const config = {
        powers: Array.from(this.installedPowers.values()),
        servers: Array.from(this.runningServers.values()).map(server => ({
          ...server,
          process: undefined // Don't serialize process
        }))
      };
      
      fs.writeFileSync(this.configPath, JSON.stringify(config, null, 2));
    } catch (error) {
      console.error('[MCP] Failed to save configuration:', error);
    }
  }

  private saveMarketplace() {
    try {
      if (this.marketplace) {
        fs.writeFileSync(this.marketplacePath, JSON.stringify(this.marketplace, null, 2));
      }
    } catch (error) {
      console.error('[MCP] Failed to save marketplace:', error);
    }
  }

  // Public API Methods

  async refreshMarketplace(): Promise<void> {
    try {
      // In a real implementation, this would fetch from a remote marketplace
      // For now, we'll update the local marketplace with latest info
      if (this.marketplace) {
        this.marketplace.lastUpdated = new Date();
        this.saveMarketplace();
      }
      console.log('[MCP] Marketplace refreshed');
    } catch (error) {
      console.error('[MCP] Failed to refresh marketplace:', error);
      throw error;
    }
  }

  getMarketplace(): MCPMarketplace | null {
    return this.marketplace;
  }

  getInstalledPowers(): MCPPower[] {
    return Array.from(this.installedPowers.values());
  }

  getAvailablePowers(): MCPPower[] {
    return this.marketplace?.powers || [];
  }

  getPowersByCategory(category: string): MCPPower[] {
    return this.getAvailablePowers().filter(power => power.category === category);
  }

  searchPowers(query: string): MCPPower[] {
    const lowerQuery = query.toLowerCase();
    return this.getAvailablePowers().filter(power => 
      power.name.toLowerCase().includes(lowerQuery) ||
      power.description.toLowerCase().includes(lowerQuery) ||
      power.keywords.some(keyword => keyword.toLowerCase().includes(lowerQuery))
    );
  }

  async installPower(powerId: string): Promise<void> {
    const power = this.marketplace?.powers.find(p => p.id === powerId);
    if (!power) {
      throw new Error(`Power ${powerId} not found in marketplace`);
    }

    if (power.installed) {
      throw new Error(`Power ${powerId} is already installed`);
    }

    try {
      console.log(`[MCP] Installing power: ${power.name}`);
      
      // Execute installation command
      await this.executeCommand(power.installCommand);
      
      // Mark as installed
      power.installed = true;
      power.lastUpdated = new Date();
      
      this.installedPowers.set(powerId, power);
      this.saveConfiguration();
      
      console.log(`[MCP] Successfully installed power: ${power.name}`);
    } catch (error) {
      console.error(`[MCP] Failed to install power ${powerId}:`, error);
      throw error;
    }
  }

  async uninstallPower(powerId: string): Promise<void> {
    const power = this.installedPowers.get(powerId);
    if (!power) {
      throw new Error(`Power ${powerId} is not installed`);
    }

    try {
      // Stop server if running
      await this.stopPowerServer(powerId);
      
      // Remove from installed powers
      this.installedPowers.delete(powerId);
      
      // Update marketplace entry
      if (this.marketplace) {
        const marketplacePower = this.marketplace.powers.find(p => p.id === powerId);
        if (marketplacePower) {
          marketplacePower.installed = false;
          marketplacePower.enabled = false;
        }
      }
      
      this.saveConfiguration();
      console.log(`[MCP] Successfully uninstalled power: ${power.name}`);
    } catch (error) {
      console.error(`[MCP] Failed to uninstall power ${powerId}:`, error);
      throw error;
    }
  }

  async enablePower(powerId: string, config?: any): Promise<void> {
    const power = this.installedPowers.get(powerId);
    if (!power) {
      throw new Error(`Power ${powerId} is not installed`);
    }

    try {
      // Validate configuration if schema provided
      if (power.configSchema && config) {
        this.validateConfig(config, power.configSchema);
      }

      // Create and start server
      const server: MCPServer = {
        id: `${powerId}-server`,
        powerId,
        name: power.name,
        command: 'uvx',
        args: power.installCommand.split(' ').slice(1), // Remove 'uvx' from command
        env: config ? this.configToEnv(config) : undefined,
        status: 'stopped',
        tools: power.tools,
        autoRestart: true
      };

      await this.startServer(server);
      
      power.enabled = true;
      this.saveConfiguration();
      
      console.log(`[MCP] Successfully enabled power: ${power.name}`);
    } catch (error) {
      console.error(`[MCP] Failed to enable power ${powerId}:`, error);
      throw error;
    }
  }

  async disablePower(powerId: string): Promise<void> {
    const power = this.installedPowers.get(powerId);
    if (!power) {
      throw new Error(`Power ${powerId} is not installed`);
    }

    try {
      await this.stopPowerServer(powerId);
      
      power.enabled = false;
      this.saveConfiguration();
      
      console.log(`[MCP] Successfully disabled power: ${power.name}`);
    } catch (error) {
      console.error(`[MCP] Failed to disable power ${powerId}:`, error);
      throw error;
    }
  }

  async executePowerTool(powerId: string, toolName: string, args: any): Promise<any> {
    const server = Array.from(this.runningServers.values()).find(s => s.powerId === powerId);
    if (!server || server.status !== 'running') {
      throw new Error(`Power ${powerId} is not running`);
    }

    const tool = server.tools.find(t => t.name === toolName);
    if (!tool) {
      throw new Error(`Tool ${toolName} not found in power ${powerId}`);
    }

    try {
      // Validate input against schema
      this.validateToolInput(args, tool.inputSchema);
      
      // Check cache first
      const cacheKey = `${powerId}:${toolName}:${JSON.stringify(args)}`;
      if (this.toolCache.has(cacheKey)) {
        console.log(`[MCP] Using cached result for ${toolName}`);
        return this.toolCache.get(cacheKey);
      }

      // Execute tool (simplified - in real implementation would use MCP protocol)
      const result = await this.executeToolOnServer(server, toolName, args);
      
      // Cache result if successful
      this.toolCache.set(cacheKey, result);
      
      return result;
    } catch (error) {
      console.error(`[MCP] Failed to execute tool ${toolName} on power ${powerId}:`, error);
      throw error;
    }
  }

  getAvailableTools(): MCPTool[] {
    const tools: MCPTool[] = [];
    
    for (const server of this.runningServers.values()) {
      if (server.status === 'running') {
        tools.push(...server.tools);
      }
    }
    
    return tools;
  }

  getToolsByCategory(category: string): MCPTool[] {
    return this.getAvailableTools().filter(tool => tool.category === category);
  }

  clearToolCache(): void {
    this.toolCache.clear();
    console.log('[MCP] Tool cache cleared');
  }

  // Private helper methods

  private async executeCommand(command: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const [cmd, ...args] = command.split(' ');
      const process = spawn(cmd, args, { stdio: 'pipe' });
      
      let output = '';
      let error = '';
      
      process.stdout?.on('data', (data) => {
        output += data.toString();
      });
      
      process.stderr?.on('data', (data) => {
        error += data.toString();
      });
      
      process.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`Command failed with code ${code}: ${error}`));
        }
      });
      
      process.on('error', (err) => {
        reject(err);
      });
    });
  }

  private async startServer(server: MCPServer): Promise<void> {
    try {
      server.status = 'starting';
      this.runningServers.set(server.id, server);
      
      const childProcess = spawn(server.command, server.args, {
        env: { ...process.env, ...server.env },
        stdio: 'pipe'
      });
      
      server.process = childProcess;
      
      childProcess.on('spawn', () => {
        server.status = 'running';
        server.lastError = undefined;
        console.log(`[MCP] Server ${server.name} started successfully`);
      });
      
      childProcess.on('error', (error) => {
        server.status = 'error';
        server.lastError = error.message;
        console.error(`[MCP] Server ${server.name} error:`, error);
        
        if (server.autoRestart) {
          setTimeout(() => this.startServer(server), 5000);
        }
      });
      
      childProcess.on('exit', (code) => {
        server.status = 'stopped';
        server.process = undefined;
        console.log(`[MCP] Server ${server.name} exited with code ${code}`);
        
        if (server.autoRestart && code !== 0) {
          setTimeout(() => this.startServer(server), 5000);
        }
      });
      
    } catch (error) {
      server.status = 'error';
      server.lastError = error instanceof Error ? error.message : 'Unknown error';
      throw error;
    }
  }

  private async stopPowerServer(powerId: string): Promise<void> {
    const server = Array.from(this.runningServers.values()).find(s => s.powerId === powerId);
    if (server) {
      await this.stopServer(server.id);
    }
  }

  private async stopServer(serverId: string): Promise<void> {
    const server = this.runningServers.get(serverId);
    if (!server) return;
    
    if (server.process) {
      server.process.kill();
      server.process = undefined;
    }
    
    server.status = 'stopped';
    console.log(`[MCP] Server ${server.name} stopped`);
  }

  private validateConfig(config: any, schema: any): void {
    // Simple validation - in real implementation would use JSON Schema validator
    if (schema.required) {
      for (const field of schema.required) {
        if (!(field in config)) {
          throw new Error(`Required field ${field} is missing`);
        }
      }
    }
  }

  private configToEnv(config: any): Record<string, string> {
    const env: Record<string, string> = {};
    
    for (const [key, value] of Object.entries(config)) {
      env[key.toUpperCase()] = String(value);
    }
    
    return env;
  }

  private validateToolInput(input: any, schema: any): void {
    // Simple validation - in real implementation would use JSON Schema validator
    if (schema.required) {
      for (const field of schema.required) {
        if (!(field in input)) {
          throw new Error(`Required field ${field} is missing`);
        }
      }
    }
  }

  private async executeToolOnServer(_server: MCPServer, toolName: string, args: any): Promise<any> {
    // Simplified implementation - in real MCP would use JSON-RPC protocol
    // For now, return mock data based on tool type
    
    await new Promise(resolve => setTimeout(resolve, 100)); // Simulate network delay
    
    switch (toolName) {
      case 'execute_query':
        return { rows: [], rowCount: 0, executionTime: '0.1ms' };
      case 'scrape_page':
        return { content: 'Mock scraped content', elements: [] };
      case 'list_ec2_instances':
        return { instances: [], region: args.region || 'us-east-1' };
      case 'analyze_security':
        return { vulnerabilities: [], score: 95, recommendations: [] };
      default:
        return { success: true, data: args };
    }
  }

  async shutdown(): Promise<void> {
    console.log('[MCP] Shutting down MCP system...');
    
    // Stop all running servers
    const stopPromises = Array.from(this.runningServers.keys()).map(serverId => 
      this.stopServer(serverId)
    );
    
    await Promise.all(stopPromises);
    
    // Clear cache
    this.toolCache.clear();
    
    console.log('[MCP] MCP system shutdown complete');
  }
}