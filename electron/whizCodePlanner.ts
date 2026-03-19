/**
 * WhizCode Planning System
 * Creates structured execution plans for agent tasks
 */

export interface WhizCodeTask {
  id: string;
  description: string;
  type: 'analysis' | 'edit' | 'command' | 'review' | 'planning';
  priority: number;
  dependencies: string[];
  estimatedDuration: number; // in seconds
}

export interface WhizCodePlan {
  id: string;
  objective: string;
  tasks: WhizCodeTask[];
  parallelGroups: WhizCodeTask[][];
  estimatedDuration: number;
  riskLevel: 'low' | 'medium' | 'high';
  fallbackStrategies: string[];
}

export interface PlanningContext {
  userRequest: string;
  workspacePath: string;
  activeFile?: { path: string; content: string };
  recentContext?: string;
}

export class WhizCodePlanner {
  private planHistory: Map<string, WhizCodePlan> = new Map();

  async createPlan(context: PlanningContext): Promise<WhizCodePlan> {
    const objective = this.extractObjective(context.userRequest);
    const taskType = this.classifyRequest(context.userRequest);

    let tasks: WhizCodeTask[] = [];

    switch (taskType) {
      case 'bug-fix':
        tasks = this.planBugFix(context);
        break;
      case 'feature-implementation':
        tasks = this.planFeatureImplementation(context);
        break;
      case 'refactoring':
        tasks = this.planRefactoring(context);
        break;
      case 'analysis':
        tasks = this.planAnalysis(context);
        break;
      default:
        tasks = this.planGenericTask(context);
    }

    const parallelGroups = this.optimizeParallelExecution(tasks);
    const plan: WhizCodePlan = {
      id: `plan_${Date.now()}`,
      objective,
      tasks,
      parallelGroups,
      estimatedDuration: this.estimateDuration(tasks),
      riskLevel: this.assessRisk(tasks),
      fallbackStrategies: this.generateFallbackStrategies(tasks)
    };

    this.planHistory.set(plan.id, plan);
    return plan;
  }

  private extractObjective(request: string): string {
    // Extract the main goal from the request
    const lines = request.split('\n');
    return lines[0].substring(0, 100);
  }

  private classifyRequest(request: string): string {
    const lower = request.toLowerCase();
    if (lower.includes('fix') || lower.includes('bug') || lower.includes('error')) {
      return 'bug-fix';
    }
    if (lower.includes('add') || lower.includes('implement') || lower.includes('create')) {
      return 'feature-implementation';
    }
    if (lower.includes('refactor') || lower.includes('improve') || lower.includes('optimize')) {
      return 'refactoring';
    }
    if (lower.includes('analyze') || lower.includes('check') || lower.includes('review')) {
      return 'analysis';
    }
    return 'generic';
  }

  private planBugFix(context: PlanningContext): WhizCodeTask[] {
    return [
      {
        id: 'analyze-bug',
        description: 'Analyze the bug and understand the issue',
        type: 'analysis',
        priority: 1,
        dependencies: [],
        estimatedDuration: 30
      },
      {
        id: 'locate-source',
        description: 'Locate the source of the bug in the codebase',
        type: 'analysis',
        priority: 2,
        dependencies: ['analyze-bug'],
        estimatedDuration: 20
      },
      {
        id: 'implement-fix',
        description: 'Implement the fix',
        type: 'edit',
        priority: 3,
        dependencies: ['locate-source'],
        estimatedDuration: 25
      },
      {
        id: 'verify-fix',
        description: 'Verify the fix works correctly',
        type: 'command',
        priority: 4,
        dependencies: ['implement-fix'],
        estimatedDuration: 15
      }
    ];
  }

  private planFeatureImplementation(context: PlanningContext): WhizCodeTask[] {
    return [
      {
        id: 'design-feature',
        description: 'Design the feature architecture',
        type: 'analysis',
        priority: 1,
        dependencies: [],
        estimatedDuration: 40
      },
      {
        id: 'create-files',
        description: 'Create necessary files and structure',
        type: 'edit',
        priority: 2,
        dependencies: ['design-feature'],
        estimatedDuration: 20
      },
      {
        id: 'implement-feature',
        description: 'Implement the feature',
        type: 'edit',
        priority: 3,
        dependencies: ['create-files'],
        estimatedDuration: 60
      },
      {
        id: 'test-feature',
        description: 'Test the feature',
        type: 'command',
        priority: 4,
        dependencies: ['implement-feature'],
        estimatedDuration: 30
      }
    ];
  }

