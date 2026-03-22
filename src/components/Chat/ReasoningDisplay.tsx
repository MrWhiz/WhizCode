import React from 'react';

interface ReasoningStep {
  step_number: number;
  phase: string;
  reasoning: string;
  confidence: number;
  alternatives_considered: string[];
  decision?: string;
}

interface CoTResponse {
  reasoning_steps: ReasoningStep[];
  final_decision: string;
  overall_confidence: number;
  reasoning_trace: string;
  execution_plan: string[];
}

interface ReasoningDisplayProps {
  cot: CoTResponse;
}

const getPhaseColor = (phase: string): string => {
  const colors: Record<string, string> = {
    analysis: '#3b82f6',      // blue
    hypothesis: '#8b5cf6',    // purple
    validation: '#ec4899',    // pink
    conclusion: '#10b981',    // green
  };
  return colors[phase] || '#6b7280';
};

const getPhaseIcon = (phase: string): string => {
  const icons: Record<string, string> = {
    analysis: '🔍',
    hypothesis: '💡',
    validation: '✓',
    conclusion: '🎯',
  };
  return icons[phase] || '•';
};

const getConfidenceColor = (confidence: number): string => {
  if (confidence >= 0.8) return '#10b981'; // green
  if (confidence >= 0.6) return '#f59e0b'; // amber
  return '#ef4444'; // red
};

const getConfidenceLabel = (confidence: number): string => {
  if (confidence >= 0.9) return 'Very Confident';
  if (confidence >= 0.7) return 'Confident';
  if (confidence >= 0.5) return 'Moderate';
  if (confidence >= 0.3) return 'Low';
  return 'Very Low';
};

export const ReasoningDisplay: React.FC<ReasoningDisplayProps> = ({ cot }) => {
  const [expanded, setExpanded] = React.useState(false);

  return (
    <div style={{ marginBottom: '16px' }}>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          cursor: 'pointer',
          padding: '12px',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          border: '1px solid rgba(59, 130, 246, 0.3)',
          borderRadius: '8px',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          transition: 'all 0.2s ease',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.backgroundColor = 'rgba(59, 130, 246, 0.15)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.backgroundColor = 'rgba(59, 130, 246, 0.1)';
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ fontSize: '16px' }}>🧠</span>
          <div>
            <div style={{ fontWeight: 'bold', fontSize: '13px', color: '#e0e0e0' }}>
              Chain-of-Thought Reasoning
            </div>
            <div style={{ fontSize: '11px', color: '#a0a0a0', marginTop: '2px' }}>
              {cot.reasoning_steps.length} phases analyzed
            </div>
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div
            style={{
              padding: '4px 8px',
              backgroundColor: getConfidenceColor(cot.overall_confidence),
              borderRadius: '4px',
              fontSize: '11px',
              fontWeight: 'bold',
              color: '#fff',
            }}
          >
            {(cot.overall_confidence * 100).toFixed(0)}%
          </div>
          <span style={{ fontSize: '12px', color: '#a0a0a0' }}>
            {expanded ? '▼' : '▶'}
          </span>
        </div>
      </div>

      {expanded && (
        <div style={{ marginTop: '12px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
          {/* Reasoning Steps */}
          {cot.reasoning_steps.map((step) => (
            <div
              key={step.step_number}
              style={{
                paddingLeft: '16px',
                borderLeft: `3px solid ${getPhaseColor(step.phase)}`,
                paddingTop: '12px',
                paddingBottom: '12px',
                paddingRight: '12px',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                borderRadius: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' }}>
                <span style={{ fontSize: '16px' }}>{getPhaseIcon(step.phase)}</span>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 'bold', fontSize: '12px', textTransform: 'uppercase', color: getPhaseColor(step.phase) }}>
                    {step.phase}
                  </div>
                </div>
                <div
                  style={{
                    padding: '2px 6px',
                    backgroundColor: getConfidenceColor(step.confidence),
                    borderRadius: '3px',
                    fontSize: '10px',
                    fontWeight: 'bold',
                    color: '#fff',
                  }}
                >
                  {(step.confidence * 100).toFixed(0)}%
                </div>
              </div>

              <div style={{ fontSize: '13px', lineHeight: '1.5', color: '#d0d0d0', marginBottom: '8px' }}>
                {step.reasoning}
              </div>

              {step.decision && (
                <div
                  style={{
                    fontSize: '12px',
                    fontStyle: 'italic',
                    color: '#a0d0ff',
                    backgroundColor: 'rgba(59, 130, 246, 0.1)',
                    padding: '6px 8px',
                    borderRadius: '3px',
                    marginBottom: '8px',
                  }}
                >
                  <strong>Decision:</strong> {step.decision}
                </div>
              )}

              {step.alternatives_considered.length > 0 && (
                <div style={{ fontSize: '11px', color: '#909090' }}>
                  <strong>Alternatives:</strong> {step.alternatives_considered.join(', ')}
                </div>
              )}
            </div>
          ))}

          {/* Final Decision */}
          <div
            style={{
              padding: '12px',
              backgroundColor: 'rgba(16, 185, 129, 0.1)',
              border: '1px solid rgba(16, 185, 129, 0.3)',
              borderRadius: '4px',
            }}
          >
            <div style={{ fontWeight: 'bold', marginBottom: '8px', fontSize: '12px', color: '#10b981' }}>
              ✓ Final Decision
            </div>
            <div style={{ fontSize: '13px', lineHeight: '1.5', color: '#d0d0d0' }}>
              {cot.final_decision}
            </div>
          </div>

          {/* Execution Plan */}
          {cot.execution_plan.length > 0 && (
            <div
              style={{
                padding: '12px',
                backgroundColor: 'rgba(139, 92, 246, 0.1)',
                border: '1px solid rgba(139, 92, 246, 0.3)',
                borderRadius: '4px',
              }}
            >
              <div style={{ fontWeight: 'bold', marginBottom: '8px', fontSize: '12px', color: '#8b5cf6' }}>
                📋 Execution Plan
              </div>
              <ol style={{ margin: 0, paddingLeft: '20px', fontSize: '12px', color: '#d0d0d0' }}>
                {cot.execution_plan.map((step, idx) => (
                  <li key={idx} style={{ marginBottom: '4px' }}>
                    {step}
                  </li>
                ))}
              </ol>
            </div>
          )}

          {/* Confidence Summary */}
          <div
            style={{
              padding: '12px',
              backgroundColor: 'rgba(0, 0, 0, 0.3)',
              borderRadius: '4px',
              fontSize: '11px',
              color: '#a0a0a0',
            }}
          >
            <div style={{ marginBottom: '6px' }}>
              <strong>Overall Confidence:</strong> {getConfidenceLabel(cot.overall_confidence)} ({(cot.overall_confidence * 100).toFixed(1)}%)
            </div>
            {cot.overall_confidence < 0.7 && (
              <div style={{ color: '#f59e0b', marginTop: '6px' }}>
                ⚠️ Low confidence - Human review recommended
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
