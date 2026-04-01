import React from "react";
import "./ContextIntegrationPanel.css";

interface ContextIntegrationPanelProps {
  context: {
    patterns_learned: number;
    context_relevance: number;
    suggestions: string[];
    knowledge_distilled: string;
    emoji: string;
  };
}

export const ContextIntegrationPanel: React.FC<
  ContextIntegrationPanelProps
> = ({ context }) => {
  return (
    <div className="context-integration-panel">
      <div className="panel-header">
        <span className="icon">🧠</span>
        <h3>Context Integration</h3>
        <span className="relevance-badge">
          {(context.context_relevance * 100).toFixed(0)}% relevant
        </span>
      </div>

      <div className="panel-content">
        <div className="patterns-section">
          <strong>Patterns Learned:</strong>
          <p className="patterns-count">{context.patterns_learned} patterns</p>
        </div>

        <div className="suggestions-section">
          <h4>Proactive Suggestions:</h4>
          <ul className="suggestions-list">
            {context.suggestions.map((suggestion, i) => (
              <li key={i}>{suggestion}</li>
            ))}
          </ul>
        </div>

        <div className="knowledge-section">
          <strong>Knowledge Distilled:</strong>
          <p className="knowledge-text">{context.knowledge_distilled}</p>
        </div>
      </div>
    </div>
  );
};
