/**
 * Diagnostics Service - Lightweight Version
 * Basic error detection without external tool dependencies
 */

import * as fs from 'node:fs/promises';

export interface Diagnostic {
  file: string;
  line: number;
  column: number;
  message: string;
  severity: 'error' | 'warning';
  code?: string;
}

class DiagnosticsService {
  private cache: Map<string, { errors: Diagnostic[]; timestamp: number }> = new Map();
  private CACHE_TTL = 5000; // 5 seconds

  /**
   * Check a file for errors using lightweight checks
   */
  async checkFile(filePath: string, workspacePath: string, content?: string): Promise<Diagnostic[]> {
    try {
      const ext = filePath.split('.').pop()?.toLowerCase();
      let diagnostics: Diagnostic[] = [];

      // Use provided content or read from file
      const fileContent = content !== undefined ? content : await fs.readFile(filePath, 'utf-8');

      // Check based on file type - lightweight checks only
      if (ext === 'json') {
        diagnostics = this.checkJSON(filePath, fileContent);
      } else if (ext === 'ts' || ext === 'tsx' || ext === 'js' || ext === 'jsx') {
        diagnostics = this.checkJavaScript(filePath, fileContent);
      }

      return diagnostics;
    } catch (error) {
      console.error(`[DIAGNOSTICS] Error checking file ${filePath}:`, error);
      return [];
    }
  }

  /**
   * Check JavaScript/TypeScript file for basic syntax errors
   */
  private checkJavaScript(filePath: string, content: string): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];
    const lines = content.split('\n');

    try {
      // Basic bracket matching
      let braceCount = 0;
      let bracketCount = 0;
      let parenCount = 0;

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        
        for (let j = 0; j < line.length; j++) {
          const char = line[j];
          const prevChar = j > 0 ? line[j - 1] : '';
          
          // Skip strings and comments
          if (char === '"' || char === "'" || char === '`') {
            // Simple string detection - skip to end of string
            const stringChar = char;
            j++;
            while (j < line.length && line[j] !== stringChar && line[j - 1] !== '\\') {
              j++;
            }
            continue;
          }
          
          if (char === '/' && line[j + 1] === '/') {
            // Skip line comments
            break;
          }

          // Count brackets
          if (char === '{') braceCount++;
          if (char === '}') braceCount--;
          if (char === '[') bracketCount++;
          if (char === ']') bracketCount--;
          if (char === '(') parenCount++;
          if (char === ')') parenCount--;
        }

        // Check for unmatched brackets at end of line
        if (braceCount < 0 || bracketCount < 0 || parenCount < 0) {
          diagnostics.push({
            file: filePath,
            line: i + 1,
            column: 1,
            message: 'Unmatched closing bracket',
            severity: 'error',
            code: 'syntax-error'
          });
          braceCount = Math.max(0, braceCount);
          bracketCount = Math.max(0, bracketCount);
          parenCount = Math.max(0, parenCount);
        }
      }

      // Check for unclosed brackets at end of file
      if (braceCount > 0) {
        diagnostics.push({
          file: filePath,
          line: lines.length,
          column: 1,
          message: `Unclosed brace (${braceCount} remaining)`,
          severity: 'error',
          code: 'syntax-error'
        });
      }
      if (bracketCount > 0) {
        diagnostics.push({
          file: filePath,
          line: lines.length,
          column: 1,
          message: `Unclosed bracket (${bracketCount} remaining)`,
          severity: 'error',
          code: 'syntax-error'
        });
      }
      if (parenCount > 0) {
        diagnostics.push({
          file: filePath,
          line: lines.length,
          column: 1,
          message: `Unclosed parenthesis (${parenCount} remaining)`,
          severity: 'error',
          code: 'syntax-error'
        });
      }
    } catch (error) {
      console.warn(`[DIAGNOSTICS] JavaScript check failed:`, error);
    }

    return diagnostics;
  }

  /**
   * Check JSON file
   */
  private checkJSON(filePath: string, content: string): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];

    try {
      JSON.parse(content);
    } catch (error: any) {
      const match = error.message.match(/position (\d+)/);
      const position = match ? parseInt(match[1]) : 0;
      const lines = content.substring(0, position).split('\n');
      const line = lines.length;
      const column = lines[lines.length - 1].length + 1;

      diagnostics.push({
        file: filePath,
        line,
        column,
        message: error.message,
        severity: 'error',
        code: 'json-error'
      });
    }

    return diagnostics;
  }

  /**
   * Clear cache for a file
   */
  clearCache(filePath: string): void {
    this.cache.delete(filePath);
  }

  /**
   * Clear all cache
   */
  clearAllCache(): void {
    this.cache.clear();
  }
}

export const diagnosticsService = new DiagnosticsService();
