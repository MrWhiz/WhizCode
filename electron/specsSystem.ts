// ====== SPECS SYSTEM ======
// Structured feature development: Requirements → Design → Implementation
// Each spec lives in .whizcode/specs/{feature-name}/ with 3 documents.

import * as fs from 'node:fs/promises';
import { join } from 'node:path';

export interface SpecTask {
  id: string;
  description: string;
  completed: boolean;
  subtasks?: SpecTask[];
}

export interface Spec {
  name: string;
  slug: string;
  path: string;
  requirements: string;
  design: string;
  tasks: SpecTask[];
  rawTasksMd: string;
  createdAt: string;
  updatedAt: string;
}

export interface SpecSummary {
  name: string;
  slug: string;
  totalTasks: number;
  completedTasks: number;
  progress: number; // 0-100
  createdAt: string;
  updatedAt: string;
}

export class SpecsManager {
  private specsDir: string;

  constructor(workspaceRoot: string) {
    this.specsDir = join(workspaceRoot, '.whizcode', 'specs');
  }

  async initialize() {
    await fs.mkdir(this.specsDir, { recursive: true });
  }

  private slugify(name: string): string {
    return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  }

  async createSpec(name: string, initialRequirements: string = ''): Promise<Spec> {
    const slug = this.slugify(name);
    const specDir = join(this.specsDir, slug);
    await fs.mkdir(specDir, { recursive: true });

    const now = new Date().toISOString();

    const requirementsMd = initialRequirements || `# ${name} — Requirements\n\n## Overview\n\n[Describe what this feature does and why it's needed]\n\n## User Stories\n\n- As a user, I want to ... so that ...\n\n## Acceptance Criteria\n\n- [ ] Criteria 1\n- [ ] Criteria 2\n`;

    const designMd = `# ${name} — Design\n\n## Architecture\n\n[Describe the technical approach]\n\n## Components\n\n[List the files/modules that will be added or modified]\n\n## Data Flow\n\n[Describe how data flows through the system]\n\n## Considerations\n\n- [ ] Error handling\n- [ ] Testing strategy\n`;

    const tasksMd = `# ${name} — Implementation Tasks\n\n## Tasks\n\n- [ ] Set up basic structure\n- [ ] Implement core logic\n- [ ] Add error handling\n- [ ] Write tests\n- [ ] Update documentation\n`;

    const metadataJson = JSON.stringify({ name, slug, createdAt: now, updatedAt: now }, null, 2);

    await Promise.all([
      fs.writeFile(join(specDir, 'requirements.md'), requirementsMd, 'utf-8'),
      fs.writeFile(join(specDir, 'design.md'), designMd, 'utf-8'),
      fs.writeFile(join(specDir, 'tasks.md'), tasksMd, 'utf-8'),
      fs.writeFile(join(specDir, 'metadata.json'), metadataJson, 'utf-8'),
    ]);

    return {
      name,
      slug,
      path: specDir,
      requirements: requirementsMd,
      design: designMd,
      tasks: this.parseTasksMd(tasksMd),
      rawTasksMd: tasksMd,
      createdAt: now,
      updatedAt: now,
    };
  }

  async getSpec(slug: string): Promise<Spec | null> {
    const specDir = join(this.specsDir, slug);
    try {
      const [requirements, design, tasksMd, metaRaw] = await Promise.all([
        fs.readFile(join(specDir, 'requirements.md'), 'utf-8'),
        fs.readFile(join(specDir, 'design.md'), 'utf-8'),
        fs.readFile(join(specDir, 'tasks.md'), 'utf-8'),
        fs.readFile(join(specDir, 'metadata.json'), 'utf-8'),
      ]);
      const meta = JSON.parse(metaRaw);
      return {
        name: meta.name,
        slug: meta.slug,
        path: specDir,
        requirements,
        design,
        tasks: this.parseTasksMd(tasksMd),
        rawTasksMd: tasksMd,
        createdAt: meta.createdAt,
        updatedAt: meta.updatedAt,
      };
    } catch {
      return null;
    }
  }

