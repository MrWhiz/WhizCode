import React from 'react';

interface EnhancedThinkingIndicatorProps {
  isThinking: boolean;
  currentPhase?: string;
  phases?: string[];
  elapsedSeconds?: number;
  streamingText?: string;
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
  'reading': '📖',
  'writing': '✍️',
  'compiling': '🔨',
  'testing': '🧪',
};

const getPhaseEmoji = (phase?: string): string => {
  if (!phase) return '🧠';
  const lower = phase.toLowerCase();
  for (const [key, emoji] of Object.entries(phaseEmojis)) {
    if (lower.includes(key)) return emoji;
  }
  return '•';
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

export const EnhancedThinkingIndicator: React.FC<EnhancedThinkingIndicatorProps> = ({
  isThinking,
  currentPhase = 'thinking',
  phases = [],
  elapsedSeconds = 0,
  streamingText = '',
}) => {
  if (!isThinking) return null;

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
        flexDirection: 'column',
        gap: '8px',
        padding: '12px',
        backgroundColor: 'rgba(59, 130, 246, 0.08)',
        border: '1px solid rgba(59, 130, 246, 0.2)',
        borderRadius: '6px',
        marginBottom: '10px',
      }}
    >
      {/* Main thinking indicator */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        {/* Pulse indicator */}
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

        {/* Phase info */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1 }}>
          <span style={{ fontSize: '14px' }}>{getPhaseEmoji(currentPhase)}</span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', flex: 1 }}>
            <div
              style={{
                fontWeight: '600',
                color: '#3b82f6',
                fontSize: '11px',
                textTransform: 'uppercase',
                letterSpacing: '0.5px',
              }}
            >
              {currentPhase}
              <AnimatedDots />
            </div>

            {/* Streaming text */}
            {streamingText && (
              <div
                style={{
                  fontSize: '11px',
                  color: '#a0a0a0',
                  fontStyle: 'italic',
                  maxWidth: '300px',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {streamingText}
              </div>
            )}
          </div>
        </div>

        {/* Timer */}
        {elapsedSeconds > 0 && (
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
        )}
      </div>

      {/* Phase progress */}
      {phases.length > 0 && (
        <div
          style={{
            display: 'flex',
            gap: '4px',
            flexWrap: 'wrap',
            fontSize: '10px',
          }}
        >
          {phases.map((phase, idx) => (
            <div
              key={idx}
              style={{
                padding: '2px 6px',
                backgroundColor: idx === phases.length - 1 ? 'rgba(59, 130, 246, 0.2)' : 'rgba(59, 130, 246, 0.1)',
                border: idx === phases.length - 1 ? '1px solid rgba(59, 130, 246, 0.4)' : 'none',
                borderRadius: '3px',
                color: idx === phases.length - 1 ? '#3b82f6' : '#a0a0a0',
                fontWeight: idx === phases.length - 1 ? '600' : '400',
                whiteSpace: 'nowrap',
              }}
            >
              {getPhaseEmoji(phase)} {phase}
            </div>
          ))}
        </div>
      )}

      {/* Progress bar */}
      <div
        style={{
          width: '100%',
          height: '3px',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          borderRadius: '2px',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            width: '100%',
            height: '100%',
            background: 'linear-gradient(90deg, #3b82f6, #60a5fa, #3b82f6)',
            backgroundSize: '200% 100%',
            animation: 'shimmer 2s infinite',
          }}
        />
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

        @keyframes shimmer {
          0% {
            background-position: 200% 0;
          }
          100% {
            background-position: -200% 0;
          }
        }
      `}</style>
    </div>
  );
};

export const SimpleThinkingIndicator: React.FC<{ isThinking: boolean }> = ({ isThinking }) => {
  if (!isThinking) return null;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        padding: '8px 12px',
        backgroundColor: 'rgba(59, 130, 246, 0.08)',
        borderRadius: '4px',
        marginBottom: '8px',
        fontSize: '12px',
        color: '#a0a0a0',
      }}
    >
      <div
        style={{
          display: 'flex',
          gap: '3px',
        }}
      >
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            style={{
              width: '6px',
              height: '6px',
              borderRadius: '50%',
              backgroundColor: '#3b82f6',
              animation: `bounce 1.4s infinite`,
              animationDelay: `${i * 0.2}s`,
            }}
          />
        ))}
      </div>
      <span style={{ fontWeight: '500' }}>
        THINKING: <span style={{ fontStyle: 'italic', opacity: 0.8 }}>Processing your request</span>
        <AnimatedDots />
      </span>

      <style>{`
        @keyframes bounce {
          0%, 80%, 100% {
            opacity: 0.5;
            transform: translateY(0);
          }
          40% {
            opacity: 1;
            transform: translateY(-8px);
          }
        }
      `}</style>
    </div>
  );
};
