import * as fs from 'node:fs/promises';
import { dirname } from 'node:path';

export interface DiffBlock {
    search: string;
    replacement: string;
}

export interface FileChange {
    path: string;
    blocks: DiffBlock[];
}

export class DiffService {
    /**
     * Parses a model's response for diff blocks.
     * Format:
     * <<<< SEARCH
     * old code
     * ====
     * new code
     * >>>> REPLACE
     */
    static parseDiffBlocks(content: string): DiffBlock[] {
        const blocks: DiffBlock[] = [];
        const regex = /<<<< SEARCH\n([\s\S]*?)\n====\n([\s\S]*?)\n>>>> REPLACE/g;
        let match;

        while ((match = regex.exec(content)) !== null) {
            blocks.push({
                search: match[1],
                replacement: match[2]
            });
        }

        return blocks;
    }

    /**
     * Applies changes to multiple files in a "transactional" manner.
     * If any file fails, it attempts to restore the original state of all files involved.
     */
    async applyTransaction(changes: FileChange[]): Promise<{ success: boolean; error?: string; appliedCount: number }> {
        const backups: Map<string, string> = new Map();
        const appliedFiles: string[] = [];

        try {
            // 1. Create backups first
            for (const change of changes) {
                try {
                    const content = await fs.readFile(change.path, 'utf-8');
                    backups.set(change.path, content);
                } catch (e) {
                    // If file doesn't exist, backup is null (for deletion/reversion purposes)
                    backups.set(change.path, '');
                }
            }

            // 2. Apply changes
            for (const change of changes) {
                let content = backups.get(change.path) || '';

                for (const block of change.blocks) {
                    if (!content.includes(block.search)) {
                        throw new Error(`Search block not found in ${change.path}. Ensure exact match including whitespace.`);
                    }
                    // Use split/join to replace all occurrences if needed, or just first? 
                    // Requirements usually imply specific unique blocks.
                    content = content.replace(block.search, block.replacement);
                }

                const dir = dirname(change.path);
                await fs.mkdir(dir, { recursive: true });
                await fs.writeFile(change.path, content, 'utf-8');
                appliedFiles.push(change.path);
            }

            return { success: true, appliedCount: appliedFiles.length };

        } catch (error: any) {
            console.error('Diff transaction failed, rolling back...', error);

            // 3. Rollback
            for (const [path, originalContent] of backups.entries()) {
                try {
                    if (originalContent === '') {
                        // If it didn't exist, we should probably delete it if we created it
                        // Internal logic: if it was in appliedFiles, we created it.
                        if (appliedFiles.includes(path)) {
                            await fs.unlink(path).catch(() => { });
                        }
                    } else {
                        await fs.writeFile(path, originalContent, 'utf-8');
                    }
                } catch (rollbackError) {
                    console.error(`Rollback failed for ${path}:`, rollbackError);
                }
            }

            return { success: false, error: error.message, appliedCount: 0 };
        }
    }
}
