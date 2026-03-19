/**
 * Agent Executor with Planning Integration
 * Orchestrates the planning and execution phases
 */

import { WhizCodePlanner, WhizCodePlan, PlanningContext } from './whizCodePlanner';

export interface ExecutionPhase {
  phase: 'planning' | 'execution' | 'summary';
  startTime: number;
  endTime?: number;
  duration?: number;
}

export interface AgentExecutionContext {
  planId: string;
  plan: WhizCodePlan;
  phases: ExecutionPhase[];
  startTime: number;
  endTime?: number;
  totalDuration?: number;
  creditsUsed?: number;
}

export class AgentExecutor {
  private planner: WhizCodePlanner;
  private executionContexts: Map<string, AgentExecutionContext> = new Map();

  constructor() {
    this.planner = new WhizCodePlanner();
  }

  /**
   * Create a plan for the user request
   */
  async planTask(context: PlanningContext): Promise<WhizCodePlan> {
    return this.planner.createPlan(context);
  }

  /**
   * Start execution tracking
   */
  startExecution(plan: WhizCodePlan): AgentExecutionContext {
    const executionContext: AgentExecutionContext = {
      planId: plan.id,
      plan,
      phases: [
        {
          phase: 'planning',
          startTime: Date.now(),
          endTime: Date.now(),
          duration: 0
        }
      ],
      startTime: Date.now()
    };

    this.executionContexts.set(plan.id, executionContext);
    return executionContext;
  }

  /**
   * Mark execution phase start
   */
  startPhase(planId: string, phase: 'execution' | 'summary'): void {
    const context = this.executionContexts.get(planId);
    if (!context) return;

    context.phases.push({
      phase,
      startTime: Date.now()
    });
  }

  /**
   * Mark execution phase end
   */
  endPhase(planId: string): void {
    const context = this.executionContexts.get(planId);
    if (!context) return;

    const lastPhase = context.phases[context.phases.length - 1];
    if (lastPhase && !lastPhase.endTime) {
      lastPhase.endTime = Date.now();
      lastPhase.duration = lastPhase.endTime - lastPhase.startTime;
    }
  }

  /**
   * Complete execution
   */
  completeExecution(planId: string, creditsUsed?: number): AgentExecutionContext | undefined {
    const context = this.executionContexts.get(planId);
    if (!context) return;

    context.endTime = Date.now();
    context.totalDuration = context.endTime - context.startTime;
    context.creditsUsed = creditsUsed;

    return context;
  }

  /**
   * Get execution summary
   */
  getExecutionSummary(planId: string): string {
    const context = this.executionContexts.get(planId);
    if (!context) return '';

    const lines: string[] = [];
    lines.push('Summary of Changes:');
    lines.push('');

    // List tasks completed
    context.plan.tasks.forEach((task, i) => {
      lines.push(`${i + 1}. **${task.description}** (${task.type})`);
    });

    lines.push('');
    lines.push(`Credits used: ${context.creditsUsed || 0}  Elapsed time: ${Math.round((context.totalDuration || 0) / 1000)}s`);

    return lines.join('\n');
  }

  /**
   * Get execution context
   */
  getExecutionContext(planId: string): AgentExecutionContext | undefined {
    return this.executionContexts.get(planId);
  }

  /**
   * Clear execution context
   */
  clearExecutionContext(planId: string): void {
    this.executionContexts.delete(planId);
  }
}
