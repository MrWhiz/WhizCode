// Process Management System for WhizCode
// Handles detection and management of running project instances

import { exec } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import { app } from 'electron';

const execAsync = promisify(exec);

export interface RunningProcess {
  pid: number;
  command: string;
  port?: number;
  workspacePath?: string;
  startTime: Date;
  type: 'dev-server' | 'build' | 'test' | 'other';
}

export interface ProcessCheckResult {
  hasRunningInstances: boolean;
  processes: RunningProcess[];
  conflictingPorts: number[];
}

export class ProcessManager {
  private processHistoryPath: string;
  private runningProcesses: Map<string, RunningProcess> = new Map();
  private commonDevPorts = [3000, 3001, 4000, 4173, 5000, 5173, 8000, 8080, 8081, 9000];

  constructor(workspacePath?: string) {
    const baseDir = workspacePath 
      ? path.join(workspacePath, '.whizcode', 'processes')
      : path.join(app.getPath('userData'), 'processes');
    
    this.processHistoryPath = path.join(baseDir, 'running-processes.json');
    this.ensureDirectories();
    this.loadProcessHistory();
  }

  private ensureDirectories() {
    const dir = path.dirname(this.processHistoryPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
  }

  private loadProcessHistory() {
    try {
      if (fs.existsSync(this.processHistoryPath)) {
        const data = JSON.parse(fs.readFileSync(this.processHistoryPath, 'utf8'));
        data.forEach((proc: any) => {
          this.runningProcesses.set(proc.pid.toString(), {
            ...proc,
            startTime: new Date(proc.startTime)
          });
        });
      }
    } catch (error) {
      console.warn('[PROCESS_MANAGER] Failed to load process history:', error);
    }
  }

  private saveProcessHistory() {
    try {
      const data = Array.from(this.runningProcesses.values());
      fs.writeFileSync(this.processHistoryPath, JSON.stringify(data, null, 2));
    } catch (error) {
      console.error('[PROCESS_MANAGER] Failed to save process history:', error);
    }
  }

  /**
   * Check for running instances of the current project
   */
  async checkForRunningInstances(workspacePath: string): Promise<ProcessCheckResult> {
    const result: ProcessCheckResult = {
      hasRunningInstances: false,
      processes: [],
      conflictingPorts: []
    };

    try {
      // Clean up dead processes first (with timeout)
      try {
        await Promise.race([
          this.cleanupDeadProcesses(),
          new Promise((_, reject) => setTimeout(() => reject(new Error('Cleanup timed out')), 5000))
        ]);
      } catch (error) {
        console.warn('[PROCESS_MANAGER] Cleanup timed out, continuing:', error);
      }

      // Check for Node.js processes that might be related to this project (with timeout)
      let nodeProcesses: RunningProcess[] = [];
      try {
        nodeProcesses = await Promise.race([
          this.findNodeProcesses(workspacePath),
          new Promise<RunningProcess[]>((resolve) => setTimeout(() => resolve([]), 10000))
        ]);
      } catch (error) {
        console.warn('[PROCESS_MANAGER] Node process check timed out:', error);
      }
      
      // Check for processes using common dev ports (with timeout)
      let portConflicts: number[] = [];
      try {
        portConflicts = await Promise.race([
          this.checkPortConflicts(),
          new Promise<number[]>((resolve) => setTimeout(() => resolve([]), 10000))
        ]);
      } catch (error) {
        console.warn('[PROCESS_MANAGER] Port conflict check timed out:', error);
      }
      
      result.processes = [...nodeProcesses];
      result.conflictingPorts = portConflicts;
      result.hasRunningInstances = nodeProcesses.length > 0 || portConflicts.length > 0;

      return result;
    } catch (error) {
      console.error('[PROCESS_MANAGER] Error checking running instances:', error);
      return result;
    }
  }

  /**
   * Find Node.js processes that might be related to the current project
   */
  private async findNodeProcesses(workspacePath: string): Promise<RunningProcess[]> {
    const processes: RunningProcess[] = [];
    
    try {
      let command: string;
      let parseOutput: (output: string) => RunningProcess[];

      if (process.platform === 'win32') {
        // Windows: Use wmic to get process info
        command = 'wmic process where "name=\'node.exe\' or name=\'npm.exe\' or name=\'yarn.exe\' or name=\'pnpm.exe\'" get ProcessId,CommandLine,CreationDate /format:csv';
        parseOutput = this.parseWindowsProcessOutput.bind(this);
      } else {
        // Unix-like: Use ps to get process info
        command = 'ps aux | grep -E "(node|npm|yarn|pnpm)" | grep -v grep';
        parseOutput = this.parseUnixProcessOutput.bind(this);
      }

      const { stdout } = await execAsync(command);
      const allProcesses = parseOutput(stdout);
      
      // Filter processes that might be related to this workspace
      const projectName = path.basename(workspacePath).toLowerCase();
      const workspacePathLower = workspacePath.toLowerCase();
      
      for (const proc of allProcesses) {
        if (this.isProjectRelated(proc, workspacePathLower, projectName)) {
          processes.push(proc);
          this.runningProcesses.set(proc.pid.toString(), proc);
        }
      }

      this.saveProcessHistory();
      return processes;
    } catch (error) {
      console.warn('[PROCESS_MANAGER] Error finding Node processes:', error);
      return processes;
    }
  }

  private parseWindowsProcessOutput(output: string): RunningProcess[] {
    const processes: RunningProcess[] = [];
    const lines = output.split('\n').slice(1); // Skip header
    
    for (const line of lines) {
      if (!line.trim()) continue;
      
      const parts = line.split(',');
      if (parts.length < 3) continue;
      
      try {
        const commandLine = parts[1]?.trim();
        const pid = parseInt(parts[2]?.trim());
        const creationDate = parts[3]?.trim();
        
        if (!commandLine || !pid || isNaN(pid)) continue;
        
        processes.push({
          pid,
          command: commandLine,
          startTime: creationDate ? new Date(creationDate) : new Date(),
          type: this.classifyProcessType(commandLine)
        });
      } catch (error) {
        // Skip malformed lines
        continue;
      }
    }
    
    return processes;
  }

  private parseUnixProcessOutput(output: string): RunningProcess[] {
    const processes: RunningProcess[] = [];
    const lines = output.split('\n');
    
    for (const line of lines) {
      if (!line.trim()) continue;
      
      try {
        const parts = line.trim().split(/\s+/);
        if (parts.length < 11) continue;
        
        const pid = parseInt(parts[1]);
        if (isNaN(pid)) continue;
        
        const command = parts.slice(10).join(' ');
        
        processes.push({
          pid,
          command,
          startTime: new Date(), // Unix ps doesn't easily give creation time
          type: this.classifyProcessType(command)
        });
      } catch (error) {
        // Skip malformed lines
        continue;
      }
    }
    
    return processes;
  }

  private isProjectRelated(proc: RunningProcess, workspacePathLower: string, projectName: string): boolean {
    const commandLower = proc.command.toLowerCase();
    
    // Check if command contains workspace path
    if (commandLower.includes(workspacePathLower)) {
      return true;
    }
    
    // Check if command contains project name
    if (commandLower.includes(projectName)) {
      return true;
    }
    
    // Check for common dev server patterns
    const devPatterns = [
      'vite',
      'webpack-dev-server',
      'next dev',
      'react-scripts start',
      'vue-cli-service serve',
      'ng serve',
      'nuxt dev',
      'gatsby develop',
      'npm run dev',
      'yarn dev',
      'pnpm dev'
    ];
    
    return devPatterns.some(pattern => commandLower.includes(pattern));
  }

  private classifyProcessType(command: string): 'dev-server' | 'build' | 'test' | 'other' {
    const commandLower = command.toLowerCase();
    
    if (commandLower.includes('dev') || commandLower.includes('serve') || commandLower.includes('start')) {
      return 'dev-server';
    }
    
    if (commandLower.includes('build') || commandLower.includes('compile')) {
      return 'build';
    }
    
    if (commandLower.includes('test') || commandLower.includes('jest') || commandLower.includes('vitest')) {
      return 'test';
    }
    
    return 'other';
  }

  /**
   * Check for processes using common development ports
   */
  private async checkPortConflicts(): Promise<number[]> {
    const conflictingPorts: number[] = [];
    
    try {
      // Check ports in parallel with timeout per port
      const portChecks = this.commonDevPorts.map(port =>
        Promise.race([
          this.isPortInUse(port),
          new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 2000))
        ]).then(isInUse => isInUse ? port : null)
      );
      
      const results = await Promise.all(portChecks);
      return results.filter((port): port is number => port !== null);
    } catch (error) {
      console.warn('[PROCESS_MANAGER] Error checking port conflicts:', error);
    }
    
