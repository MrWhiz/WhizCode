import { join } from 'node:path';
import * as fs from 'node:fs/promises';
import { app } from 'electron';
import { VoyageAIClient } from 'voyageai';
import * as chokidar from 'chokidar';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';





const _require = createRequire(import.meta.url)
/** 
 * WE USE DYNAMIC REQUIRE FOR NATIVE MODULES TO PREVENT VITE FROM TRYING TO BUNDLE BINARY DATA 
 */
let treeSitter: any;
let treeSitterTypeScript: any;

try {
    treeSitter = _require('tree-sitter');
    treeSitterTypeScript = _require('tree-sitter-typescript');
} catch (e) {
    console.error("Failed to load native modules in IndexingService:", e);
}

// Types for our semantic blocks
export interface SemanticChunk {
    id: string;
    filePath: string;
    type: 'function' | 'class' | 'constant' | 'method' | 'other';
    name: string;
    content: string;
    startLine: number;
    endLine: number;
    hash: string;
    vector?: number[];
}

export class IndexingService {
    private dbPath: string = '';
    private records: SemanticChunk[] = [];
    private parser: any;
    private voyage: VoyageAIClient | null = null;
    private azureConfig: { loginUrl: string, embeddingUrl: string, username: string, password: string } | null = null;
    private getAzureToken: ((loginUrl: string, username: string, password: string) => Promise<string>) | null = null;
    private watcher: chokidar.FSWatcher | null = null;
    private workspacePath: string | null = null;
    private fileHashes: Map<string, string> = new Map(); // Merkle-like watcher
    private onChange: ((path: string) => void) | null = null;

    constructor(
        config: { voyageKey?: string, azure?: { loginUrl: string, embeddingUrl: string, username: string, password: string }, getToken?: (loginUrl: string, username: string, password: string) => Promise<string> }, 
        onChange?: (path: string) => void
    ) {
        this.parser = new treeSitter();
        this.onChange = onChange || null;
        // Default to TypeScript TSX for most React/TS projects
        this.parser.setLanguage(treeSitterTypeScript.tsx);

        if (config.voyageKey) {
            this.voyage = new VoyageAIClient({ apiKey: config.voyageKey });
        }
        if (config.azure) {
            this.azureConfig = config.azure;
            this.getAzureToken = config.getToken || null;
        }
    }

    private async loadDb() {
        if (!this.dbPath) return;
        try {
            const data = await fs.readFile(this.dbPath, 'utf-8');
            this.records = JSON.parse(data);
        } catch {
            this.records = []; // File doesn't exist or is invalid
        }
    }

    private async saveDb() {
        if (!this.dbPath) return;
        try {
            await fs.writeFile(this.dbPath, JSON.stringify(this.records), 'utf-8');
        } catch (e) {
            console.error('Failed to save vector DB:', e);
        }
    }

    async initialize(workspacePath: string) {
        this.workspacePath = workspacePath;
        const dbDir = join(app.getPath('userData'), 'vector_db');
        await fs.mkdir(dbDir, { recursive: true });
        
        // Use a hash of the workspace path to keep a separate JSON DB per project
        const projectHash = crypto.createHash('md5').update(workspacePath).digest('hex');
        this.dbPath = join(dbDir, `semantic_chunks_${projectHash}.json`);

        await this.loadDb();
        console.log(`Loaded ${this.records.length} chunks from Pure TS Vector DB`);

        this.setupWatcher();
    }

    private setupWatcher() {
        if (this.watcher) this.watcher.close();

        this.watcher = chokidar.watch(this.workspacePath!, {
            ignored: /(^|[\/\\])\../, // ignore dotfiles
            persistent: true,
            ignoreInitial: true
        });

        this.watcher.on('change', async (path: string) => {
            console.log(`File changed: ${path}`);
            await this.indexFile(path);
            if (this.onChange) this.onChange(path);
        });
    }

    async indexWorkspace() {
        if (!this.workspacePath) return;
        console.log(`Starting full indexing of ${this.workspacePath}`);

        const files = await this.getProjectFiles(this.workspacePath);
        for (const file of files) {
            await this.indexFile(file, false); // Pass false to delay saving until the end
        }
        await this.saveDb();
    }

    private async getProjectFiles(dir: string): Promise<string[]> {
        const results: string[] = [];
        const SKIP_DIRS = new Set(['node_modules', '.git', 'dist', 'dist-electron', 'build']);
        const VALID_EXTS = new Set(['.ts', '.tsx', '.js', '.jsx']);

        async function walk(currentPath: string) {
            const entries = await fs.readdir(currentPath, { withFileTypes: true });
            for (const entry of entries) {
                const fullPath = join(currentPath, entry.name);
                if (entry.isDirectory()) {
                    if (!SKIP_DIRS.has(entry.name)) {
                        await walk(fullPath);
                    }
                } else {
                    const ext = '.' + entry.name.split('.').pop()?.toLowerCase();
                    if (VALID_EXTS.has(ext)) {
                        results.push(fullPath);
                    }
                }
            }
        }

        await walk(dir);
        return results;
    }

