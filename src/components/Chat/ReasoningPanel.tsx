import React from "react";
import "./ReasoningPanel.css";

interface ReasoningPanelProps {
  reasoning: {
    action: string;
    why: string;
    expected_outcome: string;
    alternatives: string[];
    risks: string[];
    emoji: string;
  };
}

export const ReasoningPanel: React.FC<ReasoningPanelProps> = ({
  reasoning,
}) => {
  return (
    <div className="reasoning-panel">
      <div className="panel-header">
        <span className="emoji">{reasoning.emoji}</span>
        <h3>Reasoning</h3>
      </div>

      <div className="panel-content">
        <div className="action-section">
          <strong>Action:</strong>
          <p className="action-text">{reasoning.action}</p>
        </div>

        <div className="why-section">
          <strong>Why:</strong>
          <p className="why-text">{reasoning.why}</p>
        </div>

        <div className="outcome-section">
          <strong>Expected Outcome:</strong>
          <p className="outcome-text">{reasoning.expected_outcome}</p>
        </div>

        <div className="alternatives-section">
          <h4>Alternative Approaches:</h4>
          <ul className="alternatives-list">
            {reasoning.alternatives.map((alt, i) => (
              <li key={i}>{alt}</li>
            ))}
          </ul>
        </div>

        <div className="risks-section">
          <h4>⚠️ Potential Risks:</h4>
          <ul className="risks-list">
            {reasoning.risks.map((risk, i) => (
              <li key={i}>{risk}</li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
};
