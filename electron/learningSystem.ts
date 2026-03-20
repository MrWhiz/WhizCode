// Learning System for WhizCode
// Implements adaptive learning from user interactions and outcomes

import { ContextMemory } from './contextMemory';
import type { InteractionMemory, SuccessfulStrategy, ErrorPattern } from './contextMemory';

export interface LearningInsight {
  type: 'pattern' | 'preference' | 'strategy' | 'error';
  confidence: number;
  description: string;
  recommendation: string;
  evidence: string[];
}

export interface AdaptationRule {
  id: string;
  condition: string;
  action: string;
  priority: number;
  successRate: number;
  usageCount: number;
}

export interface LearningMetrics {
  totalInteractions: number;
  successRate: number;
  averageResponseTime: number;
  userSatisfactionScore: number;
  improvementTrends: {
    period: string;
    metric: string;
    change: number;
  }[];
}

export class LearningSystem {
  private contextMemory: ContextMemory;
  private adaptationRules: Map<string, AdaptationRule> = new Map();
  private learningInsights: LearningInsight[] = [];

  constructor(contextMemory: ContextMemory) {
    this.contextMemory = contextMemory;
    this.initializeAdaptationRules();
  }

  private initializeAdaptationRules() {
    // Initialize with basic adaptation rules
    const basicRules: AdaptationRule[] = [
      {
        id: 'prefer_successful_tools',
        condition: 'tool_success_rate > 0.8',
        action: 'prioritize_tool',
        priority: 1,
        successRate: 1.0,
        usageCount: 0
      },
      {
        id: 'avoid_failed_patterns',
        condition: 'pattern_failure_rate > 0.6',
        action: 'suggest_alternative',
        priority: 2,
        successRate: 0.9,
        usageCount: 0
      },
      {
        id: 'adapt_to_user_style',
        condition: 'user_preference_confidence > 0.7',
        action: 'adjust_approach',
        priority: 3,
        successRate: 0.85,
        usageCount: 0
      }
    ];

    basicRules.forEach(rule => {
      this.adaptationRules.set(rule.id, rule);
    });
  }

  async analyzeInteractionPatterns(): Promise<LearningInsight[]> {
    const insights: LearningInsight[] = [];
    // Only analyze the most recent 20 sessions to prevent performance degradation
    const sessions = this.contextMemory.getSessionHistory().slice(-20);
    
    if (sessions.length === 0) return insights;

    // Analyze tool usage patterns
    const toolInsights = this.analyzeToolUsagePatterns(sessions);
    insights.push(...toolInsights);

    // Analyze error patterns
    const errorInsights = this.analyzeErrorPatterns();
    insights.push(...errorInsights);

    // Analyze user preferences
    const preferenceInsights = this.analyzeUserPreferences();
    insights.push(...preferenceInsights);

    // Analyze successful strategies
    const strategyInsights = this.analyzeSuccessfulStrategies();
    insights.push(...strategyInsights);

    this.learningInsights = insights;
    return insights;
  }

  private analyzeToolUsagePatterns(sessions: any[]): LearningInsight[] {
    const insights: LearningInsight[] = [];
    const toolStats = new Map<string, { success: number; total: number; avgDuration: number }>();

    // Collect tool usage statistics
    sessions.forEach(session => {
      session.interactions.forEach((interaction: InteractionMemory) => {
        interaction.toolsUsed.forEach(tool => {
          const stats = toolStats.get(tool) || { success: 0, total: 0, avgDuration: 0 };
          stats.total++;
          if (interaction.success) stats.success++;
          stats.avgDuration = (stats.avgDuration * (stats.total - 1) + interaction.duration) / stats.total;
          toolStats.set(tool, stats);
        });
      });
    });

    // Generate insights from tool statistics
    toolStats.forEach((stats, tool) => {
      const successRate = stats.success / stats.total;
      
      if (successRate > 0.9 && stats.total > 5) {
        insights.push({
          type: 'pattern',
          confidence: Math.min(successRate, 0.95),
          description: `Tool "${tool}" has high success rate (${(successRate * 100).toFixed(1)}%)`,
          recommendation: `Prioritize using "${tool}" for similar tasks`,
          evidence: [`${stats.success}/${stats.total} successful uses`, `Average duration: ${stats.avgDuration.toFixed(1)}s`]
        });
      } else if (successRate < 0.5 && stats.total > 3) {
        insights.push({
          type: 'pattern',
          confidence: 1 - successRate,
          description: `Tool "${tool}" has low success rate (${(successRate * 100).toFixed(1)}%)`,
          recommendation: `Consider alternative tools or improve "${tool}" usage strategy`,
          evidence: [`${stats.success}/${stats.total} successful uses`, `High failure rate detected`]
        });
      }
    });

    return insights;
  }