  private planRefactoring(context: PlanningContext): WhizCodeTask[] {
    return [
      {
        id: 'analyze-code',
        description: 'Analyze code for refactoring opportunities',
        type: 'analysis',
        priority: 1,
        dependencies: [],
        estimatedDuration: 30
      },
      {
        id: 'refactor-code',
        description: 'Refactor the code',
        type: 'edit',
        priority: 2,
        dependencies: ['analyze-code'],
        estimatedDuration: 45
      },
      {
        id: 'verify-refactor',
        description: 'Verify refactoring maintains functionality',
        type: 'command',
        priority: 3,
        dependencies: ['refactor-code'],
        estimatedDuration: 20
      }
    ];
  }

  private planAnalysis(context: PlanningContext): WhizCodeTask[] {
    return [
      {
        id: 'gather-info',
        description: 'Gather information about the codebase',
        type: 'analysis',
        priority: 1,
        dependencies: [],
        estimatedDuration: 25
      },
      {
        id: 'analyze-patterns',
        description: 'Analyze patterns and structure',
        type: 'analysis',
        priority: 2,
        dependencies: ['gather-info'],
        estimatedDuration: 20
      },
      {
        id: 'generate-report',
        description: 'Generate analysis report',
        type: 'review',
        priority: 3,
        dependencies: ['analyze-patterns'],
        estimatedDuration: 15
      }
    ];
  }

  private planGenericTask(context: PlanningContext): WhizCodeTask[] {
    return [
      {
        id: 'understand-request',
        description: 'Understand the request',
        type: 'analysis',
        priority: 1,
        dependencies: [],
        estimatedDuration: 20
      },
      {
        id: 'execute-task',
        description: 'Execute the task',
        type: 'edit',
        priority: 2,
        dependencies: ['understand-request'],
        estimatedDuration: 40
      },
      {
        id: 'verify-result',
        description: 'Verify the result',
        type: 'review',
        priority: 3,
        dependencies: ['execute-task'],
        estimatedDuration: 15
      }
    ];
  }

  private optimizeParallelExecution(tasks: WhizCodeTask[]): WhizCodeTask[][] {
    const groups: WhizCodeTask[][] = [];
    const completed = new Set<string>();

    while (completed.size < tasks.length) {
      const currentGroup: WhizCodeTask[] = [];

      for (const task of tasks) {
        if (completed.has(task.id)) continue;
        if (task.dependencies.every(dep => completed.has(dep))) {
          currentGroup.push(task);
        }
      }

      if (currentGroup.length === 0) break;

      groups.push(currentGroup);
      currentGroup.forEach(t => completed.add(t.id));
    }

    return groups;
  }

  private estimateDuration(tasks: WhizCodeTask[]): number {
    return tasks.reduce((sum, task) => sum + task.estimatedDuration, 0);
  }

  private assessRisk(tasks: WhizCodeTask[]): 'low' | 'medium' | 'high' {
    const editTasks = tasks.filter(t => t.type === 'edit').length;
    const totalTasks = tasks.length;
    const ratio = editTasks / totalTasks;

    if (ratio > 0.7) return 'high';
    if (ratio > 0.4) return 'medium';
    return 'low';
  }

  private generateFallbackStrategies(tasks: WhizCodeTask[]): string[] {
    const strategies: string[] = [];

    if (tasks.some(t => t.type === 'edit')) {
      strategies.push('Create backups before making changes');
    }

    if (tasks.some(t => t.type === 'command')) {
      strategies.push('Run tests to verify changes');
    }

    if (tasks.length > 3) {
      strategies.push('Break down into smaller steps if needed');
    }

    return strategies;
  }

  getPlanHistory(): WhizCodePlan[] {
    return Array.from(this.planHistory.values());
  }

  getPlan(planId: string): WhizCodePlan | undefined {
    return this.planHistory.get(planId);
  }
}