  async listSpecs(): Promise<SpecSummary[]> {
    try {
      const entries = await fs.readdir(this.specsDir, { withFileTypes: true });
      const specs: SpecSummary[] = [];

      for (const entry of entries) {
        if (!entry.isDirectory()) continue;
        const spec = await this.getSpec(entry.name);
        if (!spec) continue;
        const total = this.countTasks(spec.tasks);
        const completed = this.countCompletedTasks(spec.tasks);
        specs.push({
          name: spec.name,
          slug: spec.slug,
          totalTasks: total,
          completedTasks: completed,
          progress: total > 0 ? Math.round((completed / total) * 100) : 0,
          createdAt: spec.createdAt,
          updatedAt: spec.updatedAt,
        });
      }

      return specs.sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime());
    } catch {
      return [];
    }
  }

  async getSpecsSummaryText(): Promise<string> {
    const specs = await this.listSpecs();
    if (specs.length === 0) return '';
    const lines = specs.map(s => `- ${s.name} (${s.slug}): ${s.completedTasks}/${s.totalTasks} tasks (${s.progress}%)`);
    return `\n<feature_specs>\n${lines.join('\n')}\n</feature_specs>\n`;
  }

  async updateSpecDocument(slug: string, docType: 'requirements' | 'design' | 'tasks', content: string): Promise<boolean> {
    const specDir = join(this.specsDir, slug);
    try {
      await fs.writeFile(join(specDir, `${docType}.md`), content, 'utf-8');
      // Update timestamp
      const metaPath = join(specDir, 'metadata.json');
      const metaRaw = await fs.readFile(metaPath, 'utf-8');
      const meta = JSON.parse(metaRaw);
      meta.updatedAt = new Date().toISOString();
      await fs.writeFile(metaPath, JSON.stringify(meta, null, 2), 'utf-8');
      return true;
    } catch {
      return false;
    }
  }

  async completeTask(slug: string, taskDescription: string): Promise<{ success: boolean; message: string }> {
    const specDir = join(this.specsDir, slug);
    try {
      let tasksMd = await fs.readFile(join(specDir, 'tasks.md'), 'utf-8');

      // Match the task line (unchecked) and check it
      // Pattern: "- [ ] taskDescription" → "- [x] taskDescription"
      const escapedDesc = taskDescription.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const taskRegex = new RegExp(`^(\\s*)-\\s*\\[\\s*\\]\\s*(${escapedDesc}.*)$`, 'mi');

      if (!taskRegex.test(tasksMd)) {
        // Try fuzzy: find any unchecked task containing key words
        const words = taskDescription.toLowerCase().split(/\s+/).filter(w => w.length > 3);
        const lines = tasksMd.split('\n');
        const idx = lines.findIndex(l =>
          l.match(/\[\s*\]/) && words.some(w => l.toLowerCase().includes(w))
        );
        if (idx === -1) {
          return { success: false, message: `Task not found: "${taskDescription}"` };
        }
        lines[idx] = lines[idx].replace('[ ]', '[x]');
        tasksMd = lines.join('\n');
      } else {
        tasksMd = tasksMd.replace(taskRegex, (_, indent, rest) => `${indent}- [x] ${rest.trim()}`);
      }

      await fs.writeFile(join(specDir, 'tasks.md'), tasksMd, 'utf-8');

      // Update timestamp
      const metaPath = join(specDir, 'metadata.json');
      const metaRaw = await fs.readFile(metaPath, 'utf-8');
      const meta = JSON.parse(metaRaw);
      meta.updatedAt = new Date().toISOString();
      await fs.writeFile(metaPath, JSON.stringify(meta, null, 2), 'utf-8');

      const tasks = this.parseTasksMd(tasksMd);
      const total = this.countTasks(tasks);
      const completed = this.countCompletedTasks(tasks);
      return {
        success: true,
        message: `✅ Marked task as complete. Progress: ${completed}/${total} tasks done (${Math.round((completed / total) * 100)}%)`
      };
    } catch (e: any) {
      return { success: false, message: `Error: ${e.message}` };
    }
  }

  async deleteSpec(slug: string): Promise<boolean> {
    const specDir = join(this.specsDir, slug);
    try {
      await fs.rm(specDir, { recursive: true });
      return true;
    } catch {
      return false;
    }
  }

  // ── Parsing Helpers ────────────────────────────────────────────────────

  parseTasksMd(md: string): SpecTask[] {
    const tasks: SpecTask[] = [];
    const lines = md.split('\n');
    let idCounter = 0;

    for (const line of lines) {
      // Match "- [ ] ..." or "- [x] ..."
      const m = line.match(/^(\s*)-\s*\[([ xX])\]\s*(.+)$/);
      if (!m) continue;
      const indent = m[1].length;
      const completed = m[2].toLowerCase() === 'x';
      const description = m[3].trim();
      const task: SpecTask = { id: `task-${++idCounter}`, description, completed };

      if (indent === 0) {
        tasks.push(task);
      } else {
        // Subtask of last root task
        const parent = tasks[tasks.length - 1];
        if (parent) {
          if (!parent.subtasks) parent.subtasks = [];
          parent.subtasks.push(task);
        }
      }
    }
    return tasks;
  }

  private countTasks(tasks: SpecTask[]): number {
    return tasks.reduce((sum, t) => sum + 1 + (t.subtasks ? this.countTasks(t.subtasks) : 0), 0);
  }

  private countCompletedTasks(tasks: SpecTask[]): number {
    return tasks.reduce((sum, t) =>
      sum + (t.completed ? 1 : 0) + (t.subtasks ? this.countCompletedTasks(t.subtasks) : 0), 0);
  }

  // ── Context Builder for Agent ──────────────────────────────────────────

  buildSpecContext(spec: Spec): string {
    const tasks = spec.tasks;
    const total = this.countTasks(tasks);
    const completed = this.countCompletedTasks(tasks);
    const remaining = tasks.filter(t => !t.completed);

    return `
<active_spec>
<spec_name>${spec.name}</spec_name>
<progress>${completed}/${total} tasks complete</progress>

<requirements>
${spec.requirements}
</requirements>

<design>
${spec.design}
</design>

<tasks>
${spec.rawTasksMd}
</tasks>

<next_task>
${remaining.length > 0 ? `The next incomplete task is: "${remaining[0].description}"` : 'All tasks are complete! ✅'}
</next_task>
</active_spec>`;
  }
}
