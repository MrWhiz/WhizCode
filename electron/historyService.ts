import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export interface ChatThread {
  id: string;
  title: string;
  messages: any[];
  updatedAt: string;
}

export class HistoryManager {
  private historyDir: string;

  constructor(workspaceRoot: string) {
    this.historyDir = join(workspaceRoot, '.whizcode', 'history');
  }

  async initialize() {
    await fs.mkdir(this.historyDir, { recursive: true });
  }

  async saveThread(id: string, title: string, messages: any[]): Promise<string> {
    try {
      const now = new Date().toISOString();
      const filename = `${id}.json`;
      const filePath = join(this.historyDir, filename);
      
      const thread: ChatThread = {
        id,
        title,
        messages,
        updatedAt: now
      };

      await fs.writeFile(filePath, JSON.stringify(thread, null, 2), 'utf-8');
      return filePath;
    } catch (e) {
      console.error('Failed to save chat history:', e);
      return '';
    }
  }

  async listThreads(): Promise<Partial<ChatThread>[]> {
    try {
      const files = await fs.readdir(this.historyDir);
      const threads: Partial<ChatThread>[] = [];
      
      for (const file of files) {
        if (!file.endsWith('.json')) continue;
        const content = await fs.readFile(join(this.historyDir, file), 'utf-8');
        const data = JSON.parse(content);
        threads.push({
          id: data.id,
          title: data.title,
          updatedAt: data.updatedAt
        });
      }
      
      return threads.sort((a, b) => 
        new Date(b.updatedAt!).getTime() - new Date(a.updatedAt!).getTime()
      );
    } catch (e) {
      return [];
    }
  }

  async getThread(id: string): Promise<ChatThread | null> {
    try {
      const filePath = join(this.historyDir, `${id}.json`);
      const content = await fs.readFile(filePath, 'utf-8');
      return JSON.parse(content);
    } catch {
      return null;
    }
  }

  async deleteThread(id: string) {
    try {
      await fs.unlink(join(this.historyDir, `${id}.json`));
    } catch {}
  }
}
