// Steering Files System for WhizCode
// Custom instructions and guidelines similar to WhizCode

import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export type SteeringInclusion = 'always' | 'fileMatch' | 'manual';

export interface SteeringFile {
  id: string;
  name: string;
  path: string;
  content: string;
  inclusion: SteeringInclusion;
  fileMatchPattern?: string; // Regex pattern for fileMatch inclusion
  enabled: boolean;
}

export interface SteeringFrontMatter {
  inclusion?: SteeringInclusion;
  fileMatchPattern?: string;
  enabled?: boolean;
}

export class SteeringManager {
  private steeringFiles: Map<string, SteeringFile> = new Map();
  private steeringDir: string;

  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
    this.steeringDir = join(workspaceRoot, '.whizcode', 'steering');
  }

  async initialize() {
    try {
      await fs.mkdir(this.steeringDir, { recursive: true});
      await this.loadSteeringFiles();
    } catch (e) {
      console.error('Failed to initialize steering:', e);
    }
  }

  async loadSteeringFiles() {
    try {
      const files = await fs.readdir(this.steeringDir);
      const mdFiles = files.filter(f => f.endsWith('.md'));

      for (const file of mdFiles) {
        try {
          const filePath = join(this.steeringDir, file);
          const content = await fs.readFile(filePath, 'utf-8');
          const steering = this.parseSteeringFile(file, filePath, content);
          this.steeringFiles.set(steering.id, steering);
        } catch (e) {
          console.error(`Failed to load steering file ${file}:`, e);
        }
      }

      console.log(`Loaded ${this.steeringFiles.size} steering files`);
    } catch (e) {
      // Steering directory doesn't exist yet
    }
  }

  parseSteeringFile(filename: string, filePath: string, content: string): SteeringFile {
    const id = filename.replace('.md', '');
    let frontMatter: SteeringFrontMatter = {};
    let mainContent = content;

    // Parse front matter (YAML-like)
    const frontMatterMatch = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
    if (frontMatterMatch) {
      const frontMatterText = frontMatterMatch[1];
      mainContent = frontMatterMatch[2];

      // Simple YAML parsing
      const lines = frontMatterText.split('\n');
      for (const line of lines) {
        const match = line.match(/^(\w+):\s*(.+)$/);
        if (match) {
          const [, key, value] = match;
          if (key === 'inclusion') {
            frontMatter.inclusion = value.trim() as SteeringInclusion;
          } else if (key === 'fileMatchPattern') {
            frontMatter.fileMatchPattern = value.trim().replace(/^['"]|['"]$/g, '');
          } else if (key === 'enabled') {
            frontMatter.enabled = value.trim().toLowerCase() === 'true';
          }
        }
      }
    }

    return {
      id,
      name: filename,
      path: filePath,
      content: mainContent,
      inclusion: frontMatter.inclusion || 'always',
      fileMatchPattern: frontMatter.fileMatchPattern,
      enabled: frontMatter.enabled !== false // Default to true
    };
  }

  async saveSteeringFile(steering: SteeringFile) {
    const filePath = join(this.steeringDir, `${steering.id}.md`);
    await fs.mkdir(this.steeringDir, { recursive: true });

    // Build front matter
    let content = '';
    if (steering.inclusion !== 'always' || steering.fileMatchPattern || steering.enabled === false) {
      content += '---\n';
      if (steering.inclusion !== 'always') {
        content += `inclusion: ${steering.inclusion}\n`;
      }
      if (steering.fileMatchPattern) {
        content += `fileMatchPattern: "${steering.fileMatchPattern}"\n`;
      }
      if (steering.enabled === false) {
        content += `enabled: false\n`;
      }
      content += '---\n\n';
    }

    content += steering.content;

    await fs.writeFile(filePath, content, 'utf-8');
    this.steeringFiles.set(steering.id, steering);
  }

  async deleteSteeringFile(steeringId: string) {
    const filePath = join(this.steeringDir, `${steeringId}.md`);
    try {
      await fs.unlink(filePath);
      this.steeringFiles.delete(steeringId);
    } catch (e) {
      console.error(`Failed to delete steering file ${steeringId}:`, e);
    }
  }

  getSteeringFile(steeringId: string): SteeringFile | undefined {
    return this.steeringFiles.get(steeringId);
  }

  getAllSteeringFiles(): SteeringFile[] {
    return Array.from(this.steeringFiles.values());
  }

  getEnabledSteeringFiles(): SteeringFile[] {
    return Array.from(this.steeringFiles.values()).filter(s => s.enabled);
  }

  getAlwaysIncludedSteering(): SteeringFile[] {
    return this.getEnabledSteeringFiles().filter(s => s.inclusion === 'always');
  }

  getFileMatchSteering(filePath: string): SteeringFile[] {
    return this.getEnabledSteeringFiles().filter(s => {
      if (s.inclusion !== 'fileMatch' || !s.fileMatchPattern) return false;
      try {
        const regex = new RegExp(s.fileMatchPattern);
        return regex.test(filePath);
      } catch (e) {
        return false;
      }
    });
  }

  getManualSteering(): SteeringFile[] {
    return this.getEnabledSteeringFiles().filter(s => s.inclusion === 'manual');
  }

  private async resolveFileReferences(content: string): Promise<string> {
    const refs = content.match(/#\[\[file:([^\]]+)\]\]/g);
    if (!refs) return content;

    let result = content;
    for (const ref of refs) {
      const pathPart = ref.match(/#\[\[file:([^\]]+)\]\]/)?.[1];
      if (pathPart) {
        try {
          const fullPath = join(this.workspaceRoot, pathPart);
          const fileContent = await fs.readFile(fullPath, 'utf-8');
          result = result.replace(ref, `\n<referenced_file path="${pathPart}">\n${fileContent}\n</referenced_file>\n`);
        } catch {
          result = result.replace(ref, `[Error: Referenced file ${pathPart} not found]`);
        }
      }
    }
    return result;
  }

  async buildSteeringContext(activeFilePath?: string): Promise<string> {
    let context = '';

    // Always included steering
    const alwaysSteering = this.getAlwaysIncludedSteering();
    if (alwaysSteering.length > 0) {
      context += '\n<steering_instructions>\n';
      for (const steering of alwaysSteering) {
        const resolvedContent = await this.resolveFileReferences(steering.content);
        context += `\n<!-- ${steering.name} -->\n`;
        context += resolvedContent + '\n';
      }
      context += '</steering_instructions>\n';
    }

    // File-match steering
    if (activeFilePath) {
      const fileMatchSteering = this.getFileMatchSteering(activeFilePath);
      if (fileMatchSteering.length > 0) {
        context += '\n<file_specific_instructions>\n';
        for (const steering of fileMatchSteering) {
          const resolvedContent = await this.resolveFileReferences(steering.content);
          context += `\n<!-- ${steering.name} (matched: ${activeFilePath}) -->\n`;
          context += resolvedContent + '\n';
        }
        context += '</file_specific_instructions>\n';
      }
    }

    return context;
  }
}

// Example steering files
export const EXAMPLE_STEERING_FILES = [
  {
    filename: 'coding-standards.md',
    content: `---
inclusion: always
---

# Coding Standards

## General Guidelines
- Use TypeScript for all new code
- Follow ESLint rules strictly
- Write descriptive variable names
- Add comments for complex logic

## Code Style
- Use 2 spaces for indentation
- Use single quotes for strings
- Add trailing commas in objects/arrays
- Use async/await instead of promises

## Testing
- Write tests for all new features
- Aim for 80%+ code coverage
- Use descriptive test names
`
  },
  {
    filename: 'react-guidelines.md',
    content: `---
inclusion: fileMatch
fileMatchPattern: ".*\\.(tsx|jsx)$"
---

# React Component Guidelines

## Component Structure
- Use functional components with hooks
- Keep components small and focused
- Extract reusable logic into custom hooks

## Props
- Define prop types with TypeScript interfaces
- Use destructuring for props
- Provide default values when appropriate

## State Management
- Use useState for local state
- Use useContext for shared state
- Consider useReducer for complex state

## Performance
- Use React.memo for expensive components
- Use useCallback for event handlers
- Use useMemo for expensive calculations
`
  },
  {
    filename: 'api-conventions.md',
    content: `---
inclusion: manual
---

# API Conventions

This steering file is manually included when working on API code.

## REST API Design
- Use RESTful conventions
- Use proper HTTP methods (GET, POST, PUT, DELETE)
- Return appropriate status codes
- Include error messages in responses

## Error Handling
- Use try-catch blocks
- Log errors with context
- Return user-friendly error messages
- Don't expose internal errors to clients

## Authentication
- Use JWT tokens
- Validate tokens on every request
- Handle token expiration gracefully
- Use HTTPS in production
`
  }
];
