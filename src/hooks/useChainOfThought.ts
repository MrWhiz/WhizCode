import { invoke } from '@tauri-apps/api/core';

export interface ReasoningStep {
  step_number: number;
  phase: string;
  reasoning: string;
  confidence: number;
  alternatives_considered: string[];
  decision?: string;
}

export interface CoTResponse {
  reasoning_steps: ReasoningStep[];
  final_decision: string;
  overall_confidence: number;
  reasoning_trace: string;
  execution_plan: string[];
}

export interface CoTValidationResult {
  valid: boolean;
  cot_response?: CoTResponse;
  overall_confidence?: number;
  requires_review?: boolean;
  error?: string;
}

export const useChainOfThought = () => {
  const reasonWithCoT = async (
    task: string,
    model: { model: string },
    workspacePath?: string,
    activeFile?: { path: string }
  ): Promise<CoTResponse> => {
    try {
      const result = await invoke<CoTResponse>('agent_reasoning_with_cot', {
        task,
        model,
        workspacePath,
        activeFile,
      });
      return result;
    } catch (error) {
      console.error('CoT reasoning failed:', error);
      throw error;
    }
  };

  const validateCoTResponse = async (response: string): Promise<CoTValidationResult> => {
    try {
      const result = await invoke<CoTValidationResult>('agent_validate_cot_response', {
        response,
      });
      return result;
    } catch (error) {
      console.error('CoT validation failed:', error);
      throw error;
    }
  };

  const getCoTMetrics = async () => {
    try {
      const result = await invoke('agent_get_cot_metrics');
      return result;
    } catch (error) {
      console.error('Failed to get CoT metrics:', error);
      throw error;
    }
  };

  return {
    reasonWithCoT,
    validateCoTResponse,
    getCoTMetrics,
  };
};
