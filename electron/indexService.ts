import { join } from 'node:path';
import * as fs from 'node:fs/promises';
import { app } from 'electron';
import { VoyageAIClient } from 'voyageai';
import * as chokidar from 'chokidar';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';

import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const _require = createRequire(import.meta.url)
/** 
 * WE USE DYNAMIC REQUIRE FOR NATIVE MODULES TO PREVENT VITE FROM TRYING TO BUNDLE BINARY DATA 
 */
let treeSitter: any;
let treeSitterTypeScript: any;
let lancedb: any;

try {
    treeSitter = _require('tree-sitter');
    treeSitterTypeScript = _require('tree-sitter-typescript');
    lancedb = _require('@lancedb/lancedb');
} catch (e) {
    console.error("Failed to load native modules in IndexingService:", e);
}

// Types for our semantic blocks
interface SemanticChunk {
    id: string;
    filePath: string;
    type: 'function' | 'class' | 'constant' | 'method' | 'other';
    name: string;
    content: string;
    startLine: number;
    endLine: number;
    hash: string;
}

export class IndexingService {
    private db: any = null;
    private table: any = null;
    private parser: any;
    private voyage: VoyageAIClient;
    private watcher: chokidar.FSWatcher | null = null;
    private workspacePath: string | null = null;
    private fileHashes: Map<string, string> = new Map(); // Merkle-like watcher
    private onChange: ((path: string) => void) | null = null;

    constructor(apiKey: string, onChange?: (path: string) => void) {
        this.parser = new treeSitter();
        this.onChange = onChange || null;
        // Default to TypeScript TSX for most React/TS projects
        this.parser.setLanguage(treeSitterTypeScript.tsx);

        this.voyage = new VoyageAIClient({ apiKey });
    }

    async initialize(workspacePath: string) {
        this.workspacePath = workspacePath;
        const dbPath = join(app.getPath('userData'), 'vector_db');
        await fs.mkdir(dbPath, { recursive: true });

        this.db = await lancedb.connect(dbPath);

        // Create or open the table
        try {
            this.table = await this.db.openTable('semantic_chunks');
        } catch {
            // First time initialization
            console.log('Creating semantic_chunks table...');
        }

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
            await this.indexFile(file);
        }
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

    async indexFile(filePath: string) {
        try {
            const content = await fs.readFile(filePath, 'utf-8');
            const fileHash = crypto.createHash('sha256').update(content).digest('hex');

            // If file hash hasn't changed, skip entirely
            if (this.fileHashes.get(filePath) === fileHash) {
                return;
            }

            const chunks = this.parseFile(filePath, content);

            // Get existing hashes for this file from the table
            let existingHashes: Map<string, string> = new Map();
            if (this.table) {
                const existingEntries = await this.table
                    .query()
                    .where(`filePath = "${filePath.replace(/\\/g, '\\\\')}"`)
                    .select(['id', 'hash'])
                    .toArray();

                existingHashes = new Map(existingEntries.map((e: any) => [e.id as string, e.hash as string]));
            }

            // Filter chunks that actually changed
            const changedChunks = chunks.filter(c => existingHashes.get(c.id) !== c.hash);

            if (changedChunks.length > 0) {
                // Only generate embeddings for changed chunks
                const embeddings = await this.generateEmbeddings(changedChunks.map(c => c.content));

                const dataToUpsert = changedChunks.map((chunk, i) => ({
                    vector: embeddings[i],
                    id: chunk.id,
                    filePath: chunk.filePath,
                    type: chunk.type,
                    name: chunk.name,
                    content: chunk.content,
                    startLine: chunk.startLine,
                    endLine: chunk.endLine,
                    hash: chunk.hash
                }));

                if (!this.table) {
                    this.table = await this.db!.createTable('semantic_chunks', dataToUpsert);
                } else {
                    // Delete old versions of changed chunks and add new ones
                    for (const chunk of changedChunks) {
                        await this.table.delete(`id = "${chunk.id}"`);
                    }
                    await this.table.add(dataToUpsert);
                }
            }

            // Cleanup chunks that no longer exist in the file
            if (this.table) {
                const allCurrentIds = new Set(chunks.map(c => c.id));
                const entriesInDb = await this.table
                    .query()
                    .where(`filePath = "${filePath.replace(/\\/g, '\\\\')}"`)
                    .select(['id'])
                    .toArray();

                for (const entry of entriesInDb) {
                    if (!allCurrentIds.has(entry.id as string)) {
                        await this.table.delete(`id = "${entry.id}"`);
                    }
                }
            }

            this.fileHashes.set(filePath, fileHash);
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
        const response = await this.voyage.embed({
            input: texts,
            model: 'voyage-code-2'
        });
        return response.data?.map((d: any) => d.embedding).filter((e: any): e is number[] => !!e) || [];
    }

    async search(query: string, limit = 5) {
        if (!this.table) return [];

        const queryEmbeddingResponse = await this.voyage.embed({
            input: [query],
            model: 'voyage-code-2'
        });

        const queryEmbedding = queryEmbeddingResponse.data?.[0].embedding;
        if (!queryEmbedding) return [];

        const results = await this.table
            .vectorSearch(queryEmbedding)
            .limit(limit)
            .toArray();

        return results;
    }
}
