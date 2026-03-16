import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export interface KnowledgeItem {
  topic: string;
  content: string;
  updatedAt: string;
}

export class MemoryManager {
  private memoryDir: string;

  constructor(workspaceRoot: string) {
    this.memoryDir = join(workspaceRoot, '.kiro', 'memory');
  }

  async initialize() {
    await fs.mkdir(this.memoryDir, { recursive: true });
  }

  async learnFact(topic: string, content: string): Promise<boolean> {
    try {
      const slug = topic.toLowerCase().replace(/[^a-z0-9]+/g, '-');
      const filePath = join(this.memoryDir, `${slug}.md`);
      const now = new Date().toISOString();
      const fileContent = `---
topic: ${topic}
updatedAt: ${now}
---

${content}`;
      await fs.writeFile(filePath, fileContent, 'utf-8');
      return true;
    } catch (e) {
      console.error('Failed to learn fact:', e);
      return false;
    }
  }

  async getMemories(): Promise<KnowledgeItem[]> {
    try {
      const files = await fs.readdir(this.memoryDir).catch(() => []);
      const memories: KnowledgeItem[] = [];
      for (const file of files) {
        if (!file.endsWith('.md')) continue;
        const raw = await fs.readFile(join(this.memoryDir, file), 'utf-8');
        const match = raw.match(/^---\ntopic:\s*(.+)\nupdatedAt:\s*(.+)\n---\n\n([\s\S]*)$/);
        if (match) {
          memories.push({
            topic: match[1].trim(),
            updatedAt: match[2].trim(),
            content: match[3].trim()
          });
        }
      }
      return memories;
    } catch (e) {
      return [];
    }
  }

  async buildMemoryContext(): Promise<string> {
    const memories = await this.getMemories();
    if (memories.length === 0) return '';
    let context = '\n<learned_knowledge>\n';
    for (const ki of memories) {
      context += `\n### ${ki.topic}\n${ki.content}\n`;
    }
    context += '</learned_knowledge>\n';
    return context;
  }
}
