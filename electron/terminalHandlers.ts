/**
 * Terminal IPC Handlers
 * Integrates TerminalManager with Electron IPC
 */

import { ipcMain, BrowserWindow } from 'electron';
import { TerminalManager } from './terminalManager';
import type { TerminalConfig, TerminalType } from './terminalManager';

export function setupTerminalHandlers(win: BrowserWindow | null, getWorkspacePath?: () => string | null) {
  const terminalManager = new TerminalManager();

  // Listen to terminal events and forward to renderer
  terminalManager.on('data', (id: string, data: string) => {
    win?.webContents.send('terminal:incomingData', data, id);
  });

  terminalManager.on('exit', (id: string, code: number) => {
    win?.webContents.send('terminal:exited', id, code);
  });

  terminalManager.on('error', (id: string, err: Error) => {
    win?.webContents.send('terminal:error', id, err.message);
  });

  // Create new terminal
  ipcMain.on('terminal:create', (_event, { id, type }: { id: string; type: TerminalType }) => {
    try {
      // Get current workspace path
      const workspacePath = getWorkspacePath?.() || undefined;
      
      // Create terminal with frontend-provided ID and workspace directory
      terminalManager.createTerminal({ type, cwd: workspacePath }, id);
      _event.sender.send('terminal:created', id, id);
    } catch (err: any) {
      console.error(`Failed to create terminal:`, err);
      _event.sender.send('terminal:error', id, err.message);
    }
  });

  // Write to terminal
  ipcMain.on('terminal:keystroke', (_event, data: string, terminalId: string) => {
    try {
      const terminal = terminalManager.getTerminal(terminalId);
      if (!terminal) {
        return;
      }
      terminalManager.write(terminalId, data);
    } catch (err) {
      console.error(`Terminal write error for ${terminalId}:`, err);
    }
  });

  // Resize terminal
  ipcMain.on('terminal:resize', (_event, cols: number, rows: number, terminalId: string) => {
    try {
      if (cols > 0 && rows > 0) {
        terminalManager.resize(terminalId, cols, rows);
      }
    } catch (err) {
      console.error('Terminal resize error:', err);
    }
  });

  // Close terminal
  ipcMain.on('terminal:close', (_event, terminalId: string) => {
    try {
      terminalManager.kill(terminalId);
    } catch (err) {
      console.error('Terminal close error:', err);
    }
  });

  // Open link in browser
  ipcMain.on('terminal:openLink', (_event, uri: string) => {
    try {
      const { shell } = require('electron');
      shell.openExternal(uri);
    } catch (err) {
      console.error('Failed to open link:', err);
    }
  });

  // Get available shells
  ipcMain.handle('terminal:getAvailableShells', () => {
    return terminalManager.getAvailableShells();
  });

  // Get default shell
  ipcMain.handle('terminal:getDefaultShell', () => {
    return terminalManager.getDefaultShell();
  });

  // Get all terminals
  ipcMain.handle('terminal:getAll', () => {
    return terminalManager.getAllTerminals().map(t => ({
      id: t.id,
      type: t.type,
      cols: t.cols,
      rows: t.rows,
      cwd: t.cwd,
      createdAt: t.createdAt
    }));
  });

  // Kill all terminals
  ipcMain.handle('terminal:killAll', () => {
    terminalManager.clearAll();
    return true;
  });

  return terminalManager;
}