    async indexFile(filePath: string, saveImmediately = true) {
        try {
            const content = await fs.readFile(filePath, 'utf-8');
            const fileHash = crypto.createHash('sha256').update(content).digest('hex');

            // If file hash hasn't changed, skip entirely
            if (this.fileHashes.get(filePath) === fileHash) {
                return;
            }

            const chunks = this.parseFile(filePath, content);
            const normalizedFilePath = filePath.replace(/\\/g, '\\\\');

            // Get existing hashes for this file from in-memory records
            const existingEntries = this.records.filter(r => r.filePath === filePath || r.filePath === normalizedFilePath);
            const existingHashes = new Map(existingEntries.map((e: SemanticChunk) => [e.id, e.hash]));

            // Filter chunks that actually changed
            const changedChunks = chunks.filter(c => existingHashes.get(c.id) !== c.hash);

            if (changedChunks.length > 0) {
                // Only generate embeddings for changed chunks
                const embeddings = await this.generateEmbeddings(changedChunks.map(c => c.content));

                const newRecords = changedChunks.map((chunk, i) => ({
                    ...chunk,
                    vector: embeddings[i]
                }));

                // Remove outdated chunks for these specific IDs
                const changedIds = new Set(changedChunks.map(c => c.id));
                this.records = this.records.filter(r => !changedIds.has(r.id));
                
                // Add new chunk data
                this.records.push(...newRecords);
            }

            // Cleanup chunks that no longer exist in the file
            const allCurrentIds = new Set(chunks.map(c => c.id));
            this.records = this.records.filter(r => {
                if (r.filePath === filePath || r.filePath === normalizedFilePath) {
                    return allCurrentIds.has(r.id); // Keep only if it still exists in parsed file
                }
                return true; // Keep records from other files
            });

            this.fileHashes.set(filePath, fileHash);
            
            if (saveImmediately) {
                await this.saveDb();
            }
        } catch (error) {
            console.error(`Error indexing file ${filePath}:`, error);
        }
    }

    private parseFile(filePath: string, content: string): SemanticChunk[] {
        const tree = this.parser.parse(content);
        const chunks: SemanticChunk[] = [];

        const query = new treeSitter.Query(treeSitterTypeScript.tsx, `
      (class_declaration name: (identifier) @class.name) @class.def
      (function_declaration name: (identifier) @function.name) @function.def
      (variable_declarator name: (identifier) @const.name value: (arrow_function)) @const.def
      (method_definition name: (property_identifier) @method.name) @method.def
    `);

        const captures = query.captures(tree.rootNode);

        for (let i = 0; i < captures.length; i++) {
            const capture = captures[i];
            if (capture.name.endsWith('.def')) {
                const node = capture.node;
                const nameNode = captures.find((c: any) => c.name.split('.')[0] === capture.name.split('.')[0] && c.name.endsWith('.name') && c.node.parent === node);

                const chunk: SemanticChunk = {
                    id: `${filePath}-${node.startPosition.row}`,
                    filePath,
                    type: capture.name.split('.')[0] as any,
                    name: nameNode?.node.text || 'anonymous',
                    content: node.text,
                    startLine: node.startPosition.row + 1,
                    endLine: node.endPosition.row + 1,
                    hash: crypto.createHash('sha256').update(node.text).digest('hex')
                };
                chunks.push(chunk);
            }
        }

        return chunks;
    }

    private async generateEmbeddings(texts: string[]): Promise<number[][]> {
        if (this.azureConfig && this.getAzureToken && this.azureConfig.embeddingUrl) {
            try {
                const token = await this.getAzureToken(this.azureConfig.loginUrl, this.azureConfig.username, this.azureConfig.password);
                const response = await fetch(this.azureConfig.embeddingUrl, {
                    method: 'POST',
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({ input: texts })
                });
                
                if (response.ok) {
                    const data: any = await response.json();
                    return data.data?.map((d: any) => d.embedding).filter((e: any): e is number[] => !!e) || [];
                }
            } catch (e) {
                console.error('[AZURE_EMBEDDING] Failed:', e);
            }
        }

        if (this.voyage) {
            const response = await this.voyage.embed({
                input: texts,
                model: 'voyage-code-2'
            });
            return response.data?.map((d: any) => d.embedding).filter((e: any): e is number[] => !!e) || [];
        }

        return [];
    }

    private cosineSimilarity(A: number[], B: number[]): number {
        let dotproduct = 0;
        let mA = 0;
        let mB = 0;
        for (let i = 0; i < A.length; i++) {
            dotproduct += A[i] * B[i];
            mA += A[i] * A[i];
            mB += B[i] * B[i];
        }
        mA = Math.sqrt(mA);
        mB = Math.sqrt(mB);
        return dotproduct / (mA * mB);
    }

    async search(query: string, limit = 5) {
        if (this.records.length === 0) return [];

        let queryEmbedding: number[] | null = null;

        if (this.azureConfig && this.getAzureToken && this.azureConfig.embeddingUrl) {
            try {
                const token = await this.getAzureToken(this.azureConfig.loginUrl, this.azureConfig.username, this.azureConfig.password);
                const response = await fetch(this.azureConfig.embeddingUrl, {
                    method: 'POST',
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({ input: [query] })
                });
                if (response.ok) {
                    const data: any = await response.json();
                    queryEmbedding = data.data?.[0]?.embedding || null;
                }
            } catch (e) {
                console.error('[AZURE_SEARCH_EMBEDDING] Failed:', e);
            }
        }

        if (!queryEmbedding && this.voyage) {
            const queryEmbeddingResponse = await this.voyage.embed({
                input: [query],
                model: 'voyage-code-2'
            });
            queryEmbedding = queryEmbeddingResponse.data?.[0].embedding || null;
        }

        if (!queryEmbedding) return [];

        // Calculate cosine similarity for all records that have a vector
        const scored = this.records
            .filter(r => r.vector)
            .map(r => ({
                ...r,
                _distance: 1 - this.cosineSimilarity(queryEmbedding!, r.vector!) // lower distance is better
            }));

        // Sort by closest (lowest distance) and take the top results
        scored.sort((a, b) => a._distance - b._distance);
        
        // Remove vectors before returning to save memory over IPC/rendering
        return scored.slice(0, limit).map(({ vector, ...rest }) => rest);
    }
}