  private analyzeErrorPatterns(): LearningInsight[] {
    const insights: LearningInsight[] = [];
    const errorPatterns = this.contextMemory.getErrorPatterns();

    errorPatterns.forEach(pattern => {
      if (pattern.successRate > 0.8 && pattern.occurrences > 2) {
        insights.push({
          type: 'error',
          confidence: pattern.successRate,
          description: `Reliable solution found for "${pattern.errorType}" errors`,
          recommendation: `Apply solution: ${pattern.solution}`,
          evidence: [
            `Success rate: ${(pattern.successRate * 100).toFixed(1)}%`,
            `Occurrences: ${pattern.occurrences}`,
            `Last seen: ${pattern.lastOccurrence.toLocaleDateString()}`
          ]
        });
      } else if (pattern.successRate < 0.3 && pattern.occurrences > 3) {
        insights.push({
          type: 'error',
          confidence: 1 - pattern.successRate,
          description: `Persistent problem with "${pattern.errorType}" errors`,
          recommendation: `Research alternative solutions or escalate to user`,
          evidence: [
            `Low success rate: ${(pattern.successRate * 100).toFixed(1)}%`,
            `Frequent occurrences: ${pattern.occurrences}`,
            `Current solution may be inadequate`
          ]
        });
      }
    });

    return insights;
  }

  private analyzeUserPreferences(): LearningInsight[] {
    const insights: LearningInsight[] = [];
    const preferences = this.contextMemory.getUserPreferences();

    preferences.forEach(pref => {
      if (pref.confidence > 0.7) {
        insights.push({
          type: 'preference',
          confidence: pref.confidence / 10, // Normalize to 0-1
          description: `Strong user preference detected for "${pref.key}"`,
          recommendation: `Adapt behavior to match preference: ${JSON.stringify(pref.value)}`,
          evidence: [
            `Confidence: ${pref.confidence.toFixed(1)}/10`,
            `Last updated: ${pref.lastUpdated.toLocaleDateString()}`
          ]
        });
      }
    });

    return insights;
  }

  private analyzeSuccessfulStrategies(): LearningInsight[] {
    const insights: LearningInsight[] = [];
    const strategies = this.contextMemory.getSuccessfulStrategies();

    strategies.forEach(strategy => {
      if (strategy.successRate > 0.85 && strategy.usageCount > 3) {
        insights.push({
          type: 'strategy',
          confidence: strategy.successRate,
          description: `Highly effective strategy for "${strategy.taskType}" tasks`,
          recommendation: `Prioritize strategy: ${strategy.strategy}`,
          evidence: [
            `Success rate: ${(strategy.successRate * 100).toFixed(1)}%`,
            `Usage count: ${strategy.usageCount}`,
            `Average duration: ${strategy.averageDuration.toFixed(1)}s`,
            `Tools used: ${strategy.tools.join(', ')}`
          ]
        });
      }
    });

    return insights;
  }

  async adaptBehavior(context: any): Promise<string[]> {
    const adaptations: string[] = [];
    const insights = await this.analyzeInteractionPatterns();

    // Apply adaptation rules based on insights
    for (const insight of insights) {
      const applicableRules = this.findApplicableRules(insight, context);
      
      for (const rule of applicableRules) {
        const adaptation = this.applyAdaptationRule(rule, insight, context);
        if (adaptation) {
          adaptations.push(adaptation);
          rule.usageCount++;
        }
      }
    }

    return adaptations;
  }

  private findApplicableRules(insight: LearningInsight, context: any): AdaptationRule[] {
    const applicableRules: AdaptationRule[] = [];

    this.adaptationRules.forEach(rule => {
      if (this.evaluateRuleCondition(rule.condition, insight, context)) {
        applicableRules.push(rule);
      }
    });

    return applicableRules.sort((a, b) => b.priority - a.priority);
  }

