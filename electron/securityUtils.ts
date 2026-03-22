import { resolve } from 'node:path';
import * as crypto from 'node:crypto';

/**
 * Validates that a file path is within the workspace boundary
 * Prevents path traversal attacks
 */
export function validatePathInWorkspace(filePath: string, workspacePath: string): void {
  const resolved = resolve(filePath);
  const workspaceResolved = resolve(workspacePath);
  
  if (!resolved.startsWith(workspaceResolved)) {
    throw new Error(`Path traversal attempt detected: ${filePath} is outside workspace`);
  }
}

/**
 * Sanitizes user input for shell commands
 * Uses proper escaping for different platforms
 */
export function sanitizeShellInput(input: string): string {
  if (process.platform === 'win32') {
    // Windows: escape special characters
    return input.replace(/[&|<>^`]/g, '^$&').replace(/"/g, '\\"');
  } else {
    // Unix: use single quotes and escape single quotes
    return `'${input.replace(/'/g, "'\\''")}'`;
  }
}

/**
 * Encrypts sensitive data (like tokens) before storage
 */
export function encryptData(data: string, key?: string): string {
  const encryptionKey = key || process.env.WHIZCODE_ENCRYPTION_KEY || 'default-key-change-in-production';
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv('aes-256-cbc', crypto.scryptSync(encryptionKey, 'salt', 32), iv);
  
  let encrypted = cipher.update(data, 'utf8', 'hex');
  encrypted += cipher.final('hex');
  
  return iv.toString('hex') + ':' + encrypted;
}

/**
 * Decrypts sensitive data
 */
export function decryptData(encryptedData: string, key?: string): string {
  const encryptionKey = key || process.env.WHIZCODE_ENCRYPTION_KEY || 'default-key-change-in-production';
  const [ivHex, encrypted] = encryptedData.split(':');
  const iv = Buffer.from(ivHex, 'hex');
  const decipher = crypto.createDecipheriv('aes-256-cbc', crypto.scryptSync(encryptionKey, 'salt', 32), iv);
  
  let decrypted = decipher.update(encrypted, 'hex', 'utf8');
  decrypted += decipher.final('utf8');
  
  return decrypted;
}

/**
 * Validates that input is a string and not excessively long
 */
export function validateStringInput(input: any, maxLength: number = 10000): string {
  if (typeof input !== 'string') {
    throw new Error('Invalid input: expected string');
  }
  if (input.length > maxLength) {
    throw new Error(`Input exceeds maximum length of ${maxLength} characters`);
  }
  return input;
}

/**
 * Validates that input is a valid file path
 */
export function validateFilePath(input: any): string {
  const path = validateStringInput(input, 500);
  // Reject paths with suspicious patterns
  if (path.includes('..') || path.includes('\0') || path.startsWith('~')) {
    throw new Error('Invalid file path');
  }
  return path;
}
