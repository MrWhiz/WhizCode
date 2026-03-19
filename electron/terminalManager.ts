/**
 * Terminal Manager - Handles multiple terminal instances
 * Supports bash, cmd, powershell on Windows/Mac/Linux
 */

import { platform } from 'os';
import { EventEmitter } from 'events';
import * as pty from 'node-pty';

export type TerminalType = 'bash' | 'cmd' | 'powershell' | 'zsh' | 'sh';

export interface TerminalInstance {
  id: string;
  type: TerminalType;
  process: pty.IPty;
  cols: number;
  rows: number;
  cwd: string;
  createdAt: number;
  lastActivity: number;
}

export interface TerminalConfig {
  type: TerminalType;
  cwd?: string;
  env?: NodeJS.ProcessEnv;
}

export class TerminalManager extends EventEmitter {
  private terminals: Map<string, TerminalInstance> = new Map();
  private terminalCounter = 0;

  /**
   * Create a new terminal instance with a specific ID
   */
  /**
     * Create a new terminal instance with a specific ID
     */
    createTerminal(config: TerminalConfig, customId?: string): string {
      const id = customId || `terminal_${++this.terminalCounter}`;
      let shell = this.getShellCommand(config.type);
      const cwd = config.cwd || process.cwd();

      try {
        let childProcess;

        try {
          childProcess = pty.spawn(shell.command, shell.args, {
            name: 'xterm-color',
            cols: 80,
            rows: 24,
            cwd,
            env: { ...process.env, ...config.env }
          });
        } catch (err: any) {
          // If shell not found, try fallback
          const currentPlatform = platform();
          if (currentPlatform === 'win32' && config.type === 'bash') {
            // Fallback to PowerShell on Windows if bash not found
            shell = { command: 'powershell.exe', args: ['-NoExit', '-Command', 'Clear-Host'] };
            childProcess = pty.spawn(shell.command, shell.args, {
              name: 'xterm-color',
              cols: 80,
              rows: 24,
              cwd,
              env: { ...process.env, ...config.env }
            });
          } else {
            throw err;
          }
        }

        const terminal: TerminalInstance = {
          id,
          type: config.type,
          process: childProcess,
          cols: 80,
          rows: 24,
          cwd,
          createdAt: Date.now(),
          lastActivity: Date.now()
        };

        // Register data handler BEFORE adding to map
        childProcess.onData((data: string) => {
          this.emit('data', id, data);
        });

        // Register exit handler BEFORE adding to map
        childProcess.onExit(({ exitCode }: { exitCode: number }) => {
          this.emit('exit', id, exitCode);
          this.terminals.delete(id);
        });

        // Now add to map
        this.terminals.set(id, terminal);

        // Send initial resize to trigger prompt display
        try {
          childProcess.resize(80, 24);
        } catch (err) {
          // Ignore resize errors
        }

        return id;
      } catch (err) {
        throw new Error(`Failed to create terminal: ${err}`);
      }
    }


  /**
   * Write data to terminal
   */
  write(id: string, data: string): void {
    const terminal = this.terminals.get(id);
    if (!terminal) {
      throw new Error(`Terminal ${id} not found`);
    }

    terminal.lastActivity = Date.now();
    terminal.process.write(data);
  }

  /**
   * Resize terminal
   */
  resize(id: string, cols: number, rows: number): void {
    const terminal = this.terminals.get(id);
    if (!terminal) throw new Error(`Terminal ${id} not found`);

    terminal.cols = cols;
    terminal.rows = rows;

    try {
      terminal.process.resize(cols, rows);
    } catch (err) {
      console.error('Failed to resize terminal:', err);
    }
  }

  /**
   * Kill terminal
   */
  kill(id: string): void {
    const terminal = this.terminals.get(id);
    if (!terminal) return;

    try {
      terminal.process.kill();
    } catch (err) {
      console.error('Failed to kill terminal:', err);
    }

    this.terminals.delete(id);
  }

  /**
   * Get terminal info
   */
  getTerminal(id: string): TerminalInstance | undefined {
    return this.terminals.get(id);
  }

  /**
   * Get all terminals
   */
  getAllTerminals(): TerminalInstance[] {
    return Array.from(this.terminals.values());
  }

  /**
   * Get available shell types for current platform
   */
  getAvailableShells(): TerminalType[] {
    const currentPlatform = platform();
    
    if (currentPlatform === 'win32') {
      return ['powershell', 'cmd'];
    } else if (currentPlatform === 'darwin') {
      return ['bash', 'zsh', 'sh'];
    } else {
      return ['bash', 'sh'];
    }
  }

  /**
   * Get default shell for platform
   */
  getDefaultShell(): TerminalType {
    const currentPlatform = platform();
    
    if (currentPlatform === 'win32') {
      return 'powershell';
    } else if (currentPlatform === 'darwin') {
      return 'zsh';
    } else {
      return 'bash';
    }
  }

  /**
   * Get shell command and args
   */
  private getShellCommand(type: TerminalType): { command: string; args: string[] } {
    const currentPlatform = platform();

    switch (type) {
      case 'bash':
        if (currentPlatform === 'win32') {
          // On Windows, try to find bash (Git Bash, WSL, etc.)
          return { command: 'bash.exe', args: ['-i'] };
        }
        return { command: 'bash', args: ['-i'] };
      case 'zsh':
        return { command: 'zsh', args: ['-i'] };
      case 'sh':
        return { command: 'sh', args: ['-i'] };
      case 'cmd':
        return { command: 'cmd.exe', args: [] };
      case 'powershell':
        if (currentPlatform === 'win32') {
          // PowerShell needs -NoExit to keep running, and -Command to execute initial command
          return { command: 'powershell.exe', args: ['-NoExit', '-Command', '$host.ui.RawUI.WindowTitle = "PowerShell"'] };
        } else {
          return { command: 'pwsh', args: ['-NoExit', '-Command', '$host.ui.RawUI.WindowTitle = "PowerShell"'] };
        }
      default:
        throw new Error(`Unknown shell type: ${type}`);
    }
  }

  /**
   * Clear all terminals
   */
  clearAll(): void {
    for (const [id] of this.terminals) {
      this.kill(id);
    }
    this.terminals.clear();
  }
}
