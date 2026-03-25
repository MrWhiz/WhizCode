/**
 * WhizCode Frontend Integration
 * Handles WhizCode events and metrics from the backend
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface PhaseChangeEvent {
  phase: string;
  status: 'started' | 'completed' | 'failed';
  description: string;
}

export interface MetricsEvent {
  total_tokens: number;
  tokens_per_second: number;
  estimated_time_remaining: number;
  current_phase: string;
}

export class WhizCodeIntegration {
  private phaseListeners: UnlistenFn[] = [];
  private metricsListeners: UnlistenFn[] = [];

  /**
   * Listen for phase change events
   */
  async onPhaseChange(callback: (event: PhaseChangeEvent) => void): Promise<UnlistenFn> {
    const unlisten = await listen<PhaseChangeEvent>('agent:phase', (event) => {
      callback(event.payload);
    });
    this.phaseListeners.push(unlisten);
    return unlisten;
  }

  /**
   * Listen for metrics events
   */
  async onMetrics(callback: (event: MetricsEvent) => void): Promise<UnlistenFn> {
    const unlisten = await listen<MetricsEvent>('agent:metrics', (event) => {
      callback(event.payload);
    });
    this.metricsListeners.push(unlisten);
    return unlisten;
  }

  /**
   * Clean up all listeners
   */
  cleanup(): void {
    this.phaseListeners.forEach(unlisten => unlisten());
    this.metricsListeners.forEach(unlisten => unlisten());
    this.phaseListeners = [];
    this.metricsListeners = [];
  }

  /**
   * Get phase emoji
   */
  getPhaseEmoji(phase: string): string {
    const emojis: Record<string, string> = {
      'planning': '📋',
      'research': '🔎',
      'query_analysis': '🔍',
      'workflow_routing': '🛣️',
      'steering': '🧭',
      'context_building': '🏗️',
      'context_optimization': '✂️',
      'prompt_optimization': '✨',
      'streaming': '⚡',
      'execution': '⚙️',
      'analyzing': '🔍',
      'thinking': '🧠',
      'processing': '⏳',
      'generating': '✨',
      'loading': '📥',
    };
    return emojis[phase] || '•';
  }

  /**
   * Format time remaining
   */
  formatTimeRemaining(seconds: number): string {
    if (seconds < 60) {
      return `${seconds}s`;
    }
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  }

  /**
   * Format tokens per second
   */
  formatTokensPerSecond(tps: number): string {
    return `${tps.toFixed(1)} tok/s`;
  }
}

export const whizCodeIntegration = new WhizCodeIntegration();
