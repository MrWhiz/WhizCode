import { invoke } from '@tauri-apps/api/core';

export interface ConfidenceDecision {
  decision: string;
  confidence: number;
  risk_level: string;
  action: string;  // "auto_execute", "ask_user", "escalate"
  reasoning: string;
}

export interface ConfidenceMetrics {
  task_confidence: number;
  tool_selection_confidence: number;
  risk_level: string;
  uncertainty_factors: string[];
  requires_human_review: boolean;
}

export interface ConfidenceThresholds {
  very_confident: number;
  confident: number;
  moderate: number;
  low: number;
  actions: Record<string, string>;
  risk_levels: Record<string, string>;
}

export const useConfidenceScoring = () => {
  const evaluateConfidence = async (
    confidence: number,
    taskType: string
  ): Promise<ConfidenceDecision> => {
    try {
      const result = await invoke<ConfidenceDecision>('agent_evaluate_confidence', {
        confidence,
        taskType,
      });
      return result;
    } catch (error) {
      console.error('Confidence evaluation failed:', error);
      throw error;
    }
  };

  const calculateToolConfidence = async (
    toolName: string,
    successRate: number,
    executionTimeMs: number
  ): Promise<number> => {
    try {
      const result = await invoke<number>('agent_calculate_tool_confidence', {
        toolName,
        successRate,
        executionTimeMs,
      });
      return result;
    } catch (error) {
      console.error('Tool confidence calculation failed:', error);
      throw error;
    }
  };

  const assessDecisionRisk = async (
    confidence: number,
    toolCalls: Array<{ tool: string; args: any }>
  ): Promise<ConfidenceMetrics> => {
    try {
      const result = await invoke<ConfidenceMetrics>('agent_assess_decision_risk', {
        confidence,
        toolCalls,
      });
      return result;
    } catch (error) {
      console.error('Risk assessment failed:', error);
      throw error;
    }
  };

  const getConfidenceThresholds = async (): Promise<ConfidenceThresholds> => {
    try {
      const result = await invoke<ConfidenceThresholds>('agent_get_confidence_thresholds');
      return result;
    } catch (error) {
      console.error('Failed to get confidence thresholds:', error);
      throw error;
    }
  };

  const getConfidenceColor = (confidence: number): string => {
    if (confidence >= 0.9) return '#10b981'; // green
    if (confidence >= 0.7) return '#3b82f6'; // blue
    if (confidence >= 0.5) return '#f59e0b'; // amber
    if (confidence >= 0.3) return '#f97316'; // orange
    return '#ef4444'; // red
  };

  const getConfidenceLabel = (confidence: number): string => {
    if (confidence >= 0.9) return 'Very Confident';
    if (confidence >= 0.7) return 'Confident';
    if (confidence >= 0.5) return 'Moderate';
    if (confidence >= 0.3) return 'Low';
    return 'Very Low';
  };

  const getRiskLevel = (confidence: number): string => {
    if (confidence >= 0.8) return 'low';
    if (confidence >= 0.6) return 'medium';
    return 'high';
  };

  const shouldAutoExecute = (confidence: number): boolean => {
    return confidence >= 0.7;
  };

  const shouldAskUser = (confidence: number): boolean => {
    return confidence >= 0.3 && confidence < 0.7;
  };

  const shouldEscalate = (confidence: number): boolean => {
    return confidence < 0.3;
  };

  return {
    evaluateConfidence,
    calculateToolConfidence,
    assessDecisionRisk,
    getConfidenceThresholds,
    getConfidenceColor,
    getConfidenceLabel,
    getRiskLevel,
    shouldAutoExecute,
    shouldAskUser,
    shouldEscalate,
  };
};
