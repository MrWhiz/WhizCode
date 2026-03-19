// Tool Result Caching System for WhizCode
// Implements intelligent caching of tool results for performance optimization

import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import { app } from 'electron';

export interface CacheEntry {
  id: string;
  toolName: string;
  parameters: any;
  result: any;
  timestamp: Date;
  expiresAt?: Date;
  accessCount: number;
  lastAccessed: Date;
  size: number;
  hash: string;
  metadata: {
    workspacePath?: string;
    filePath?: string;
    fileHash?: string;
    dependencies?: string[];
    tags?: string[];
  };
}

export interface CacheStats {
  totalEntries: number;
  totalSize: number;
  hitRate: number;
  missRate: number;
  totalHits: number;
  totalMisses: number;
  oldestEntry?: Date;
  newestEntry?: Date;
  mostAccessedTool?: string;
}

export interface CacheConfig {
  maxSize: number; // Maximum cache size in bytes
  maxEntries: number; // Maximum number of entries
  defaultTTL: number; // Default time-to-live in milliseconds
  cleanupInterval: number; // Cleanup interval in milliseconds
  persistToDisk: boolean; // Whether to persist cache to disk
}

export class ToolResultCache {
  private cache: Map<string, CacheEntry> = new Map();
  private cachePath: string;
  private config: CacheConfig;
  private stats = {
    hits: 0,
    misses: 0,
    totalSize: 0
  };
  private cleanupTimer?: NodeJS.Timeout;

  // Tool-specific cache configurations
  private toolConfigs: Map<string, {
    ttl?: number;
    cacheable: boolean;
    invalidateOn?: string[];
    dependencies?: string[];
  }> = new Map();

  constructor(workspacePath?: string, config?: Partial<CacheConfig>) {
    const baseDir = workspacePath 
      ? path.join(workspacePath, '.whizcode', 'tool-cache')
      : path.join(app.getPath('userData'), 'tool-cache');
    
    this.cachePath = path.join(baseDir, 'cache.json');
    
    this.config = {
      maxSize: 100 * 1024 * 1024, // 100MB
      maxEntries: 10000,
      defaultTTL: 60 * 60 * 1000, // 1 hour
      cleanupInterval: 5 * 60 * 1000, // 5 minutes
      persistToDisk: true,
      ...config
    };

    this.initializeToolConfigs();
    this.ensureDirectories();
    
    if (this.config.persistToDisk) {
      this.loadCache();
    }
    
    this.startCleanupTimer();
  }

  private initializeToolConfigs() {
    // Configure caching behavior for different tools
    const configs = [
      {
        tool: 'read_file',
        config: {
          ttl: 30 * 1000, // 30 seconds
          cacheable: true,
          invalidateOn: ['write_file', 'edit_code', 'strReplace'],
          dependencies: ['file_content']
        }
      },
      {
        tool: 'list_directory',
        config: {
          ttl: 60 * 1000, // 1 minute
          cacheable: true,
          invalidateOn: ['write_file', 'delete_file', 'create_directory'],
          dependencies: ['directory_structure']
        }
      },
      {
        tool: 'getDiagnostics',
        config: {
          ttl: 10 * 1000, // 10 seconds
          cacheable: true,
          invalidateOn: ['write_file', 'edit_code', 'strReplace'],
          dependencies: ['file_content', 'project_config']
        }
      },
      {
        tool: 'grepSearch',
        config: {
          ttl: 5 * 60 * 1000, // 5 minutes
          cacheable: true,
          invalidateOn: ['write_file', 'edit_code', 'strReplace'],
          dependencies: ['file_content']
        }
      },
      {
        tool: 'fuzzy_find_file',
        config: {
          ttl: 2 * 60 * 1000, // 2 minutes
          cacheable: true,
          invalidateOn: ['write_file', 'delete_file', 'create_directory'],
          dependencies: ['directory_structure']
        }
      },
      {
        tool: 'readCode',
        config: {
          ttl: 60 * 1000, // 1 minute
          cacheable: true,
          invalidateOn: ['write_file', 'edit_code', 'strReplace'],
          dependencies: ['file_content']
        }
      },
      {
        tool: 'run_command',
        config: {
          ttl: 0, // No caching for commands by default
          cacheable: false,
          dependencies: ['system_state']
        }
      },
      {
        tool: 'semantic_rename',
        config: {
          ttl: 0, // No caching for destructive operations
          cacheable: false,
          invalidateOn: ['file_content', 'project_structure']
        }
      },
      {
        tool: 'smart_relocate',
        config: {
          ttl: 0, // No caching for destructive operations
          cacheable: false,
          invalidateOn: ['file_content', 'project_structure']
        }
      }
    ];

    configs.forEach(({ tool, config }) => {
      this.toolConfigs.set(tool, config);
    });
  }

