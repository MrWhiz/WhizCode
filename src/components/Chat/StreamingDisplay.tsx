import React, { useEffect, useMemo, useState } from 'react'

interface StreamingDisplayProps {
  content: string
  isStreaming: boolean
}

export const StreamingDisplay: React.FC<StreamingDisplayProps> = ({ content, isStreaming }) => {
  const [displayContent, setDisplayContent] = useState('')

  useEffect(() => {
    setDisplayContent(content)
  }, [content])

  const previewText = useMemo(() => {
    const normalized = displayContent.replace(/\s+/g, ' ').trim()
    if (!normalized) {
      return ''
    }

    const thoughtMatch =
      normalized.match(/"thought"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)"/) ||
      normalized.match(/"thought"\s*:\s*"([^]*)$/)
    if (thoughtMatch?.[1]) {
      return thoughtMatch[1]
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, '\\')
        .replace(/",?\s*"tool"\s*:\s*[^]*$/, '')
        .trim()
    }

    const toolMatch = normalized.match(/"tool"\s*:\s*"([^"]+)"/)
    if (toolMatch?.[1]) {
      return `Preparing ${toolMatch[1]}`
    }

    return normalized
  }, [displayContent])

  if (!previewText) {
    return null
  }

  return (
    <div
      className="glass"
      style={{
        border: '1px solid rgba(59, 130, 246, 0.14)',
        borderRadius: '10px',
        padding: '8px 10px',
        background: 'linear-gradient(180deg, rgba(15, 23, 42, 0.6), rgba(15, 23, 42, 0.42))',
        boxShadow: '0 8px 18px rgba(0, 0, 0, 0.14)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          fontSize: '10px',
          fontWeight: 800,
          letterSpacing: '0.08em',
          textTransform: 'uppercase',
          color: 'var(--accent-primary)',
          marginBottom: '6px',
        }}
      >
        <span
          style={{
            width: '7px',
            height: '7px',
            borderRadius: '999px',
            background: 'var(--accent-primary)',
            boxShadow: '0 0 10px var(--accent-primary)',
          }}
        />
        Live Stream
      </div>

      <div
        style={{
          fontSize: '12px',
          color: 'var(--text-primary)',
          lineHeight: '1.45',
          display: '-webkit-box',
          WebkitLineClamp: 2,
          WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
          wordBreak: 'break-word',
        }}
      >
        {previewText}
        {isStreaming && <span style={{ animation: 'blink 1s infinite', marginLeft: '2px' }}>▌</span>}
      </div>
    </div>
  )
}