  private evaluateRuleCondition(condition: string, insight: LearningInsight, context: any): boolean {
    // Simple condition evaluation - in a real system, this would be more sophisticated
    if (condition.includes('tool_success_rate') && insight.type === 'pattern') {
      return insight.confidence > 0.8;
    }
    
    if (condition.includes('pattern_failure_rate') && insight.type === 'pattern') {
      return insight.confidence > 0.6 && insight.description.includes('low success rate');
    }
    
    if (condition.includes('user_preference_confidence') && insight.type === 'preference') {
      return insight.confidence > 0.7;
    }

    return false;
  }

  private applyAdaptationRule(rule: AdaptationRule, insight: LearningInsight, context: any): string | null {
    switch (rule.action) {
      case 'prioritize_tool':
        if (insight.type === 'pattern' && insight.description.includes('high success rate')) {
          return `Prioritizing successful tools based on learned patterns`;
        }
        break;

      case 'suggest_alternative':
        if (insight.type === 'pattern' && insight.description.includes('low success rate')) {
          return `Suggesting alternative approaches due to poor performance patterns`;
        }
        break;

      case 'adjust_approach':
        if (insight.type === 'preference') {
          return `Adjusting approach based on learned user preferences`;
        }
        break;
    }

    return null;
  }

  async generateRecommendations(taskType: string, context: any): Promise<string[]> {
    const recommendations: string[] = [];
    
    // Get relevant successful strategies
    const strategies = this.contextMemory.getBestStrategies(taskType);
    strategies.forEach(strategy => {
      if (strategy.successRate > 0.8) {
        recommendations.push(`Use proven strategy: ${strategy.strategy} (${(strategy.successRate * 100).toFixed(1)}% success rate)`);
      }
    });

    // Get relevant error patterns to avoid
    const errorPatterns = this.contextMemory.getSimilarErrorPatterns(taskType, JSON.stringify(context));
    errorPatterns.forEach(pattern => {
      if (pattern.successRate < 0.5) {
        recommendations.push(`Avoid common pitfall: ${pattern.errorType} - ${pattern.solution}`);
      }
    });

    // Get relevant code patterns
    const codePatterns = this.contextMemory.getRelevantCodePatterns(taskType);
    codePatterns.slice(0, 3).forEach(pattern => {
      recommendations.push(`Consider using pattern: ${pattern.pattern} (used ${pattern.frequency} times)`);
    });

    return recommendations;
  }

  async updateLearning(interaction: InteractionMemory) {
    // Record the interaction in context memory
    this.contextMemory.recordInteraction(
      interaction.userRequest,
      interaction.agentResponse,
      interaction.toolsUsed,
      interaction.success,
      interaction.duration,
      interaction.context
    );

    // Extract and record patterns
    await this.extractAndRecordPatterns(interaction);

    // Update strategy success rates
    await this.updateStrategyMetrics(interaction);

    // Learn from errors
    if (!interaction.success) {
      await this.learnFromError(interaction);
    }
  }

  private async extractAndRecordPatterns(interaction: InteractionMemory) {
    // Extract code patterns from successful interactions
    if (interaction.success && interaction.toolsUsed.includes('write_file')) {
      // This would analyze the code written and extract patterns
      // For now, we'll record basic patterns
      this.contextMemory.recordCodePattern(
        interaction.toolsUsed.join(' -> '),
        interaction.userRequest,
        interaction.context?.projectType,
        interaction.context?.language
      );
    }
  }

  private async updateStrategyMetrics(interaction: InteractionMemory) {
    const strategy = interaction.toolsUsed.join(' -> ');
    const taskType = this.classifyTaskType(interaction.userRequest);
    
    this.contextMemory.recordSuccessfulStrategy(
      taskType,
      strategy,
      interaction.toolsUsed,
      interaction.duration,
      interaction.success
    );
  }

  private async learnFromError(interaction: InteractionMemory) {
    const errorType = this.extractErrorType(interaction.agentResponse);
    const context = interaction.userRequest;
    const attemptedSolution = interaction.toolsUsed.join(' -> ');

    this.contextMemory.recordErrorPattern(
      errorType,
      context,
      attemptedSolution,
      false
    );
  }

