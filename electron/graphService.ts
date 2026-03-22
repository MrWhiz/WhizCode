import { join, dirname, resolve, extname } from 'node:path';
import * as fs from 'node:fs/promises';
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
    console.error("Failed to load native modules in CodeGraphService:", e);
}

export interface GraphNode {
    path: string;
    imports: string[];
    dependents: string[];
}

export class CodeGraphService {
    private parser: any;
    private graph: Map<string, GraphNode> = new Map();
    private workspacePath: string | null = null;

    constructor() {
        if (treeSitter) {
            this.parser = new treeSitter();
            this.parser.setLanguage(treeSitterTypeScript.tsx);
        }
    }

    async initialize(workspacePath: string) {
        this.workspacePath = workspacePath;
        await this.rebuildGraph();
    }

    async rebuildGraph() {
        if (!this.workspacePath) return;
        this.graph.clear();
        const files = await this.getProjectFiles(this.workspacePath);

        for (const file of files) {
            await this.processFile(file);
        }

        this.calculateDependents();
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
                    const ext = extname(entry.name).toLowerCase();
                    if (VALID_EXTS.has(ext)) {
                        results.push(fullPath);
                    }
                }
            }
        }

        await walk(dir);
        return results;
    }

    async updateFile(filePath: string) {
        await this.processFile(filePath);
        this.calculateDependents();
    }

    private async processFile(filePath: string) {
        if (!this.parser) return;
        try {
            const content = await fs.readFile(filePath, 'utf-8');
            const tree = this.parser.parse(content);

            const imports: string[] = [];

            // Corrected queries for tree-sitter-typescript.
            // (import_statement source: (string) @path)
            // (export_statement source: (string) @path)
            const query = new treeSitter.Query(treeSitterTypeScript.tsx, `
        (import_statement source: (string) @import.path)
        (export_statement source: (string) @import.path)
      `);

            const captures = query.captures(tree.rootNode);
            for (const capture of captures) {
                // capture.node is the (string) node in both cases
                let importPath = capture.node.text.replace(/['"]/g, '');
                const resolvedPath = await this.resolveImport(importPath, filePath);
                if (resolvedPath && !imports.includes(resolvedPath)) {
                    imports.push(resolvedPath);
                }
            }

            this.graph.set(filePath, {
                path: filePath,
                imports,
                dependents: [] // Will be calculated after all nodes are processed
            });
        } catch (e) {
            console.error(`Error processing graph for ${filePath}:`, e);
        }
    }

    private async resolveImport(importPath: string, sourceFile: string): Promise<string | null> {
        if (!this.workspacePath) return null;

        // Handle relative imports
        if (importPath.startsWith('.')) {
            const absolutePath = resolve(dirname(sourceFile), importPath);
            return await this.verifyPath(absolutePath);
        }

        // Handle absolute-like imports (e.g. from src or aliases)
        // For now, let's check if it exists relative to workspace root (very basic alias support)
        const workspaceResolved = resolve(this.workspacePath, importPath);
        const checked = await this.verifyPath(workspaceResolved);
        if (checked) return checked;

        // Could be an external dependency (node_modules)
        return null;
    }

    private async verifyPath(basePath: string): Promise<string | null> {
        const exts = ['', '.ts', '.tsx', '.js', '.jsx', '/index.ts', '/index.tsx', '/index.js'];
        for (const ext of exts) {
            const fullPath = basePath + ext;
            try {
                const stat = await fs.stat(fullPath);
                if (stat.isFile()) return fullPath;
            } catch {
                continue;
            }
        }
        return null;
    }

    private calculateDependents() {
        // Reset dependents
        for (const node of this.graph.values()) {
            node.dependents = [];
        }

        // Populate dependents
        for (const [filePath, node] of this.graph.entries()) {
            for (const importPath of node.imports) {
                const importedNode = this.graph.get(importPath);
                if (importedNode && !importedNode.dependents.includes(filePath)) {
                    importedNode.dependents.push(filePath);
                }
            }
        }
    }

    getBlastRadius(filePath: string, depth = 3): string[] {
        const affected = new Set<string>();
        const queue: { path: string; currentDepth: number }[] = [{ path: filePath, currentDepth: 0 }];

        while (queue.length > 0) {
            const { path, currentDepth } = queue.shift()!;
            if (currentDepth >= depth) continue;

            const node = this.graph.get(path);
            if (node) {
                for (const dependent of node.dependents) {
                    if (!affected.has(dependent)) {
                        affected.add(dependent);
                        queue.push({ path: dependent, currentDepth: currentDepth + 1 });
                    }
                }
            }
        }

        return Array.from(affected);
    }

    getGraphSummary(): any {
        const summary: any = {};
        for (const [path, node] of this.graph.entries()) {
            const relPath = this.workspacePath ? path.replace(this.workspacePath.replace(/\\/g, '/'), '').replace(/^[\\/]/, '') : path;
            summary[relPath] = {
                imports: node.imports.map(i => this.workspacePath ? i.replace(this.workspacePath.replace(/\\/g, '/'), '').replace(/^[\\/]/, '') : i),
                dependents: node.dependents.map(d => this.workspacePath ? d.replace(this.workspacePath.replace(/\\/g, '/'), '').replace(/^[\\/]/, '') : d)
            };
        }
        return summary;
    }
}
