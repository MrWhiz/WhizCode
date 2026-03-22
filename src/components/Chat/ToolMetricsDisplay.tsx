import React from 'react';

interface ToolRanking {
  tool_name: string;
  success_rate: number;
  reliability_score: number;
  avg_time_ms: number;
  rank: number;
  recommendation: string;
}

interface ToolMetricsDisplayProps {
  rankings: ToolRanking[];
  compact?: boolean;
}

const getSuccessRateColor = (successRate: number): string => {
  if (successRate >= 0.9) return '#10b981'; // green
  if (successRate >= 0.7) return '#3b82f6'; // blue
  if (successRate >= 0.5) return '#f59e0b'; // amber
  return '#ef4444'; // red
};

const getReliabilityIcon = (score: number): string => {
  if (score >= 0.9) return '⭐⭐⭐⭐⭐';
  if (score >= 0.7) return '⭐⭐⭐⭐';
  if (score >= 0.5) return '⭐⭐⭐';
  if (score >= 0.3) return '⭐⭐';
  return '⭐';
};

export const ToolMetricsDisplay: React.FC<ToolMetricsDisplayProps> = ({
  rankings,
  compact = false,
}) => {
  const [expanded, setExpanded] = React.useState(false);

  if (compact) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
        {rankings.slice(0, 3).map((ranking) => (
          <div
            key={ranking.tool_name}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              padding: '6px 8px',
              backgroundColor: 'rgba(0, 0, 0, 0.2)',
              borderRadius: '4px',
              fontSize: '11px',
            }}
          >
            <span style={{ fontWeight: 'bold', flex: 1 }}>{ranking.tool_name}</span>
            <div
              style={{
                width: '40px',
                height: '4px',
                backgroundColor: 'rgba(255, 255, 255, 0.1)',
                borderRadius: '2px',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  width: `${ranking.success_rate * 100}%`,
                  height: '100%',
                  backgroundColor: getSuccessRateColor(ranking.success_rate),
                }}
              />
            </div>
            <span style={{ color: getSuccessRateColor(ranking.success_rate), fontWeight: 'bold' }}>
              {(ranking.success_rate * 100).toFixed(0)}%
            </span>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div style={{ marginBottom: '16px' }}>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          cursor: 'pointer',
          padding: '12px',
          backgroundColor: 'rgba(139, 92, 246, 0.1)',
          border: '1px solid rgba(139, 92, 246, 0.3)',
          borderRadius: '8px',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          transition: 'all 0.2s ease',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.backgroundColor = 'rgba(139, 92, 246, 0.15)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.backgroundColor = 'rgba(139, 92, 246, 0.1)';
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ fontSize: '16px' }}>📊</span>
          <div>
            <div style={{ fontWeight: 'bold', fontSize: '13px', color: '#e0e0e0' }}>
              Tool Performance Metrics
            </div>
            <div style={{ fontSize: '11px', color: '#a0a0a0', marginTop: '2px' }}>
              {rankings.length} tools ranked by reliability
            </div>
          </div>
        </div>
        <span style={{ fontSize: '12px', color: '#a0a0a0' }}>
          {expanded ? '▼' : '▶'}
        </span>
      </div>

      {expanded && (
        <div style={{ marginTop: '12px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {rankings.map((ranking) => (
            <div
              key={ranking.tool_name}
              style={{
                padding: '12px',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                borderRadius: '6px',
                border: `1px solid ${getSuccessRateColor(ranking.success_rate)}20`,
              }}
            >
              {/* Header */}
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
                <div>
                  <div style={{ fontWeight: 'bold', fontSize: '12px', color: '#e0e0e0' }}>
                    #{ranking.rank} {ranking.tool_name}
                  </div>
                  <div style={{ fontSize: '10px', color: '#a0a0a0', marginTop: '2px' }}>
                    {ranking.recommendation}
                  </div>
                </div>
                <div style={{ fontSize: '14px' }}>
                  {getReliabilityIcon(ranking.reliability_score)}
                </div>
              </div>

              {/* Success Rate Bar */}
              <div style={{ marginBottom: '8px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
                  <span style={{ fontSize: '11px', color: '#a0a0a0' }}>Success Rate</span>
                  <span
                    style={{
                      fontSize: '11px',
                      fontWeight: 'bold',
                      color: getSuccessRateColor(ranking.success_rate),
                    }}
                  >
                    {(ranking.success_rate * 100).toFixed(1)}%
                  </span>
                </div>
                <div
                  style={{
                    width: '100%',
                    height: '6px',
                    backgroundColor: 'rgba(0, 0, 0, 0.3)',
                    borderRadius: '3px',
                    overflow: 'hidden',
                  }}
                >
                  <div
                    style={{
                      width: `${ranking.success_rate * 100}%`,
                      height: '100%',
                      backgroundColor: getSuccessRateColor(ranking.success_rate),
                      transition: 'width 0.3s ease',
                    }}
                  />
                </div>
              </div>

              {/* Reliability Score */}
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
                <span style={{ fontSize: '11px', color: '#a0a0a0' }}>Reliability</span>
                <span style={{ fontSize: '11px', fontWeight: 'bold', color: '#8b5cf6' }}>
                  {(ranking.reliability_score * 100).toFixed(0)}%
                </span>
              </div>

              {/* Execution Time */}
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: '#a0a0a0' }}>
                <span>Avg Time</span>
                <span style={{ fontWeight: 'bold', color: '#d0d0d0' }}>
                  {ranking.avg_time_ms}ms
                </span>
              </div>
            </div>
          ))}

          {rankings.length === 0 && (
            <div
              style={{
                padding: '16px',
                textAlign: 'center',
                color: '#a0a0a0',
                fontSize: '12px',
              }}
            >
              No tool metrics available yet
            </div>
          )}
        </div>
      )}
    </div>
  );
};
