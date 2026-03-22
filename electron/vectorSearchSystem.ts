// Vector Search System for WhizCode
// Implements semantic code search and context retrieval using embeddings

import * as fs from 'fs';
import * as path from 'path';
import { app } from 'electron';

export interface CodeChunk {
  id: string;
  content: string;
  filePath: string;
  startLine: number;
  endLine: number;
  type: 'function' | 'class' | 'method' | 'variable' | 'comment' | 'import' | 'block';
  language: string;
  embedding?: number[];
  metadata: {
    symbolName?: string;
    scope?: string;
    complexity?: number;
    dependencies?: string[];
    lastModified: Date;
  };
}

export interface SearchResult {
  chunk: CodeChunk;
  similarity: number;
  relevanceScore: number;
  context: {
    beforeLines: string[];
    afterLines: string[];
  };
}

export interface SemanticQuery {
  text: string;
  type?: 'code' | 'comment' | 'function' | 'class' | 'any';
  language?: string;
  filePath?: string;
  maxResults?: number;
  minSimilarity?: number;
}

export interface ContextRecommendation {
  type: 'similar_code' | 'related_function' | 'dependency' | 'usage_example';
  description: string;
  filePath: string;
  lineNumber: number;
  confidence: number;
  snippet: string;
}

export class VectorSearchSystem {
  private indexPath: string;
  private chunks: Map<string, CodeChunk> = new Map();
  private embeddings: Map<string, number[]> = new Map();
  private fileIndex: Map<string, string[]> = new Map(); // file -> chunk IDs
  private symbolIndex: Map<string, string[]> = new Map(); // symbol -> chunk IDs
  private isIndexing: boolean = false;
  private lastIndexTime: Date | null = null;

  constructor(workspacePath?: string) {
    const baseDir = workspacePath 
      ? path.join(workspacePath, '.whizcode', 'vector-index')
      : path.join(app.getPath('userData'), 'vector-index');
    
    this.indexPath = baseDir;
    this.ensureDirectories();
    this.loadIndex();
  }

  private ensureDirectories() {
    if (!fs.existsSync(this.indexPath)) {
      fs.mkdirSync(this.indexPath, { recursive: true });
    }
  }

  private loadIndex() {
    try {
      const chunksPath = path.join(this.indexPath, 'chunks.json');
      const embeddingsPath = path.join(this.indexPath, 'embeddings.json');
      const metadataPath = path.join(this.indexPath, 'metadata.json');

      if (fs.existsSync(chunksPath)) {
        const chunksData = JSON.parse(fs.readFileSync(chunksPath, 'utf8'));
        chunksData.forEach((chunk: any) => {
          chunk.metadata.lastModified = new Date(chunk.metadata.lastModified);
          this.chunks.set(chunk.id, chunk);
        });
      }

      if (fs.existsSync(embeddingsPath)) {
        const embeddingsData = JSON.parse(fs.readFileSync(embeddingsPath, 'utf8'));
        Object.entries(embeddingsData).forEach(([id, embedding]) => {
          this.embeddings.set(id, embedding as number[]);
        });
      }

      if (fs.existsSync(metadataPath)) {
        const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
        this.lastIndexTime = metadata.lastIndexTime ? new Date(metadata.lastIndexTime) : null;
        
        if (metadata.fileIndex) {
          Object.entries(metadata.fileIndex).forEach(([file, chunkIds]) => {
            this.fileIndex.set(file, chunkIds as string[]);
          });
        }
        
        if (metadata.symbolIndex) {
          Object.entries(metadata.symbolIndex).forEach(([symbol, chunkIds]) => {
            this.symbolIndex.set(symbol, chunkIds as string[]);
          });
        }
      }

      console.log(`[VECTOR] Loaded ${this.chunks.size} chunks from index`);
    } catch (error) {
      console.warn('[VECTOR] Failed to load index:', error);
    }
  }

