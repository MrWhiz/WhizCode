// Hooks System for WhizCode
// Event-driven automation similar to WhizCode

import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export type HookEventType = 
  | 'fileEdited' 
  | 'fileCreated' 
  | 'fileDeleted' 
  | 'userTriggered' 
  | 'promptSubmit' 
  | 'agentStop'
  | 'preToolUse'
  | 'postToolUse'
  | 'preTaskExecution'
  | 'postTaskExecution';

export type HookActionType = 'askAgent' | 'runCommand';

export interface Hook {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  eventType: HookEventType;
  filePatterns?: string[]; // For file events
  toolTypes?: string[]; // For tool events (categories or regex)
  action: HookActionType;
  prompt?: string; // For askAgent
  command?: string; // For runCommand
  timeout?: number; // For runCommand (seconds)
}

export class HooksManager {
  private hooks: Map<string, Hook> = new Map();
  private hooksDir: string;

  constructor(workspaceRoot: string) {
    this.hooksDir = join(workspaceRoot, '.whizcode', 'hooks');
  }

  async initialize() {
    try {
      await fs.mkdir(this.hooksDir, { recursive: true });
      await this.loadHooks();
    } catch (e) {
      console.error('Failed to initialize hooks:', e);
    }
  }

  async loadHooks() {
    try {
      const files = await fs.readdir(this.hooksDir);
      const jsonFiles = files.filter(f => f.endsWith('.json'));

      for (const file of jsonFiles) {
        try {
          const content = await fs.readFile(join(this.hooksDir, file), 'utf-8');
          const hook: Hook = JSON.parse(content);
          this.hooks.set(hook.id, hook);
        } catch (e) {
          console.error(`Failed to load hook ${file}:`, e);
        }
      }

      console.log(`Loaded ${this.hooks.size} hooks`);
    } catch (e) {
      // Hooks directory doesn't exist yet
    }
  }

  async saveHook(hook: Hook) {
    const filePath = join(this.hooksDir, `${hook.id}.json`);
    await fs.mkdir(this.hooksDir, { recursive: true });
    await fs.writeFile(filePath, JSON.stringify(hook, null, 2), 'utf-8');
    this.hooks.set(hook.id, hook);
  }

  async deleteHook(hookId: string) {
    const filePath = join(this.hooksDir, `${hookId}.json`);
    try {
      await fs.unlink(filePath);
      this.hooks.delete(hookId);
    } catch (e) {
      console.error(`Failed to delete hook ${hookId}:`, e);
    }
  }

  getHook(hookId: string): Hook | undefined {
    return this.hooks.get(hookId);
  }

  getAllHooks(): Hook[] {
    return Array.from(this.hooks.values());
  }

  getEnabledHooks(): Hook[] {
    return Array.from(this.hooks.values()).filter(h => h.enabled);
  }

  getHooksForEvent(eventType: HookEventType): Hook[] {
    return this.getEnabledHooks().filter(h => h.eventType === eventType);
  }

  matchesFilePattern(filePath: string, patterns: string[]): boolean {
    if (!patterns || patterns.length === 0) return true;

    return patterns.some(pattern => {
      // Simple glob matching
      const regex = new RegExp(
        '^' + pattern
          .replace(/\./g, '\\.')
          .replace(/\*/g, '.*')
          .replace(/\?/g, '.')
        + '$'
      );
      return regex.test(filePath);
    });
  }

  matchesToolType(toolName: string, toolTypes: string[]): boolean {
    if (!toolTypes || toolTypes.length === 0) return true;

    // Built-in categories
    const categories: Record<string, string[]> = {
      'read': ['read_file', 'readCode', 'readMultipleFiles', 'list_directory', 'search_files', 'grepSearch', 'fileSearch'],
      'write': ['write_file', 'edit_file', 'editCode', 'delete_file', 'strReplace', 'smartRelocate'],
      'shell': ['run_command'],
      'web': ['remote_web_search', 'webFetch'],
      'spec': ['createSpec', 'updateSpec'],
      '*': ['*'] // All tools
    };

    return toolTypes.some(type => {
      // Check if it's a category
      if (categories[type]) {
        return categories[type].includes(toolName) || categories[type].includes('*');
      }
      
      // Otherwise treat as regex pattern
      try {
        const regex = new RegExp(type);
        return regex.test(toolName);
      } catch (e) {
        return false;
      }
    });
  }

  async triggerFileEvent(eventType: 'fileEdited' | 'fileCreated' | 'fileDeleted', filePath: string): Promise<Hook[]> {
    const hooks = this.getHooksForEvent(eventType);
    return hooks.filter(hook => this.matchesFilePattern(filePath, hook.filePatterns || []));
  }

  async triggerToolEvent(eventType: 'preToolUse' | 'postToolUse', toolName: string): Promise<Hook[]> {
    const hooks = this.getHooksForEvent(eventType);
    return hooks.filter(hook => this.matchesToolType(toolName, hook.toolTypes || []));
  }

  async triggerEvent(eventType: HookEventType): Promise<Hook[]> {
    return this.getHooksForEvent(eventType);
  }
}

// Example hooks for reference
export const EXAMPLE_HOOKS: Hook[] = [
  {
    id: 'lint-on-save',
    name: 'Lint on Save',
    description: 'Run linter when TypeScript files are edited',
    enabled: true,
    eventType: 'fileEdited',
    filePatterns: ['*.ts', '*.tsx'],
    action: 'runCommand',
    command: 'npm run lint',
    timeout: 30
  },
  {
    id: 'review-write-ops',
    name: 'Review Write Operations',
    description: 'Ask agent to verify write operations follow standards',
    enabled: true,
    eventType: 'preToolUse',
    toolTypes: ['write'],
    action: 'askAgent',
    prompt: 'Verify this write operation follows our coding standards and best practices.'
  },
  {
    id: 'test-after-task',
    name: 'Run Tests After Task',
    description: 'Run tests when agent completes a task',
    enabled: false,
    eventType: 'postTaskExecution',
    action: 'runCommand',
    command: 'npm test',
    timeout: 60
  }
];