  private ensureDirectories() {
    const dir = path.dirname(this.cachePath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
  }

  private loadCache() {
    try {
      if (fs.existsSync(this.cachePath)) {
        const data = JSON.parse(fs.readFileSync(this.cachePath, 'utf8'));
        
        data.entries.forEach((entry: any) => {
          entry.timestamp = new Date(entry.timestamp);
          entry.lastAccessed = new Date(entry.lastAccessed);
          if (entry.expiresAt) {
            entry.expiresAt = new Date(entry.expiresAt);
          }
          
          this.cache.set(entry.id, entry);
          this.stats.totalSize += entry.size;
        });

        if (data.stats) {
          this.stats.hits = data.stats.hits || 0;
          this.stats.misses = data.stats.misses || 0;
        }

        console.log(`[CACHE] Loaded ${this.cache.size} entries from disk`);
      }
    } catch (error) {
      console.warn('[CACHE] Failed to load cache from disk:', error);
    }
  }

  private saveCache() {
    if (!this.config.persistToDisk) return;

    try {
      const data = {
        entries: Array.from(this.cache.values()),
        stats: this.stats,
        timestamp: new Date()
      };

      fs.writeFileSync(this.cachePath, JSON.stringify(data, null, 2));
    } catch (error) {
      console.error('[CACHE] Failed to save cache to disk:', error);
    }
  }

  private startCleanupTimer() {
    this.cleanupTimer = setInterval(() => {
      this.cleanup();
    }, this.config.cleanupInterval);
  }

  private generateCacheKey(toolName: string, parameters: any, metadata?: any): string {
    const keyData = {
      tool: toolName,
      params: parameters,
      meta: metadata
    };
    
    const keyString = JSON.stringify(keyData, Object.keys(keyData).sort());
    return crypto.createHash('sha256').update(keyString).digest('hex');
  }

  private calculateSize(data: any): number {
    return Buffer.byteLength(JSON.stringify(data), 'utf8');
  }

  private isExpired(entry: CacheEntry): boolean {
    if (!entry.expiresAt) return false;
    return new Date() > entry.expiresAt;
  }

  private shouldCache(toolName: string): boolean {
    const config = this.toolConfigs.get(toolName);
    return config?.cacheable !== false;
  }

  private getTTL(toolName: string): number {
    const config = this.toolConfigs.get(toolName);
    return config?.ttl ?? this.config.defaultTTL;
  }

  private getFileHash(filePath: string): string | undefined {
    try {
      if (fs.existsSync(filePath)) {
        const content = fs.readFileSync(filePath);
        return crypto.createHash('md5').update(content).digest('hex');
      }
    } catch {
      // Ignore errors
    }
    return undefined;
  }

  // Public API Methods

  async get(toolName: string, parameters: any, metadata?: any): Promise<any | null> {
    if (!this.shouldCache(toolName)) {
      this.stats.misses++;
      return null;
    }

    const cacheKey = this.generateCacheKey(toolName, parameters, metadata);
    const entry = this.cache.get(cacheKey);

    if (!entry) {
      this.stats.misses++;
      return null;
    }

    // Check if expired
    if (this.isExpired(entry)) {
      this.cache.delete(cacheKey);
      this.stats.totalSize -= entry.size;
      this.stats.misses++;
      return null;
    }

    // Check file dependencies
    if (entry.metadata.filePath && entry.metadata.fileHash) {
      const currentHash = this.getFileHash(entry.metadata.filePath);
      if (currentHash !== entry.metadata.fileHash) {
        this.cache.delete(cacheKey);
        this.stats.totalSize -= entry.size;
        this.stats.misses++;
        return null;
      }
    }

    // Update access statistics
    entry.accessCount++;
    entry.lastAccessed = new Date();
    this.stats.hits++;

    console.log(`[CACHE] Hit for ${toolName}: ${cacheKey.substring(0, 8)}...`);
    return entry.result;
  }

  async set(toolName: string, parameters: any, result: any, metadata?: any): Promise<void> {
    if (!this.shouldCache(toolName)) {
      return;
    }

    const cacheKey = this.generateCacheKey(toolName, parameters, metadata);
    const size = this.calculateSize(result);
    const ttl = this.getTTL(toolName);
    const now = new Date();

    // Prepare metadata
    const entryMetadata = {
      workspacePath: metadata?.workspacePath,
      filePath: metadata?.filePath,
      fileHash: metadata?.filePath ? this.getFileHash(metadata.filePath) : undefined,
      dependencies: this.toolConfigs.get(toolName)?.dependencies || [],
      tags: metadata?.tags || [],
      ...metadata
    };

    const entry: CacheEntry = {
      id: cacheKey,
      toolName,
      parameters,
      result,
      timestamp: now,
      expiresAt: ttl > 0 ? new Date(now.getTime() + ttl) : undefined,
      accessCount: 1,
      lastAccessed: now,
      size,
      hash: cacheKey,
      metadata: entryMetadata
    };

    // Check if we need to make space
    await this.ensureSpace(size);

    // Remove existing entry if it exists
    const existingEntry = this.cache.get(cacheKey);
    if (existingEntry) {
      this.stats.totalSize -= existingEntry.size;
    }

    // Add new entry
    this.cache.set(cacheKey, entry);
    this.stats.totalSize += size;

    console.log(`[CACHE] Cached ${toolName}: ${cacheKey.substring(0, 8)}... (${size} bytes)`);

    // Save to disk periodically
    if (this.cache.size % 10 === 0) {
      this.saveCache();
    }
  }

  private async ensureSpace(requiredSize: number): Promise<void> {
    // Check size limit
    while (this.stats.totalSize + requiredSize > this.config.maxSize && this.cache.size > 0) {
      await this.evictLeastRecentlyUsed();
    }

    // Check entry count limit
    while (this.cache.size >= this.config.maxEntries) {
      await this.evictLeastRecentlyUsed();
    }
  }

  private async evictLeastRecentlyUsed(): Promise<void> {
    let oldestEntry: CacheEntry | null = null;
    let oldestKey: string | null = null;

    for (const [key, entry] of this.cache) {
      if (!oldestEntry || entry.lastAccessed < oldestEntry.lastAccessed) {
        oldestEntry = entry;
        oldestKey = key;
      }
    }

    if (oldestKey && oldestEntry) {
      this.cache.delete(oldestKey);
      this.stats.totalSize -= oldestEntry.size;
      console.log(`[CACHE] Evicted ${oldestEntry.toolName}: ${oldestKey.substring(0, 8)}...`);
    }
  }

  invalidate(toolName?: string, filePath?: string, tags?: string[]): number {
    let invalidatedCount = 0;
    const toDelete: string[] = [];

    for (const [key, entry] of this.cache) {
      let shouldInvalidate = false;

      // Invalidate by tool name
      if (toolName && entry.toolName === toolName) {
        shouldInvalidate = true;
      }

      // Invalidate by file path
      if (filePath && entry.metadata.filePath === filePath) {
        shouldInvalidate = true;
      }

      // Invalidate by tags
      if (tags && entry.metadata.tags) {
        const hasMatchingTag = tags.some(tag => entry.metadata.tags?.includes(tag));
        if (hasMatchingTag) {
          shouldInvalidate = true;
        }
      }

      // Check tool-specific invalidation rules
      if (!shouldInvalidate && toolName) {
        const config = this.toolConfigs.get(toolName);
        if (config?.invalidateOn) {
          const shouldInvalidateByRule = config.invalidateOn.some(rule => {
            // Check if this entry should be invalidated by the rule
            return entry.metadata.dependencies?.includes(rule);
          });
          if (shouldInvalidateByRule) {
            shouldInvalidate = true;
          }
        }
      }

      if (shouldInvalidate) {
        toDelete.push(key);
        this.stats.totalSize -= entry.size;
        invalidatedCount++;
      }
    }

    // Delete invalidated entries
    toDelete.forEach(key => this.cache.delete(key));

    if (invalidatedCount > 0) {
      console.log(`[CACHE] Invalidated ${invalidatedCount} entries`);
      this.saveCache();
    }

    return invalidatedCount;
  }

  cleanup(): number {
    let cleanedCount = 0;
    const now = new Date();
    const toDelete: string[] = [];

    for (const [key, entry] of this.cache) {
      if (this.isExpired(entry)) {
        toDelete.push(key);
        this.stats.totalSize -= entry.size;
        cleanedCount++;
      }
    }

    toDelete.forEach(key => this.cache.delete(key));

    if (cleanedCount > 0) {
      console.log(`[CACHE] Cleaned up ${cleanedCount} expired entries`);
      this.saveCache();
    }

    return cleanedCount;
  }

  clear(): void {
    this.cache.clear();
    this.stats.totalSize = 0;
    this.stats.hits = 0;
    this.stats.misses = 0;
    
    if (this.config.persistToDisk) {
      this.saveCache();
    }
    
    console.log('[CACHE] Cache cleared');
  }

  getStats(): CacheStats {
    const entries = Array.from(this.cache.values());
    const totalRequests = this.stats.hits + this.stats.misses;
    
    // Find most accessed tool
    const toolCounts = new Map<string, number>();
    entries.forEach(entry => {
      const current = toolCounts.get(entry.toolName) || 0;
      toolCounts.set(entry.toolName, current + entry.accessCount);
    });
    
    let mostAccessedTool: string | undefined;
    let maxAccess = 0;
    for (const [tool, count] of toolCounts) {
      if (count > maxAccess) {
        maxAccess = count;
        mostAccessedTool = tool;
      }
    }

    return {
      totalEntries: this.cache.size,
      totalSize: this.stats.totalSize,
      hitRate: totalRequests > 0 ? this.stats.hits / totalRequests : 0,
      missRate: totalRequests > 0 ? this.stats.misses / totalRequests : 0,
      totalHits: this.stats.hits,
      totalMisses: this.stats.misses,
      oldestEntry: entries.length > 0 ? new Date(Math.min(...entries.map(e => e.timestamp.getTime()))) : undefined,
      newestEntry: entries.length > 0 ? new Date(Math.max(...entries.map(e => e.timestamp.getTime()))) : undefined,
      mostAccessedTool
    };
  }

  getEntriesByTool(toolName: string): CacheEntry[] {
    return Array.from(this.cache.values()).filter(entry => entry.toolName === toolName);
  }

  getConfig(): CacheConfig {
    return { ...this.config };
  }

  updateConfig(newConfig: Partial<CacheConfig>): void {
    this.config = { ...this.config, ...newConfig };
    
    // Restart cleanup timer if interval changed
    if (newConfig.cleanupInterval && this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.startCleanupTimer();
    }
  }

  // Tool-specific cache management
  preloadCache(toolName: string, parametersList: any[], metadata?: any): Promise<void[]> {
    // Preload cache with common tool calls
    const promises = parametersList.map(async (parameters) => {
      const cached = await this.get(toolName, parameters, metadata);
      if (!cached) {
        // Would need to actually execute the tool to cache the result
        // This is a placeholder for the preloading logic
        console.log(`[CACHE] Would preload ${toolName} with params:`, parameters);
      }
    });
    
    return Promise.all(promises);
  }

  warmupCache(workspacePath: string): Promise<void> {
    // Warm up cache with common operations for a workspace
    return new Promise(async (resolve) => {
      try {
        console.log('[CACHE] Warming up cache for workspace...');
        
        // Common operations to preload
        const commonOperations = [
          { tool: 'list_directory', params: { path: workspacePath } },
          { tool: 'list_directory', params: { path: path.join(workspacePath, 'src') } },
          { tool: 'fuzzy_find_file', params: { query: 'package.json', workspacePath } },
          { tool: 'fuzzy_find_file', params: { query: 'tsconfig.json', workspacePath } },
        ];
        
        // This would need integration with the actual tool execution system
        console.log(`[CACHE] Would warm up ${commonOperations.length} common operations`);
        
        resolve();
      } catch (error) {
        console.error('[CACHE] Cache warmup failed:', error);
        resolve();
      }
    });
  }

  shutdown(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
    }
    
    if (this.config.persistToDisk) {
      this.saveCache();
    }
    
    console.log('[CACHE] Cache system shutdown');
  }
}