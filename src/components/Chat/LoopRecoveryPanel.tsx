import React from "react";
import "./LoopRecoveryPanel.css";

interface LoopRecoveryPanelProps {
  guidance: {
    pattern: string;
    analysis: string;
    suggestions: string[];
    next_step: string;
    confidence: number;
  };
}

export const LoopRecoveryPanel: React.FC<LoopRecoveryPanelProps> = ({
  guidance,
}) => {
  return (
    <div className="loop-recovery-panel">
      <div className="panel-header">
        <span className="icon">🔄</span>
        <h3>Loop Detected: {guidance.pattern}</h3>
        <span className="confidence-badge">
          {(guidance.confidence * 100).toFixed(0)}% confident
        </span>
      </div>

      <div className="panel-content">
        <div className="analysis-section">
          <p className="analysis-text">{guidance.analysis}</p>
        </div>

        <div className="suggestions-section">
          <h4>Suggestions to Break the Loop:</h4>
          <ol className="suggestions-list">
            {guidance.suggestions.map((suggestion, i) => (
              <li key={i}>{suggestion}</li>
            ))}
          </ol>
        </div>

        <div className="next-step-section">
          <strong>Next Step:</strong>
          <p>{guidance.next_step}</p>
        </div>
      </div>
    </div>
  );
};
