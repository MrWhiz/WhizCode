// Strategic Planning System for WhizCode
// Implements intelligent task decomposition and execution planning

export interface Task {
  id: string;
  type: 'analysis' | 'implementation' | 'validation' | 'refactoring';
  description: string;
  tools: string[];
  dependencies: string[];
  priority: number;
  estimatedComplexity: number;
  parallelizable: boolean;
}

export interface ExecutionPlan {
  id: string;
  objective: string;
  tasks: Task[];
  parallelGroups: Task[][];
  estimatedDuration: number;
  riskLevel: 'low' | 'medium' | 'high';
  fallbackStrategies: string[];
}

export interface PlanningContext {
  userRequest: string;
  workspacePath: string;
  projectType?: string;
  codebaseSize?: 'small' | 'medium' | 'large';
  availableTools: string[];
  previousPlans?: ExecutionPlan[];
}

export class StrategicPlanner {
  private planHistory: Map<string, ExecutionPlan> = new Map();
  private taskTemplates: Map<string, Partial<Task>> = new Map();

  constructor() {
    this.initializeTaskTemplates();
  }

  private initializeTaskTemplates() {
    // Common task patterns for efficient planning
    this.taskTemplates.set('file-analysis', {
      type: 'analysis',
      tools: ['read_file', 'readCode', 'getDiagnostics'],
      parallelizable: true,
      estimatedComplexity: 2
    });

    this.taskTemplates.set('code-search', {
      type: 'analysis', 
      tools: ['grepSearch', 'fuzzy_find_file', 'search_files'],
      parallelizable: true,
      estimatedComplexity: 1
    });

    this.taskTemplates.set('file-modification', {
      type: 'implementation',
      tools: ['write_file', 'edit_code', 'strReplace'],
      parallelizable: false,
      estimatedComplexity: 3
    });

    this.taskTemplates.set('validation', {
      type: 'validation',
      tools: ['getDiagnostics', 'run_command'],
      parallelizable: true,
      estimatedComplexity: 2
    });
  }

