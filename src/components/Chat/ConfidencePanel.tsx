import React from "react";
import "./ConfidencePanel.css";

interface ConfidencePanelProps {
  confidence: {
    tool: string;
    score: number;
    level: string;
    emoji: string;
    reasons_for: string[];
    reasons_against: string[];
    recommendation: string;
  };
}

export const ConfidencePanel: React.FC<ConfidencePanelProps> = ({
  confidence,
}) => {
  return (
    <div className="confidence-panel">
      <div className="panel-header">
        <span className="emoji">{confidence.emoji}</span>
        <h3>{confidence.level} Confidence</h3>
        <span className="score-badge">
          {(confidence.score * 100).toFixed(0)}%
        </span>
      </div>

      <div className="panel-content">
        <div className="tool-info">
          <strong>Tool:</strong> <code>{confidence.tool}</code>
        </div>

        <div className="reasons-grid">
          <div className="reasons-for">
            <h4>✓ Reasons For:</h4>
            <ul>
              {confidence.reasons_for.map((reason, i) => (
                <li key={i}>{reason}</li>
              ))}
            </ul>
          </div>

          <div className="reasons-against">
            <h4>✗ Reasons Against:</h4>
            <ul>
              {confidence.reasons_against.map((reason, i) => (
                <li key={i}>{reason}</li>
              ))}
            </ul>
          </div>
        </div>

        <div className="recommendation-section">
          <strong>Recommendation:</strong>
          <p>{confidence.recommendation}</p>
        </div>
      </div>
    </div>
  );
};