    return conflictingPorts;
  }

  private async isPortInUse(port: number): Promise<boolean> {
    try {
      let command: string;
      
      if (process.platform === 'win32') {
        command = `netstat -an | findstr :${port}`;
      } else {
        command = `lsof -i :${port}`;
      }
      
      const { stdout } = await execAsync(command);
      return stdout.trim().length > 0;
    } catch (error) {
      // Command failed, assume port is not in use
      return false;
    }
  }

  /**
   * Stop running processes
   */
  async stopProcesses(processes: RunningProcess[]): Promise<{ stopped: number; failed: number; errors: string[] }> {
    const result = { stopped: 0, failed: 0, errors: [] as string[] };
    
    for (const proc of processes) {
      try {
        await this.stopProcess(proc.pid);
        this.runningProcesses.delete(proc.pid.toString());
        result.stopped++;
        console.log(`[PROCESS_MANAGER] Stopped process ${proc.pid}: ${proc.command}`);
      } catch (error) {
        result.failed++;
        const errorMsg = `Failed to stop process ${proc.pid}: ${error}`;
        result.errors.push(errorMsg);
        console.error(`[PROCESS_MANAGER] ${errorMsg}`);
      }
    }
    
    this.saveProcessHistory();
    return result;
  }

  private async stopProcess(pid: number): Promise<void> {
    return new Promise((resolve, reject) => {
      let command: string;
      
      if (process.platform === 'win32') {
        command = `taskkill /F /PID ${pid}`;
      } else {
        command = `kill -TERM ${pid}`;
      }
      
      exec(command, (error, _stdout, _stderr) => {
        if (error) {
          // Try force kill on Unix systems
          if (process.platform !== 'win32') {
            exec(`kill -KILL ${pid}`, (killError) => {
              if (killError) {
                reject(new Error(`Failed to kill process ${pid}: ${killError.message}`));
              } else {
                resolve();
              }
            });
          } else {
            reject(new Error(`Failed to terminate process ${pid}: ${error.message}`));
          }
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * Clean up processes that are no longer running
   */
  private async cleanupDeadProcesses(): Promise<void> {
    const deadProcesses: string[] = [];
    
    for (const [pidStr, proc] of this.runningProcesses.entries()) {
      const isAlive = await this.isProcessAlive(proc.pid);
      if (!isAlive) {
        deadProcesses.push(pidStr);
      }
    }
    
    for (const pidStr of deadProcesses) {
      this.runningProcesses.delete(pidStr);
    }
    
    if (deadProcesses.length > 0) {
      this.saveProcessHistory();
    }
  }

  private async isProcessAlive(pid: number): Promise<boolean> {
    try {
      let command: string;
      
      if (process.platform === 'win32') {
        command = `tasklist /FI "PID eq ${pid}" /FO CSV | findstr ${pid}`;
      } else {
        command = `kill -0 ${pid}`;
      }
      
      await execAsync(command);
      return true;
    } catch (error) {
      return false;
    }
  }

  /**
   * Get summary of current running processes
   */
  getRunningProcessesSummary(): string {
    if (this.runningProcesses.size === 0) {
      return 'No tracked processes currently running.';
    }
    
    const summary = Array.from(this.runningProcesses.values())
      .map(proc => `• PID ${proc.pid}: ${proc.type} - ${proc.command.substring(0, 80)}${proc.command.length > 80 ? '...' : ''}`)
      .join('\n');
    
    return `Currently tracked processes (${this.runningProcesses.size}):\n${summary}`;
  }

  /**
   * Clear all tracked processes (useful for cleanup)
   */
  clearTrackedProcesses(): void {
    this.runningProcesses.clear();
    this.saveProcessHistory();
  }
}