  async createExecutionPlan(context: PlanningContext): Promise<ExecutionPlan> {
    const objective = this.extractObjective(context.userRequest);
    const taskType = this.classifyRequest(context.userRequest);
    
    let tasks: Task[] = [];
    
    switch (taskType) {
      case 'bug-fix':
        tasks = await this.planBugFix(context);
        break;
      case 'feature-implementation':
        tasks = await this.planFeatureImplementation(context);
        break;
      case 'refactoring':
        tasks = await this.planRefactoring(context);
        break;
      case 'analysis':
        tasks = await this.planAnalysis(context);
        break;
      default:
        tasks = await this.planGenericTask(context);
    }

    const parallelGroups = this.optimizeParallelExecution(tasks);
    const plan: ExecutionPlan = {
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
    // Extract the main objective from user request
    const patterns = [
      /(?:fix|solve|resolve)\s+(.+)/i,
      /(?:implement|create|add)\s+(.+)/i,
      /(?:refactor|improve|optimize)\s+(.+)/i,
      /(?:analyze|understand|explain)\s+(.+)/i
    ];

    for (const pattern of patterns) {
      const match = request.match(pattern);
      if (match) return match[1].trim();
    }

    return request.slice(0, 100) + (request.length > 100 ? '...' : '');
  }

  private classifyRequest(request: string): string {
    const bugKeywords = ['fix', 'error', 'bug', 'issue', 'problem', 'broken'];
    const featureKeywords = ['add', 'create', 'implement', 'new', 'feature'];
    const refactorKeywords = ['refactor', 'improve', 'optimize', 'clean', 'restructure'];
    const analysisKeywords = ['analyze', 'understand', 'explain', 'show', 'find'];

    const lowerRequest = request.toLowerCase();

    if (bugKeywords.some(keyword => lowerRequest.includes(keyword))) return 'bug-fix';
    if (featureKeywords.some(keyword => lowerRequest.includes(keyword))) return 'feature-implementation';
    if (refactorKeywords.some(keyword => lowerRequest.includes(keyword))) return 'refactoring';
    if (analysisKeywords.some(keyword => lowerRequest.includes(keyword))) return 'analysis';

    return 'generic';
  }

  private async planBugFix(context: PlanningContext): Promise<Task[]> {
    return [
      {
        id: 'analyze-error',
        type: 'analysis',
        description: 'Analyze error patterns and symptoms',
        tools: ['getDiagnostics', 'grepSearch', 'read_file'],
        dependencies: [],
        priority: 1,
        estimatedComplexity: 2,
        parallelizable: true
      },
      {
        id: 'locate-source',
        type: 'analysis',
        description: 'Locate source of the issue',
        tools: ['fuzzy_find_file', 'readCode', 'grepSearch'],
        dependencies: ['analyze-error'],
        priority: 2,
        estimatedComplexity: 3,
        parallelizable: false
      },
      {
        id: 'implement-fix',
        type: 'implementation',
        description: 'Implement the bug fix',
        tools: ['edit_code', 'strReplace', 'write_file'],
        dependencies: ['locate-source'],
        priority: 3,
        estimatedComplexity: 4,
        parallelizable: false
      },
      {
        id: 'validate-fix',
        type: 'validation',
        description: 'Validate the fix works',
        tools: ['getDiagnostics', 'run_command'],
        dependencies: ['implement-fix'],
        priority: 4,
        estimatedComplexity: 2,
        parallelizable: true
      }
    ];
  }

  private async planFeatureImplementation(context: PlanningContext): Promise<Task[]> {
    return [
      {
        id: 'understand-requirements',
        type: 'analysis',
        description: 'Understand feature requirements and context',
        tools: ['read_file', 'readCode', 'list_directory'],
        dependencies: [],
        priority: 1,
        estimatedComplexity: 2,
        parallelizable: true
      },
      {
        id: 'design-architecture',
        type: 'analysis',
        description: 'Design feature architecture and integration points',
        tools: ['grepSearch', 'readCode', 'fuzzy_find_file'],
        dependencies: ['understand-requirements'],
        priority: 2,
        estimatedComplexity: 3,
        parallelizable: false
      },
      {
        id: 'implement-core',
        type: 'implementation',
        description: 'Implement core feature functionality',
        tools: ['write_file', 'edit_code'],
        dependencies: ['design-architecture'],
        priority: 3,
        estimatedComplexity: 5,
        parallelizable: false
      },
      {
        id: 'integrate-feature',
        type: 'implementation',
        description: 'Integrate feature with existing codebase',
        tools: ['edit_code', 'strReplace', 'semantic_rename'],
        dependencies: ['implement-core'],
        priority: 4,
        estimatedComplexity: 4,
        parallelizable: false
      },
      {
        id: 'validate-integration',
        type: 'validation',
        description: 'Validate feature integration and functionality',
        tools: ['getDiagnostics', 'run_command'],
        dependencies: ['integrate-feature'],
        priority: 5,
        estimatedComplexity: 3,
        parallelizable: true
      }
    ];
  }

  private async planRefactoring(context: PlanningContext): Promise<Task[]> {
    return [
      {
        id: 'analyze-current-code',
        type: 'analysis',
        description: 'Analyze current code structure and patterns',
        tools: ['readCode', 'grepSearch', 'getDiagnostics'],
        dependencies: [],
        priority: 1,
        estimatedComplexity: 3,
        parallelizable: true
      },
      {
        id: 'identify-improvements',
        type: 'analysis',
        description: 'Identify improvement opportunities',
        tools: ['read_file', 'grepSearch'],
        dependencies: ['analyze-current-code'],
        priority: 2,
        estimatedComplexity: 2,
        parallelizable: false
      },
      {
        id: 'plan-refactoring',
        type: 'analysis',
        description: 'Plan refactoring steps and dependencies',
        tools: ['readCode', 'fuzzy_find_file'],
        dependencies: ['identify-improvements'],
        priority: 3,
        estimatedComplexity: 2,
        parallelizable: false
      },
      {
        id: 'execute-refactoring',
        type: 'refactoring',
        description: 'Execute refactoring changes',
        tools: ['edit_code', 'semantic_rename', 'smart_relocate'],
        dependencies: ['plan-refactoring'],
        priority: 4,
        estimatedComplexity: 4,
        parallelizable: false
      },
      {
        id: 'validate-refactoring',
        type: 'validation',
        description: 'Validate refactoring maintains functionality',
        tools: ['getDiagnostics', 'run_command'],
        dependencies: ['execute-refactoring'],
        priority: 5,
        estimatedComplexity: 2,
        parallelizable: true
      }
    ];
  }

  private async planAnalysis(context: PlanningContext): Promise<Task[]> {
    return [
      {
        id: 'explore-structure',
        type: 'analysis',
        description: 'Explore project structure and organization',
        tools: ['list_directory', 'read_file'],
        dependencies: [],
        priority: 1,
        estimatedComplexity: 1,
        parallelizable: true
      },
      {
        id: 'analyze-components',
        type: 'analysis',
        description: 'Analyze key components and their relationships',
        tools: ['readCode', 'grepSearch', 'fuzzy_find_file'],
        dependencies: ['explore-structure'],
        priority: 2,
        estimatedComplexity: 3,
        parallelizable: true
      },
      {
        id: 'generate-insights',
        type: 'analysis',
        description: 'Generate insights and recommendations',
        tools: ['getDiagnostics', 'read_file'],
        dependencies: ['analyze-components'],
        priority: 3,
        estimatedComplexity: 2,
        parallelizable: false
      }
    ];
  }

  private async planGenericTask(context: PlanningContext): Promise<Task[]> {
    return [
      {
        id: 'understand-request',
        type: 'analysis',
        description: 'Understand the user request and context',
        tools: ['read_file', 'list_directory'],
        dependencies: [],
        priority: 1,
        estimatedComplexity: 1,
        parallelizable: true
      },
      {
        id: 'execute-task',
        type: 'implementation',
        description: 'Execute the requested task',
        tools: ['write_file', 'edit_code', 'run_command'],
        dependencies: ['understand-request'],
        priority: 2,
        estimatedComplexity: 3,
        parallelizable: false
      }
    ];
  }

  private optimizeParallelExecution(tasks: Task[]): Task[][] {
    const groups: Task[][] = [];
    const processed = new Set<string>();
    
    while (processed.size < tasks.length) {
      const currentGroup: Task[] = [];
      
      for (const task of tasks) {
        if (processed.has(task.id)) continue;
        
        // Check if all dependencies are satisfied
        const dependenciesSatisfied = task.dependencies.every(dep => processed.has(dep));
        
        if (dependenciesSatisfied && task.parallelizable) {
          currentGroup.push(task);
          processed.add(task.id);
        } else if (dependenciesSatisfied && currentGroup.length === 0) {
          // Non-parallelizable task, but dependencies are met
          currentGroup.push(task);
          processed.add(task.id);
          break; // Only one non-parallelizable task per group
        }
      }
      
      if (currentGroup.length > 0) {
        groups.push(currentGroup);
      } else {
        // Handle remaining tasks that couldn't be parallelized
        const remaining = tasks.filter(t => !processed.has(t.id));
        if (remaining.length > 0) {
          groups.push([remaining[0]]);
          processed.add(remaining[0].id);
        }
      }
    }
    
    return groups;
  }

  private estimateDuration(tasks: Task[]): number {
    return tasks.reduce((total, task) => total + task.estimatedComplexity, 0);
  }

  private assessRisk(tasks: Task[]): 'low' | 'medium' | 'high' {
    const totalComplexity = tasks.reduce((sum, task) => sum + task.estimatedComplexity, 0);
    const hasHighRiskTools = tasks.some(task => 
      task.tools.some(tool => ['run_command', 'delete_file', 'smart_relocate'].includes(tool))
    );

    if (totalComplexity > 15 || hasHighRiskTools) return 'high';
    if (totalComplexity > 8) return 'medium';
    return 'low';
  }

  private generateFallbackStrategies(tasks: Task[]): string[] {
    const strategies: string[] = [];
    
    if (tasks.some(t => t.type === 'implementation')) {
      strategies.push('Break down complex implementations into smaller steps');
      strategies.push('Use alternative tools if primary tools fail');
    }
    
    if (tasks.some(t => t.tools.includes('run_command'))) {
      strategies.push('Provide manual command instructions if execution fails');
    }
    
    if (tasks.some(t => t.type === 'validation')) {
      strategies.push('Skip validation if tools are unavailable');
    }
    
    strategies.push('Request user guidance if automated approaches fail');
    
    return strategies;
  }

  getPlanHistory(): ExecutionPlan[] {
    return Array.from(this.planHistory.values());
  }

  getPlan(planId: string): ExecutionPlan | undefined {
    return this.planHistory.get(planId);
  }
}