  private saveIndex() {
    try {
      const chunksPath = path.join(this.indexPath, 'chunks.json');
      const embeddingsPath = path.join(this.indexPath, 'embeddings.json');
      const metadataPath = path.join(this.indexPath, 'metadata.json');

      // Save chunks
      const chunksData = Array.from(this.chunks.values());
      fs.writeFileSync(chunksPath, JSON.stringify(chunksData, null, 2));

      // Save embeddings
      const embeddingsData = Object.fromEntries(this.embeddings);
      fs.writeFileSync(embeddingsPath, JSON.stringify(embeddingsData, null, 2));

      // Save metadata
      const metadata = {
        lastIndexTime: this.lastIndexTime,
        fileIndex: Object.fromEntries(this.fileIndex),
        symbolIndex: Object.fromEntries(this.symbolIndex)
      };
      fs.writeFileSync(metadataPath, JSON.stringify(metadata, null, 2));

      console.log(`[VECTOR] Saved ${this.chunks.size} chunks to index`);
    } catch (error) {
      console.error('[VECTOR] Failed to save index:', error);
    }
  }

  async indexWorkspace(workspacePath: string, options?: {
    forceReindex?: boolean;
    includePatterns?: string[];
    excludePatterns?: string[];
  }): Promise<void> {
    if (this.isIndexing) {
      console.log('[VECTOR] Indexing already in progress');
      return;
    }

    this.isIndexing = true;
    console.log('[VECTOR] Starting workspace indexing...');

    try {
      const files = await this.findCodeFiles(workspacePath, options);
      const totalFiles = files.length;
      let processedFiles = 0;

      // Clear existing index if force reindex
      if (options?.forceReindex) {
        this.chunks.clear();
        this.embeddings.clear();
        this.fileIndex.clear();
        this.symbolIndex.clear();
      }

      for (const filePath of files) {
        try {
          await this.indexFile(filePath);
          processedFiles++;
          
          if (processedFiles % 10 === 0) {
            console.log(`[VECTOR] Processed ${processedFiles}/${totalFiles} files`);
          }
        } catch (error) {
          console.warn(`[VECTOR] Failed to index file ${filePath}:`, error);
        }
      }

      this.lastIndexTime = new Date();
      this.saveIndex();
      
      console.log(`[VECTOR] Indexing complete: ${processedFiles}/${totalFiles} files, ${this.chunks.size} chunks`);
    } catch (error) {
      console.error('[VECTOR] Indexing failed:', error);
      throw error;
    } finally {
      this.isIndexing = false;
    }
  }

  private async findCodeFiles(workspacePath: string, options?: {
    includePatterns?: string[];
    excludePatterns?: string[];
  }): Promise<string[]> {
    const files: string[] = [];
    const codeExtensions = ['.ts', '.tsx', '.js', '.jsx', '.py', '.java', '.cpp', '.c', '.cs', '.go', '.rs', '.php', '.rb'];
    
    const defaultExcludePatterns = [
      'node_modules',
      '.git',
      'dist',
      'build',
      '.next',
      '__pycache__',
      '.venv',
      'venv',
      '.cache',
      'coverage'
    ];

    const excludePatterns = [...defaultExcludePatterns, ...(options?.excludePatterns || [])];

    const walkDir = (dir: string, depth: number = 0) => {
      if (depth > 10) return; // Prevent infinite recursion

      try {
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        
        for (const entry of entries) {
          const fullPath = path.join(dir, entry.name);
          
          if (entry.isDirectory()) {
            // Skip excluded directories
            if (excludePatterns.some(pattern => entry.name.includes(pattern))) {
              continue;
            }
            walkDir(fullPath, depth + 1);
          } else if (entry.isFile()) {
            const ext = path.extname(entry.name).toLowerCase();
            if (codeExtensions.includes(ext)) {
              // Check include patterns if specified
              if (options?.includePatterns) {
                const matches = options.includePatterns.some(pattern => 
                  fullPath.includes(pattern) || entry.name.includes(pattern)
                );
                if (!matches) continue;
              }
              
              files.push(fullPath);
            }
          }
        }
      } catch (error) {
        console.warn(`[VECTOR] Failed to read directory ${dir}:`, error);
      }
    };

    walkDir(workspacePath);
    return files;
  }

