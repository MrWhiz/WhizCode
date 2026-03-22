import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface RecoveryAttempt {
  error_type: string;
  strategy_id: string;
  success: boolean;
  timestamp: number;
  execution_time_ms: number;
}

export interface RecoveryResult {
  recovered: boolean;
  message: string;
  suggested_action?: string;
  fallback_recommendations: string[];
}

export interface RecoveryStrategy {
  id: string;
  error_pattern: string;
  recovery_steps: string[];
  success_rate: number;
}

export interface ErrorContext {
  error_type: string;
  message: string;
  tool: string;
  workspace_path?: string;
  timestamp: number;
}

export function useErrorRecovery() {
  const [isRecovering, setIsRecovering] = useState(false);
  const [recoveryLog, setRecoveryLog] = useState<RecoveryAttempt[]>([]);
  const [strategies, setStrategies] = useState<RecoveryStrategy[]>([]);

  // Auto-recover from an error
  const autoRecover = useCallback(
    async (error: string, tool: string, workspacePath?: string): Promise<RecoveryResult> => {
      setIsRecovering(true);
      try {
        const result = await invoke<RecoveryResult>('error_recovery_auto_recover', {
          error,
          tool,
          workspace_path: workspacePath,
        });
        return result;
      } finally {
        setIsRecovering(false);
      }
    },
    []
  );

  // Execute a specific recovery strategy
  const executeStrategy = useCallback(
    async (errorType: string, strategyId: string): Promise<RecoveryResult> => {
      setIsRecovering(true);
      try {
        const result = await invoke<RecoveryResult>('error_recovery_execute_strategy', {
          error_type: errorType,
          strategy_id: strategyId,
        });
        return result;
      } finally {
        setIsRecovering(false);
      }
    },
    []
  );

  // Get recovery log
  const getRecoveryLog = useCallback(async (limit?: number) => {
    try {
      const log = await invoke<RecoveryAttempt[]>('error_recovery_get_log', {
        limit,
      });
      setRecoveryLog(log);
      return log;
    } catch (error) {
      console.error('Failed to get recovery log:', error);
      return [];
    }
  }, []);

  // Get strategy effectiveness
  const getStrategyEffectiveness = useCallback(
    async (strategyId: string): Promise<number | null> => {
      try {
        const effectiveness = await invoke<number | null>(
          'error_recovery_strategy_effectiveness',
          {
            strategy_id: strategyId,
          }
        );
        return effectiveness;
      } catch (error) {
        console.error('Failed to get strategy effectiveness:', error);
        return null;
      }
    },
    []
  );

  // Update strategy success rates
  const updateStrategyRates = useCallback(async () => {
    try {
      await invoke('error_recovery_update_strategy_rates');
    } catch (error) {
      console.error('Failed to update strategy rates:', error);
    }
  }, []);

  // Get best strategy for error type
  const getBestStrategy = useCallback(
    async (errorType: string): Promise<RecoveryStrategy | null> => {
      try {
        const strategy = await invoke<RecoveryStrategy | null>(
          'error_recovery_best_strategy',
          {
            error_type: errorType,
          }
        );
        return strategy;
      } catch (error) {
        console.error('Failed to get best strategy:', error);
        return null;
      }
    },
    []
  );

  // Get all strategies
  const getStrategies = useCallback(async () => {
    try {
      const strats = await invoke<RecoveryStrategy[]>('error_recovery_strategies');
      setStrategies(strats);
      return strats;
    } catch (error) {
      console.error('Failed to get strategies:', error);
      return [];
    }
  }, []);

  return {
    isRecovering,
    recoveryLog,
    strategies,
    autoRecover,
    executeStrategy,
    getRecoveryLog,
    getStrategyEffectiveness,
    updateStrategyRates,
    getBestStrategy,
    getStrategies,
  };
}