  private classifyTaskType(userRequest: string): string {
    const lowerRequest = userRequest.toLowerCase();
    
    if (lowerRequest.includes('fix') || lowerRequest.includes('error') || lowerRequest.includes('bug')) {
      return 'bug-fix';
    }
    if (lowerRequest.includes('add') || lowerRequest.includes('create') || lowerRequest.includes('implement')) {
      return 'feature-implementation';
    }
    if (lowerRequest.includes('refactor') || lowerRequest.includes('improve') || lowerRequest.includes('optimize')) {
      return 'refactoring';
    }
    if (lowerRequest.includes('analyze') || lowerRequest.includes('understand') || lowerRequest.includes('explain')) {
      return 'analysis';
    }
    
    return 'general';
  }

  private extractErrorType(agentResponse: string): string {
    // Simple error type extraction - could be more sophisticated
    if (agentResponse.includes('syntax error')) return 'syntax-error';
    if (agentResponse.includes('file not found')) return 'file-not-found';
    if (agentResponse.includes('permission denied')) return 'permission-error';
    if (agentResponse.includes('timeout')) return 'timeout-error';
    
    return 'unknown-error';
  }

  async generateLearningMetrics(): Promise<LearningMetrics> {
    const sessions = this.contextMemory.getSessionHistory();
    const allInteractions = sessions.flatMap(s => s.interactions);

    if (allInteractions.length === 0) {
      return {
        totalInteractions: 0,
        successRate: 0,
        averageResponseTime: 0,
        userSatisfactionScore: 0,
        improvementTrends: []
      };
    }

    const successfulInteractions = allInteractions.filter(i => i.success);
    const successRate = successfulInteractions.length / allInteractions.length;
    const averageResponseTime = allInteractions.reduce((sum, i) => sum + i.duration, 0) / allInteractions.length;
    
    const satisfactionScores = sessions
      .filter(s => s.userSatisfaction !== undefined)
      .map(s => s.userSatisfaction!);
    const userSatisfactionScore = satisfactionScores.length > 0 
      ? satisfactionScores.reduce((sum, score) => sum + score, 0) / satisfactionScores.length
      : 0;

    // Calculate improvement trends (simplified)
    const improvementTrends = this.calculateImprovementTrends(sessions);

    return {
      totalInteractions: allInteractions.length,
      successRate,
      averageResponseTime,
      userSatisfactionScore,
      improvementTrends
    };
  }

  private calculateImprovementTrends(sessions: any[]): { period: string; metric: string; change: number }[] {
    if (sessions.length < 2) return [];

    const trends = [];
    const recentSessions = sessions.slice(-10);
    const olderSessions = sessions.slice(-20, -10);

    if (olderSessions.length > 0) {
      const recentSuccessRate = this.calculateSuccessRate(recentSessions);
      const olderSuccessRate = this.calculateSuccessRate(olderSessions);
      
      trends.push({
        period: 'recent',
        metric: 'success_rate',
        change: recentSuccessRate - olderSuccessRate
      });

      const recentAvgTime = this.calculateAverageTime(recentSessions);
      const olderAvgTime = this.calculateAverageTime(olderSessions);
      
      trends.push({
        period: 'recent',
        metric: 'response_time',
        change: recentAvgTime - olderAvgTime
      });
    }

    return trends;
  }

  private calculateSuccessRate(sessions: any[]): number {
    const allInteractions = sessions.flatMap(s => s.interactions);
    if (allInteractions.length === 0) return 0;
    
    const successful = allInteractions.filter(i => i.success).length;
    return successful / allInteractions.length;
  }

  private calculateAverageTime(sessions: any[]): number {
    const allInteractions = sessions.flatMap(s => s.interactions);
    if (allInteractions.length === 0) return 0;
    
    return allInteractions.reduce((sum, i) => sum + i.duration, 0) / allInteractions.length;
  }

  getLearningInsights(): LearningInsight[] {
    return this.learningInsights;
  }

  getAdaptationRules(): AdaptationRule[] {
    return Array.from(this.adaptationRules.values());
  }
}