  private async indexFile(filePath: string): Promise<void> {
    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const language = this.detectLanguage(filePath);
      const chunks = this.chunkFile(content, filePath, language);
      
      // Remove old chunks for this file
      const oldChunkIds = this.fileIndex.get(filePath) || [];
      oldChunkIds.forEach(id => {
        this.chunks.delete(id);
        this.embeddings.delete(id);
      });

      // Add new chunks
      const newChunkIds: string[] = [];
      
      for (const chunk of chunks) {
        // Generate embedding for the chunk
        const embedding = await this.generateEmbedding(chunk.content);
        chunk.embedding = embedding;
        
        this.chunks.set(chunk.id, chunk);
        this.embeddings.set(chunk.id, embedding);
        newChunkIds.push(chunk.id);
        
        // Update symbol index
        if (chunk.metadata.symbolName) {
          const symbolChunks = this.symbolIndex.get(chunk.metadata.symbolName) || [];
          symbolChunks.push(chunk.id);
          this.symbolIndex.set(chunk.metadata.symbolName, symbolChunks);
        }
      }
      
      this.fileIndex.set(filePath, newChunkIds);
    } catch (error) {
      console.warn(`[VECTOR] Failed to index file ${filePath}:`, error);
    }
  }

  private chunkFile(content: string, filePath: string, language: string): CodeChunk[] {
    const chunks: CodeChunk[] = [];
    const lines = content.split('\n');
    
    // Simple chunking strategy - can be enhanced with AST parsing
    let currentChunk = '';
    let startLine = 0;
    let chunkType: CodeChunk['type'] = 'block';
    let symbolName: string | undefined;
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmedLine = line.trim();
      
      // Detect chunk boundaries and types
      if (this.isChunkBoundary(line, language)) {
        // Save previous chunk if it has content
        if (currentChunk.trim()) {
          chunks.push(this.createChunk(
            currentChunk,
            filePath,
            startLine,
            i - 1,
            chunkType,
            language,
            symbolName
          ));
        }
        
        // Start new chunk
        currentChunk = line + '\n';
        startLine = i;
        chunkType = this.detectChunkType(line, language);
        symbolName = this.extractSymbolName(line, language);
      } else {
        currentChunk += line + '\n';
      }
      
      // Create chunk if it gets too large
      if (currentChunk.length > 2000) {
        chunks.push(this.createChunk(
          currentChunk,
          filePath,
          startLine,
          i,
          chunkType,
          language,
          symbolName
        ));
        
        currentChunk = '';
        startLine = i + 1;
        chunkType = 'block';
        symbolName = undefined;
      }
    }
    
    // Add final chunk
    if (currentChunk.trim()) {
      chunks.push(this.createChunk(
        currentChunk,
        filePath,
        startLine,
        lines.length - 1,
        chunkType,
        language,
        symbolName
      ));
    }
    
    return chunks;
  }

  private createChunk(
    content: string,
    filePath: string,
    startLine: number,
    endLine: number,
    type: CodeChunk['type'],
    language: string,
    symbolName?: string
  ): CodeChunk {
    const id = `${filePath}:${startLine}-${endLine}:${Date.now()}`;
    
    return {
      id,
      content: content.trim(),
      filePath,
      startLine,
      endLine,
      type,
      language,
      metadata: {
        symbolName,
        scope: this.detectScope(content, language),
        complexity: this.calculateComplexity(content, language),
        dependencies: this.extractDependencies(content, language),
        lastModified: new Date()
      }
    };
  }

  private isChunkBoundary(line: string, language: string): boolean {
    const trimmed = line.trim();
    
    // Common patterns that indicate chunk boundaries
    const patterns = [
      /^(export\s+)?(async\s+)?function\s+\w+/,
      /^(export\s+)?(default\s+)?class\s+\w+/,
      /^(export\s+)?interface\s+\w+/,
      /^(export\s+)?type\s+\w+/,
      /^(export\s+)?enum\s+\w+/,
      /^import\s+/,
      /^\/\*\*/, // JSDoc comments
      /^\/\/ ===/, // Section comments
    ];
    
    return patterns.some(pattern => pattern.test(trimmed));
  }

  private detectChunkType(line: string, language: string): CodeChunk['type'] {
    const trimmed = line.trim();
    
    if (trimmed.startsWith('import') || trimmed.startsWith('from')) return 'import';
    if (trimmed.includes('function')) return 'function';
    if (trimmed.includes('class')) return 'class';
    if (trimmed.startsWith('//') || trimmed.startsWith('/*')) return 'comment';
    if (trimmed.includes('const') || trimmed.includes('let') || trimmed.includes('var')) return 'variable';
    
    return 'block';
  }

  private extractSymbolName(line: string, language: string): string | undefined {
    const patterns = [
      /(?:function\s+)(\w+)/,
      /(?:class\s+)(\w+)/,
      /(?:interface\s+)(\w+)/,
      /(?:type\s+)(\w+)/,
      /(?:enum\s+)(\w+)/,
      /(?:const\s+)(\w+)/,
      /(?:let\s+)(\w+)/,
      /(?:var\s+)(\w+)/,
    ];
    
    for (const pattern of patterns) {
      const match = line.match(pattern);
      if (match) return match[1];
    }
    
    return undefined;
  }

  private detectScope(content: string, language: string): string {
    // Simple scope detection - can be enhanced
    if (content.includes('export')) return 'global';
    if (content.includes('private')) return 'private';
    if (content.includes('protected')) return 'protected';
    if (content.includes('public')) return 'public';
    return 'local';
  }

  private calculateComplexity(content: string, language: string): number {
    // Simple cyclomatic complexity calculation
    const wordKeywords = ['if', 'else', 'while', 'for', 'switch', 'case', 'catch'];
    const operatorKeywords = ['&&', '||', '?'];
    let complexity = 1;
    
    // Count word-bounded keywords
    for (const keyword of wordKeywords) {
      const regex = new RegExp(`\\b${keyword}\\b`, 'g');
      const matches = content.match(regex);
      if (matches) complexity += matches.length;
    }
    
    // Count operator keywords (without word boundaries)
    for (const operator of operatorKeywords) {
      const escapedOperator = operator.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(escapedOperator, 'g');
      const matches = content.match(regex);
      if (matches) complexity += matches.length;
    }
    
    return complexity;
  }

  private extractDependencies(content: string, language: string): string[] {
    const dependencies: string[] = [];
    
    // Extract imports
    const importRegex = /import\s+.*?from\s+['"]([^'"]+)['"]/g;
    let match;
    while ((match = importRegex.exec(content)) !== null) {
      dependencies.push(match[1]);
    }
    
    // Extract function calls (simplified)
    const callRegex = /(\w+)\s*\(/g;
    while ((match = callRegex.exec(content)) !== null) {
      if (!['if', 'for', 'while', 'switch', 'catch'].includes(match[1])) {
        dependencies.push(match[1]);
      }
    }
    
    return [...new Set(dependencies)]; // Remove duplicates
  }

  private detectLanguage(filePath: string): string {
    const ext = path.extname(filePath).toLowerCase();
    const langMap: Record<string, string> = {
      '.ts': 'typescript',
      '.tsx': 'typescript',
      '.js': 'javascript',
      '.jsx': 'javascript',
      '.py': 'python',
      '.java': 'java',
      '.cpp': 'cpp',
      '.c': 'c',
      '.cs': 'csharp',
      '.go': 'go',
      '.rs': 'rust',
      '.php': 'php',
      '.rb': 'ruby'
    };
    
    return langMap[ext] || 'plaintext';
  }

  private async generateEmbedding(text: string): Promise<number[]> {
    // Simplified embedding generation - in real implementation would use
    // a proper embedding model like sentence-transformers or OpenAI embeddings
    
    // For now, create a simple hash-based embedding
    const words = text.toLowerCase().match(/\w+/g) || [];
    const embedding = new Array(384).fill(0); // 384-dimensional embedding
    
    for (const word of words) {
      const hash = this.simpleHash(word);
      const index = Math.abs(hash) % embedding.length;
      embedding[index] += 1;
    }
    
    // Normalize
    const magnitude = Math.sqrt(embedding.reduce((sum, val) => sum + val * val, 0));
    if (magnitude > 0) {
      for (let i = 0; i < embedding.length; i++) {
        embedding[i] /= magnitude;
      }
    }
    
    return embedding;
  }

  private simpleHash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return hash;
  }

  private cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) return 0;
    
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    
    if (normA === 0 || normB === 0) return 0;
    
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  // Public API Methods

  async semanticSearch(query: SemanticQuery): Promise<SearchResult[]> {
    if (this.chunks.size === 0) {
      console.warn('[VECTOR] No chunks indexed for search');
      return [];
    }

    try {
      // Generate embedding for query
      const queryEmbedding = await this.generateEmbedding(query.text);
      
      // Find similar chunks
      const results: SearchResult[] = [];
      
      for (const [chunkId, chunk] of this.chunks) {
        // Apply filters
        if (query.type && chunk.type !== query.type && query.type !== 'any') continue;
        if (query.language && chunk.language !== query.language) continue;
        if (query.filePath && !chunk.filePath.includes(query.filePath)) continue;
        
        const chunkEmbedding = this.embeddings.get(chunkId);
        if (!chunkEmbedding) continue;
        
        const similarity = this.cosineSimilarity(queryEmbedding, chunkEmbedding);
        
        if (similarity >= (query.minSimilarity || 0.1)) {
          const context = this.getChunkContext(chunk);
          const relevanceScore = this.calculateRelevanceScore(chunk, query, similarity);
          
          results.push({
            chunk,
            similarity,
            relevanceScore,
            context
          });
        }
      }
      
      // Sort by relevance score
      results.sort((a, b) => b.relevanceScore - a.relevanceScore);
      
      // Limit results
      const maxResults = query.maxResults || 10;
      return results.slice(0, maxResults);
      
    } catch (error) {
      console.error('[VECTOR] Semantic search failed:', error);
      return [];
    }
  }

  async findSimilarCode(codeSnippet: string, options?: {
    language?: string;
    maxResults?: number;
    minSimilarity?: number;
  }): Promise<SearchResult[]> {
    return this.semanticSearch({
      text: codeSnippet,
      type: 'any',
      language: options?.language,
      maxResults: options?.maxResults || 5,
      minSimilarity: options?.minSimilarity || 0.3
    });
  }

  async getContextualRecommendations(context: string, filePath?: string): Promise<ContextRecommendation[]> {
    const recommendations: ContextRecommendation[] = [];
    
    try {
      // Find similar code
      const similarResults = await this.findSimilarCode(context, { maxResults: 3 });
      
      for (const result of similarResults) {
        recommendations.push({
          type: 'similar_code',
          description: `Similar ${result.chunk.type} found`,
          filePath: result.chunk.filePath,
          lineNumber: result.chunk.startLine,
          confidence: result.similarity,
          snippet: result.chunk.content.slice(0, 200) + '...'
        });
      }
      
      // Find related functions by symbol name
      if (filePath) {
        const fileChunks = this.fileIndex.get(filePath) || [];
        for (const chunkId of fileChunks) {
          const chunk = this.chunks.get(chunkId);
          if (chunk && chunk.metadata.symbolName) {
            const relatedChunks = this.symbolIndex.get(chunk.metadata.symbolName) || [];
            for (const relatedId of relatedChunks) {
              if (relatedId !== chunkId) {
                const relatedChunk = this.chunks.get(relatedId);
                if (relatedChunk) {
                  recommendations.push({
                    type: 'related_function',
                    description: `Related function: ${relatedChunk.metadata.symbolName}`,
                    filePath: relatedChunk.filePath,
                    lineNumber: relatedChunk.startLine,
                    confidence: 0.8,
                    snippet: relatedChunk.content.slice(0, 200) + '...'
                  });
                }
              }
            }
          }
        }
      }
      
      // Sort by confidence
      recommendations.sort((a, b) => b.confidence - a.confidence);
      
      return recommendations.slice(0, 10);
      
    } catch (error) {
      console.error('[VECTOR] Failed to get contextual recommendations:', error);
      return [];
    }
  }

  private getChunkContext(chunk: CodeChunk): { beforeLines: string[]; afterLines: string[] } {
    try {
      const content = fs.readFileSync(chunk.filePath, 'utf8');
      const lines = content.split('\n');
      
      const beforeStart = Math.max(0, chunk.startLine - 3);
      const afterEnd = Math.min(lines.length, chunk.endLine + 4);
      
      return {
        beforeLines: lines.slice(beforeStart, chunk.startLine),
        afterLines: lines.slice(chunk.endLine + 1, afterEnd)
      };
    } catch (error) {
      return { beforeLines: [], afterLines: [] };
    }
  }

  private calculateRelevanceScore(chunk: CodeChunk, query: SemanticQuery, similarity: number): number {
    let score = similarity;
    
    // Boost score based on chunk type match
    if (query.type && chunk.type === query.type) {
      score *= 1.2;
    }
    
    // Boost score for functions and classes
    if (chunk.type === 'function' || chunk.type === 'class') {
      score *= 1.1;
    }
    
    // Penalize very complex code
    if (chunk.metadata.complexity && chunk.metadata.complexity > 10) {
      score *= 0.9;
    }
    
    // Boost recent modifications
    const daysSinceModified = (Date.now() - chunk.metadata.lastModified.getTime()) / (1000 * 60 * 60 * 24);
    if (daysSinceModified < 7) {
      score *= 1.05;
    }
    
    return Math.min(score, 1.0);
  }

  // Index management methods

  getIndexStats(): {
    totalChunks: number;
    totalFiles: number;
    totalSymbols: number;
    lastIndexTime: Date | null;
    isIndexing: boolean;
  } {
    return {
      totalChunks: this.chunks.size,
      totalFiles: this.fileIndex.size,
      totalSymbols: this.symbolIndex.size,
      lastIndexTime: this.lastIndexTime,
      isIndexing: this.isIndexing
    };
  }

  async updateFileIndex(filePath: string): Promise<void> {
    if (fs.existsSync(filePath)) {
      await this.indexFile(filePath);
      this.saveIndex();
    } else {
      // File was deleted, remove from index
      this.removeFileFromIndex(filePath);
    }
  }

  private removeFileFromIndex(filePath: string): void {
    const chunkIds = this.fileIndex.get(filePath) || [];
    
    for (const chunkId of chunkIds) {
      const chunk = this.chunks.get(chunkId);
      if (chunk && chunk.metadata.symbolName) {
        const symbolChunks = this.symbolIndex.get(chunk.metadata.symbolName) || [];
        const filteredChunks = symbolChunks.filter(id => id !== chunkId);
        if (filteredChunks.length > 0) {
          this.symbolIndex.set(chunk.metadata.symbolName, filteredChunks);
        } else {
          this.symbolIndex.delete(chunk.metadata.symbolName);
        }
      }
      
      this.chunks.delete(chunkId);
      this.embeddings.delete(chunkId);
    }
    
    this.fileIndex.delete(filePath);
    this.saveIndex();
  }

  clearIndex(): void {
    this.chunks.clear();
    this.embeddings.clear();
    this.fileIndex.clear();
    this.symbolIndex.clear();
    this.lastIndexTime = null;
    
    // Remove index files
    try {
      const files = ['chunks.json', 'embeddings.json', 'metadata.json'];
      files.forEach(file => {
        const filePath = path.join(this.indexPath, file);
        if (fs.existsSync(filePath)) {
          fs.unlinkSync(filePath);
        }
      });
    } catch (error) {
      console.warn('[VECTOR] Failed to clear index files:', error);
    }
    
    console.log('[VECTOR] Index cleared');
  }
}