import React, { useState, useEffect } from 'react';
import { useErrorRecovery } from '../../hooks/useErrorRecovery';
import type { RecoveryAttempt, RecoveryStrategy } from '../../hooks/useErrorRecovery';
import './ErrorRecoveryPanel.css';

interface ErrorRecoveryPanelProps {
  error?: string;
  tool?: string;
  workspacePath?: string;
  onRecovered?: (result: any) => void;
}

export const ErrorRecoveryPanel: React.FC<ErrorRecoveryPanelProps> = ({
  error,
  tool,
  workspacePath,
  onRecovered,
}) => {
  const {
    isRecovering,
    recoveryLog,
    strategies,
    autoRecover,
    getRecoveryLog,
    getStrategies,
    getBestStrategy,
  } = useErrorRecovery();

  const [bestStrategy, setBestStrategy] = useState<RecoveryStrategy | null>(null);
  const [showLog, setShowLog] = useState(false);
  const [recoveryMessage, setRecoveryMessage] = useState<string>('');

  useEffect(() => {
    getStrategies();
    getRecoveryLog(10);
  }, [getStrategies, getRecoveryLog]);

  useEffect(() => {
    if (error) {
      const errorType = error.toLowerCase();
      getBestStrategy(errorType).then(setBestStrategy);
    }
  }, [error, getBestStrategy]);

  const handleAutoRecover = async () => {
    if (!error || !tool) return;

    try {
      const result = await autoRecover(error, tool, workspacePath);
      setRecoveryMessage(result.message);

      if (result.recovered) {
        onRecovered?.(result);
      }
    } catch (err) {
      setRecoveryMessage(`Recovery failed: ${err}`);
    }
  };

  const getSuccessRateColor = (rate: number): string => {
    if (rate >= 0.8) return '#10b981'; // green
    if (rate >= 0.6) return '#f59e0b'; // amber
    return '#ef4444'; // red
  };

  return (
    <div className="error-recovery-panel">
      {error && (
        <div className="recovery-section">
          <div className="recovery-header">
            <span className="recovery-icon">⚠️</span>
            <span className="recovery-title">Error Recovery</span>
          </div>

          <div className="error-info">
            <div className="error-message">{error}</div>
            {tool && <div className="error-tool">Tool: {tool}</div>}
          </div>

          {bestStrategy && (
            <div className="best-strategy">
              <div className="strategy-label">Recommended Strategy:</div>
              <div className="strategy-name">{bestStrategy.id}</div>
              <div className="strategy-steps">
                {bestStrategy.recovery_steps.map((step: string, idx: number) => (
                  <div key={idx} className="strategy-step">
                    {idx + 1}. {step}
                  </div>
                ))}
              </div>
              <div
                className="strategy-success-rate"
                style={{ color: getSuccessRateColor(bestStrategy.success_rate) }}
              >
                Success Rate: {(bestStrategy.success_rate * 100).toFixed(1)}%
              </div>
            </div>
          )}

          <button
            className="auto-recover-button"
            onClick={handleAutoRecover}
            disabled={isRecovering}
          >
            {isRecovering ? (
              <>
                <span className="spinner">⟳</span> Attempting Recovery...
              </>
            ) : (
              <>
                <span className="icon">🔧</span> Auto-Recover
              </>
            )}
          </button>

          {recoveryMessage && (
            <div className={`recovery-message ${recoveryMessage.includes('failed') ? 'error' : 'success'}`}>
              {recoveryMessage}
            </div>
          )}
        </div>
      )}

      <div className="recovery-stats">
        <div className="stats-header">
          <span>Recovery Statistics</span>
          <button
            className="toggle-log-button"
            onClick={() => setShowLog(!showLog)}
          >
            {showLog ? '▼' : '▶'} Recent Attempts ({recoveryLog.length})
          </button>
        </div>

        {showLog && recoveryLog.length > 0 && (
          <div className="recovery-log">
            {recoveryLog.map((attempt: RecoveryAttempt, idx: number) => (
              <div key={idx} className={`log-entry ${attempt.success ? 'success' : 'failed'}`}>
                <div className="log-status">
                  {attempt.success ? '✓' : '✗'} {attempt.error_type}
                </div>
                <div className="log-strategy">{attempt.strategy_id}</div>
                <div className="log-time">{attempt.execution_time_ms}ms</div>
              </div>
            ))}
          </div>
        )}

        {strategies.length > 0 && (
          <div className="strategies-summary">
            <div className="summary-label">Available Strategies: {strategies.length}</div>
            <div className="strategies-grid">
              {strategies.slice(0, 5).map((strategy: RecoveryStrategy) => (
                <div key={strategy.id} className="strategy-badge">
                  <div className="badge-name">{strategy.id}</div>
                  <div
                    className="badge-rate"
                    style={{ color: getSuccessRateColor(strategy.success_rate) }}
                  >
                    {(strategy.success_rate * 100).toFixed(0)}%
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
