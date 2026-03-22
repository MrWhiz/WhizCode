import React from 'react';

interface ConfidenceIndicatorProps {
  confidence: number;
  riskLevel: string;
  action: string;
  reasoning: string;
  uncertaintyFactors?: string[];
  compact?: boolean;
}

const getConfidenceColor = (confidence: number): string => {
  if (confidence >= 0.9) return '#10b981'; // green
  if (confidence >= 0.7) return '#3b82f6'; // blue
  if (confidence >= 0.5) return '#f59e0b'; // amber
  if (confidence >= 0.3) return '#f97316'; // orange
  return '#ef4444'; // red
};

const getConfidenceLabel = (confidence: number): string => {
  if (confidence >= 0.9) return 'Very Confident';
  if (confidence >= 0.7) return 'Confident';
  if (confidence >= 0.5) return 'Moderate';
  if (confidence >= 0.3) return 'Low';
  return 'Very Low';
};

const getRiskIcon = (riskLevel: string): string => {
  switch (riskLevel) {
    case 'low':
      return '✓';
    case 'medium':
      return '⚠️';
    case 'high':
      return '🚨';
    case 'critical':
      return '❌';
    default:
      return '•';
  }
};

const getActionLabel = (action: string): string => {
  switch (action) {
    case 'auto_execute':
      return 'Auto Execute';
    case 'ask_user':
      return 'Ask User';
    case 'escalate':
      return 'Escalate';
    default:
      return action;
  }
};

export const ConfidenceIndicator: React.FC<ConfidenceIndicatorProps> = ({
  confidence,
  riskLevel,
  action,
  reasoning,
  uncertaintyFactors = [],
  compact = false,
}) => {
  const color = getConfidenceColor(confidence);
  const label = getConfidenceLabel(confidence);
  const riskIcon = getRiskIcon(riskLevel);
  const actionLabel = getActionLabel(action);

  if (compact) {
    return (
      <div
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '6px',
          padding: '4px 8px',
          backgroundColor: `${color}20`,
          border: `1px solid ${color}`,
          borderRadius: '4px',
          fontSize: '11px',
          fontWeight: 'bold',
          color: color,
        }}
      >
        <span>{riskIcon}</span>
        <span>{(confidence * 100).toFixed(0)}%</span>
      </div>
    );
  }

  return (
    <div
      style={{
        padding: '12px',
        backgroundColor: `${color}10`,
        border: `1px solid ${color}40`,
        borderRadius: '6px',
        marginBottom: '12px',
      }}
    >
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '8px' }}>
        <div
          style={{
            fontSize: '20px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '32px',
            height: '32px',
            backgroundColor: `${color}20`,
            borderRadius: '4px',
          }}
        >
          {riskIcon}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: '12px', fontWeight: 'bold', color: color }}>
            {label}
          </div>
          <div style={{ fontSize: '11px', color: '#a0a0a0', marginTop: '2px' }}>
            {actionLabel}
          </div>
        </div>
        <div
          style={{
            fontSize: '18px',
            fontWeight: 'bold',
            color: color,
          }}
        >
          {(confidence * 100).toFixed(0)}%
        </div>
      </div>

      {/* Confidence Bar */}
      <div
        style={{
          width: '100%',
          height: '6px',
          backgroundColor: 'rgba(0, 0, 0, 0.2)',
          borderRadius: '3px',
          overflow: 'hidden',
          marginBottom: '8px',
        }}
      >
        <div
          style={{
            width: `${confidence * 100}%`,
            height: '100%',
            backgroundColor: color,
            transition: 'width 0.3s ease',
          }}
        />
      </div>

      {/* Reasoning */}
      <div style={{ fontSize: '12px', color: '#d0d0d0', marginBottom: '8px', lineHeight: '1.4' }}>
        {reasoning}
      </div>

      {/* Risk Level Badge */}
      <div
        style={{
          display: 'inline-block',
          padding: '4px 8px',
          backgroundColor: `${color}20`,
          border: `1px solid ${color}`,
          borderRadius: '3px',
          fontSize: '10px',
          fontWeight: 'bold',
          color: color,
          textTransform: 'uppercase',
          marginBottom: '8px',
        }}
      >
        Risk: {riskLevel}
      </div>

      {/* Uncertainty Factors */}
      {uncertaintyFactors.length > 0 && (
        <div style={{ marginTop: '8px', paddingTop: '8px', borderTop: `1px solid ${color}20` }}>
          <div style={{ fontSize: '10px', fontWeight: 'bold', color: '#a0a0a0', marginBottom: '4px' }}>
            Uncertainty Factors:
          </div>
          <ul
            style={{
              margin: 0,
              paddingLeft: '16px',
              fontSize: '11px',
              color: '#909090',
            }}
          >
            {uncertaintyFactors.map((factor, idx) => (
              <li key={idx} style={{ marginBottom: '2px' }}>
                {factor}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Action Recommendation */}
      {action === 'ask_user' && (
        <div
          style={{
            marginTop: '8px',
            padding: '8px',
            backgroundColor: '#f59e0b20',
            border: '1px solid #f59e0b40',
            borderRadius: '4px',
            fontSize: '11px',
            color: '#fbbf24',
          }}
        >
          ⚠️ This decision requires your approval before proceeding.
        </div>
      )}

      {action === 'escalate' && (
        <div
          style={{
            marginTop: '8px',
            padding: '8px',
            backgroundColor: '#ef444420',
            border: '1px solid #ef444440',
            borderRadius: '4px',
            fontSize: '11px',
            color: '#fca5a5',
          }}
        >
          ❌ Low confidence - Please review and provide guidance.
        </div>
      )}

      {action === 'auto_execute' && (
        <div
          style={{
            marginTop: '8px',
            padding: '8px',
            backgroundColor: '#10b98120',
            border: '1px solid #10b98140',
            borderRadius: '4px',
            fontSize: '11px',
            color: '#86efac',
          }}
        >
          ✓ High confidence - Proceeding autonomously.
        </div>
      )}
    </div>
  );
};
