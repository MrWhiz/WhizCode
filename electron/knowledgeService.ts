// Knowledge Service for WhizCode
// Manages creation, retrieval, and indexing of human-readable Knowledge Items (KIs)

import * as fs from 'fs';
import * as path from 'path';

export interface KnowledgeItem {
  id: string;
  title: string;
  summary: string;
  content: string;
  tags: string[];
  relatedFiles: string[];
  timestamp: Date;
}

export class KnowledgeService {
  private workspacePath: string;
  private knowledgeDir: string;

  constructor(workspacePath: string) {
    this.workspacePath = workspacePath;
    this.knowledgeDir = path.join(workspacePath, '.knowledge');
    this.ensureKnowledgeDirectory();
  }

  private ensureKnowledgeDirectory() {
    if (!fs.existsSync(this.knowledgeDir)) {
      try {
        fs.mkdirSync(this.knowledgeDir, { recursive: true });
        const readmeContent = '# WhizCode Knowledge Base\n\nThis directory contains distilled knowledge items (KIs) created by WhizCode during development. These items help maintain architectural context and learn from past implementations.';
        fs.writeFileSync(path.join(this.knowledgeDir, 'README.md'), readmeContent);
      } catch (error) {
        console.error('[KNOWLEDGE] Failed to create knowledge directory:', error);
      }
    }
  }

  /**
   * Saves a new knowledge item to the workspace
   */
  async saveKnowledgeItem(ki: Omit<KnowledgeItem, 'id' | 'timestamp'>): Promise<string> {
    const id = ki.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    const fileName = `${id}.md`;
    const filePath = path.join(this.knowledgeDir, fileName);
    const now = new Date();

    const fileContent = `---
id: ${id}
title: ${ki.title}
summary: ${ki.summary}
tags: [${ki.tags.join(', ')}]
relatedFiles: [${ki.relatedFiles.join(', ')}]
timestamp: ${now.toISOString()}
---

# ${ki.title}

${ki.content}
`;

    try {
      fs.writeFileSync(filePath, fileContent, 'utf8');
      return filePath;
    } catch (error) {
      console.error('[KNOWLEDGE] Failed to save knowledge item:', error);
      throw error;
    }
  }

  /**
   * Retrieves all knowledge items from the workspace (cached metadata only)
   */
  async listKnowledgeItems(): Promise<Omit<KnowledgeItem, 'content'>[]> {
    if (!fs.existsSync(this.knowledgeDir)) return [];

    try {
      const files = fs.readdirSync(this.knowledgeDir).filter(f => f.endsWith('.md') && f !== 'README.md');
      const items: Omit<KnowledgeItem, 'content'>[] = [];

      for (const file of files) {
        const filePath = path.join(this.knowledgeDir, file);
        const fileStat = fs.statSync(filePath);
        const data = fs.readFileSync(filePath, 'utf8');
        const metadata = this.parseKIMetadata(data);
        if (metadata) {
          items.push({
            id: metadata.id || file.replace('.md', ''),
            title: metadata.title || file.replace('.md', ''),
            summary: metadata.summary || '',
            tags: metadata.tags || [],
            relatedFiles: metadata.relatedFiles || [],
            timestamp: metadata.timestamp ? new Date(metadata.timestamp) : fileStat.mtime
          });
        }
      }

      return items;
    } catch (error) {
      console.error('[KNOWLEDGE] Failed to list knowledge items:', error);
      return [];
    }
  }

  /**
   * Reads a specific knowledge item
   */
  async getKnowledgeItem(id: string): Promise<KnowledgeItem | null> {
    const filePath = path.join(this.knowledgeDir, `${id}.md`);
    if (!fs.existsSync(filePath)) return null;

    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const metadata = this.parseKIMetadata(content);
      if (!metadata) return null;

      const bodyStart = content.indexOf('---', 3) + 3;
      const body = content.substring(bodyStart).trim();

      return {
        id: metadata.id || id,
        title: metadata.title || id,
        summary: metadata.summary || '',
        tags: metadata.tags || [],
        relatedFiles: metadata.relatedFiles || [],
        timestamp: new Date(metadata.timestamp),
        content: body
      };
    } catch (error) {
      console.error('[KNOWLEDGE] Failed to read knowledge item:', error);
      return null;
    }
  }

  private parseKIMetadata(content: string): any {
    const match = content.match(/^---([\s\S]*?)---/);
    if (!match) return null;

    const metadata: any = {};
    const yamlPart = match[1];
    const lines = yamlPart.split('\n');
    
    for (const line of lines) {
      const colonIndex = line.indexOf(':');
      if (colonIndex === -1) continue;
      
      const key = line.substring(0, colonIndex).trim();
      let value = line.substring(colonIndex + 1).trim();
      
      if (value.startsWith('[') && value.endsWith(']')) {
        metadata[key] = value.slice(1, -1).split(',').map(v => v.trim()).filter(v => v);
      } else {
        metadata[key] = value;
      }
    }
    return metadata;
  }

  /**
   * Semantic/Keyword hybrid search for relevant knowledge items
   */
  async searchKnowledge(query: string, maxResults: number = 3): Promise<KnowledgeItem[]> {
    const allItems = await this.listKnowledgeItems();
    if (allItems.length === 0) return [];
    
    const queryLower = query.toLowerCase();
    const queryWords = queryLower.split(/\W+/).filter(w => w.length > 2);
    
    const results = await Promise.all(allItems.map(async item => {
      let score = 0;
      const titleLower = item.title.toLowerCase();
      const summaryLower = item.summary.toLowerCase();
      
      // Title match
      if (titleLower.includes(queryLower)) score += 20;
      
      // Keyword matching
      for (const word of queryWords) {
        if (titleLower.includes(word)) score += 5;
        if (summaryLower.includes(word)) score += 3;
        if (item.tags.some(t => t.toLowerCase().includes(word))) score += 4;
      }
      
      // Full content match (more expensive)
      if (score > 0 || allItems.length < 10) {
        const full = await this.getKnowledgeItem(item.id);
        if (full) {
          const contentLower = full.content.toLowerCase();
          for (const word of queryWords) {
            if (contentLower.includes(word)) score += 1;
          }
          return { item: full, score };
        }
      }
      
      return { item: null, score: 0 };
    }));

    return results
      .filter(r => r.item !== null && r.score > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, maxResults)
      .map(r => r.item as KnowledgeItem);
  }

  /**
   * Generates a context block for the LLM from relevant knowledge items
   */
  async buildKnowledgeContext(query: string): Promise<string> {
    const relevant = await this.searchKnowledge(query, 3);
    if (relevant.length === 0) return '';

    let context = '\n<relevant_knowledge_items>\n';
    for (const ki of relevant) {
      context += `\n### KI: ${ki.title}\n`;
      context += `Summary: ${ki.summary}\n`;
      context += `Related: ${ki.relatedFiles.join(', ')}\n`;
      context += `Content:\n${ki.content}\n`;
    }
    context += '</relevant_knowledge_items>\n';
    return context;
  }
}
