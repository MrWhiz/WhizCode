import { invoke } from '@tauri-apps/api/core';

export interface ToolMetrics {
  name: string;
  category: string;
  success_count: number;
  failure_count: number;
  total_executions: number;
  success_rate: number;
  avg_execution_time_ms: number;
  min_execution_time_ms: number;
  max_execution_time_ms: number;
  failure_modes: string[];
  prerequisites: string[];
  post_conditions: string[];
  cost_estimate: number;
  last_used: number;
  reliability_score: number;
}

export interface ToolExecution {
  tool_name: string;
  timestamp: number;
  duration_ms: number;
  success: boolean;
  error_message?: string;
  input_tokens: number;
  output_tokens: number;
}

export interface ToolRanking {
  tool_name: string;
  success_rate: number;
  reliability_score: number;
  avg_time_ms: number;
  rank: number;
  recommendation: string;
}

export interface ToolStatistics {
  total_tools: number;
  total_executions: number;
  total_successes: number;
  total_failures: number;
  average_success_rate: number;
  average_execution_time_ms: number;
  history_size: number;
}

export const useToolMetrics = () => {
  const recordExecution = async (
    toolName: string,
    durationMs: number,
    success: boolean,
    errorMessage?: string,
    inputTokens: number = 0,
    outputTokens: number = 0
  ): Promise<void> => {
    try {
      await invoke('tool_metrics_record_execution', {
        toolName,
        durationMs,
        success,
        errorMessage,
        inputTokens,
        outputTokens,
      });
    } catch (error) {
      console.error('Failed to record tool execution:', error);
      throw error;
    }
  };

  const getMetrics = async (toolName: string): Promise<ToolMetrics | null> => {
    try {
      const result = await invoke<ToolMetrics | null>('tool_metrics_get_metrics', {
        toolName,
      });
      return result;
    } catch (error) {
      console.error('Failed to get tool metrics:', error);
      throw error;
    }
  };

  const getAllMetrics = async (): Promise<ToolMetrics[]> => {
    try {
      const result = await invoke<ToolMetrics[]>('tool_metrics_get_all');
      return result;
    } catch (error) {
      console.error('Failed to get all metrics:', error);
      throw error;
    }
  };

  const rankTools = async (): Promise<ToolRanking[]> => {
    try {
      const result = await invoke<ToolRanking[]>('tool_metrics_rank_tools');
      return result;
    } catch (error) {
      console.error('Failed to rank tools:', error);
      throw error;
    }
  };

  const getRecommendations = async (taskType: string): Promise<ToolRanking[]> => {
    try {
      const result = await invoke<ToolRanking[]>('tool_metrics_get_recommendations', {
        taskType,
      });
      return result;
    } catch (error) {
      console.error('Failed to get recommendations:', error);
      throw error;
    }
  };

  const getHistory = async (limit?: number): Promise<ToolExecution[]> => {
    try {
      const result = await invoke<ToolExecution[]>('tool_metrics_get_history', {
        limit,
      });
      return result;
    } catch (error) {
      console.error('Failed to get execution history:', error);
      throw error;
    }
  };

  const getFailureAnalysis = async (toolName: string): Promise<any> => {
    try {
      const result = await invoke('tool_metrics_get_failure_analysis', {
        toolName,
      });
      return result;
    } catch (error) {
      console.error('Failed to get failure analysis:', error);
      throw error;
    }
  };

  const getStatistics = async (): Promise<ToolStatistics> => {
    try {
      const result = await invoke<ToolStatistics>('tool_metrics_get_statistics');
      return result;
    } catch (error) {
      console.error('Failed to get statistics:', error);
      throw error;
    }
  };

  const clearMetrics = async (): Promise<void> => {
    try {
      await invoke('tool_metrics_clear');
    } catch (error) {
      console.error('Failed to clear metrics:', error);
      throw error;
    }
  };

  // Helper functions
  const getSuccessRateColor = (successRate: number): string => {
    if (successRate >= 0.9) return '#10b981'; // green
    if (successRate >= 0.7) return '#3b82f6'; // blue
    if (successRate >= 0.5) return '#f59e0b'; // amber
    return '#ef4444'; // red
  };

  const getReliabilityLabel = (score: number): string => {
    if (score >= 0.9) return 'Excellent';
    if (score >= 0.7) return 'Good';
    if (score >= 0.5) return 'Fair';
    return 'Poor';
  };

  const shouldUseToolByDefault = (ranking: ToolRanking): boolean => {
    return ranking.success_rate >= 0.8 && ranking.avg_time_ms < 2000;
  };

  return {
    recordExecution,
    getMetrics,
    getAllMetrics,
    rankTools,
    getRecommendations,
    getHistory,
    getFailureAnalysis,
    getStatistics,
    clearMetrics,
    getSuccessRateColor,
    getReliabilityLabel,
    shouldUseToolByDefault,
  };
};
