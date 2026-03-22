import React from 'react';

interface StreamingStatusProps {
  isActive: boolean;
  currentPhase: string;
  phases: string[];
  elapsedSeconds: number;
}

const phaseEmojis: Record<string, string> = {
  'analyzing': '🔍',
  'planning': '📋',
  'researching': '🔎',
  'executing': '⚙️',
  'validating': '✓',
  'thinking': '🧠',
  'processing': '⏳',
  'generating': '✨',
  'loading': '📥',
  'default': '•',
};

const getPhaseEmoji = (phase: string): string => {
  const lower = phase.toLowerCase();
  for (const [key, emoji] of Object.entries(phaseEmojis)) {
    if (lower.includes(key)) return emoji;
  }
  return phaseEmojis['default'];
};

const AnimatedDots: React.FC<{ count?: number }> = ({ count = 3 }) => {
  const [dots, setDots] = React.useState('');

  React.useEffect(() => {
    const interval = setInterval(() => {
      setDots((prev) => {
        const next = prev.length < count ? prev + '.' : '';
        return next;
      });
    }, 300);
    return () => clearInterval(interval);
  }, [count]);

  return <span>{dots}</span>;
};

export const StreamingStatus: React.FC<StreamingStatusProps> = ({
  isActive,
  currentPhase,
  phases,
  elapsedSeconds,
}) => {
  if (!isActive) return null;

  const formatTime = (seconds: number): string => {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        padding: '10px 12px',
        backgroundColor: 'rgba(59, 130, 246, 0.08)',
        border: '1px solid rgba(59, 130, 246, 0.2)',
        borderRadius: '6px',
        marginBottom: '10px',
        fontSize: '12px',
        color: '#a0a0a0',
      }}
    >
      {/* Animated pulse indicator */}
      <div
        style={{
          width: '8px',
          height: '8px',
          borderRadius: '50%',
          background: '#3b82f6',
          boxShadow: '0 0 8px #3b82f6',
          animation: 'pulse 1.5s ease-in-out infinite',
          flexShrink: 0,
        }}
      />

      {/* Phase indicator */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1 }}>
        <span style={{ fontSize: '14px' }}>{getPhaseEmoji(currentPhase)}</span>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          <div style={{ fontWeight: '600', color: '#3b82f6', fontSize: '11px', textTransform: 'uppercase' }}>
            {currentPhase}
            <AnimatedDots />
          </div>
          {phases.length > 0 && (
            <div style={{ fontSize: '10px', color: '#707070' }}>
              {phases.map((p, i) => (
                <span key={i}>
                  {i > 0 && ' → '}
                  <span style={{ opacity: 0.6 }}>{p}</span>
                </span>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Timer */}
      <div
        style={{
          padding: '2px 6px',
          backgroundColor: 'rgba(59, 130, 246, 0.15)',
          borderRadius: '3px',
          fontSize: '10px',
          fontWeight: 'bold',
          color: '#3b82f6',
          whiteSpace: 'nowrap',
        }}
      >
        {formatTime(elapsedSeconds)}
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% {
            opacity: 1;
          }
          50% {
            opacity: 0.5;
          }
        }
      `}</style>
    </div>
  );
};

export const StreamingPhaseIndicator: React.FC<{ phases: string[] }> = ({ phases }) => {
  return (
    <div
      style={{
        display: 'flex',
        gap: '4px',
        alignItems: 'center',
        fontSize: '11px',
        color: '#a0a0a0',
      }}
    >
      {phases.map((phase, idx) => (
        <React.Fragment key={idx}>
          {idx > 0 && <span style={{ color: '#606060' }}>→</span>}
          <span
            style={{
              padding: '2px 6px',
              backgroundColor: 'rgba(59, 130, 246, 0.1)',
              borderRadius: '3px',
              whiteSpace: 'nowrap',
            }}
          >
            {getPhaseEmoji(phase)} {phase}
          </span>
        </React.Fragment>
      ))}
    </div>
  );
};

export const StreamingProgressBar: React.FC<{ progress: number; label?: string }> = ({
  progress,
  label,
}) => {
  return (
    <div style={{ marginBottom: '8px' }}>
      {label && (
        <div style={{ fontSize: '11px', color: '#a0a0a0', marginBottom: '4px' }}>
          {label}
        </div>
      )}
      <div
        style={{
          width: '100%',
          height: '4px',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          borderRadius: '2px',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            width: `${Math.min(progress * 100, 100)}%`,
            height: '100%',
            backgroundColor: '#3b82f6',
            transition: 'width 0.3s ease',
            boxShadow: '0 0 8px rgba(59, 130, 246, 0.5)',
          }}
        />
      </div>
    </div>
  );
};
