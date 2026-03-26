import React, { useEffect, useMemo, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'

interface StreamingDisplayProps {
  content: string
  isStreaming: boolean
}

type ParsedPart = {
  type: 'thought' | 'tool'
  content: string
}

export const StreamingDisplay: React.FC<StreamingDisplayProps> = ({ content, isStreaming }) => {
  const [displayContent, setDisplayContent] = useState('')

  useEffect(() => {
    setDisplayContent(content)
  }, [content])

  const normalizeStreamingText = (value: string) => {
    const collapsedWhitespace = value.replace(/\s+/g, ' ').trim()
    return collapsedWhitespace
      .replace(/\b([A-Za-z][A-Za-z'-]{1,})\1\b/g, '$1')
      .replace(/\b([A-Za-z][A-Za-z'-]{1,})(?:\s+\1\b)+/g, '$1')
  }

  const sanitizeThought = (value: string) =>
    value
      .replace(/\\"/g, '"')
      .replace(/\\\\/g, '\\')
      .replace(/",?\s*"tool"\s*:\s*[^]*$/, '')
      .replace(/\}\s*$/, '')
      .replace(/^[{\s"]+/, '')
      .replace(/^thought\b[:\s-]*/i, '')
      .trim()

  const normalizeThoughtMarkdown = (value: string) =>
    sanitizeThought(value)
      .replace(/\r\n/g, '\n')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .replace(/(^|\n)\s*\*{3,}\s*(?=\n|$)/g, '\n')
      .replace(/(^|\n)\s*#{4,}\s+/g, '\n### ')
      .replace(/(^|\n)\s*[-*]\s+/g, '\n- ')
      .trim()

  const parsed = useMemo(() => {
    const parts: ParsedPart[] = []

    try {
      const jsonStr = normalizeStreamingText(displayContent)

      if (jsonStr.startsWith('{')) {
        const completeThought = jsonStr.match(/"thought"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)"/)
        const partialThought = jsonStr.match(/"thought"\s*:\s*"([^]*)$/)

        const thoughtValue = completeThought?.[1] || partialThought?.[1]
        const cleanedThought = thoughtValue ? normalizeThoughtMarkdown(thoughtValue) : ''
        if (cleanedThought) {
          parts.push({
            type: 'thought',
            content: cleanedThought,
          })
        }

        let braceCount = 0
        let lastCompleteIdx = -1
        for (let i = 0; i < jsonStr.length; i++) {
          if (jsonStr[i] === '{') braceCount++
          else if (jsonStr[i] === '}') {
            braceCount--
            if (braceCount === 0) lastCompleteIdx = i
          }
        }

        if (lastCompleteIdx > 0) {
          const completeJson = jsonStr.substring(0, lastCompleteIdx + 1)
          try {
            const obj = JSON.parse(completeJson)
            if (obj.tool && typeof obj.tool === 'string') {
              const toolObj = { tool: obj.tool, args: obj.args || {} }
              parts.push({ type: 'tool', content: JSON.stringify(toolObj, null, 2) })
            }
          } catch {
            // Skip malformed tool snippets while streaming.
          }
        }
      } else if (isStreaming && jsonStr) {
        const fallbackThought = normalizeThoughtMarkdown(jsonStr)
        if (fallbackThought) {
          parts.push({ type: 'thought', content: fallbackThought })
        }
      }
    } catch {
      // Ignore rendering failures and keep the stream visible.
    }

    return parts
  }, [displayContent, isStreaming])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      {parsed.map((part, idx) => (
        <div
          key={idx}
          className="glass"
          style={{
            border: '1px solid rgba(59, 130, 246, 0.14)',
            borderRadius: '12px',
            padding: '12px 14px',
            background: 'linear-gradient(180deg, rgba(15, 23, 42, 0.72), rgba(15, 23, 42, 0.5))',
            boxShadow: '0 10px 30px rgba(0, 0, 0, 0.18)',
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
              color: part.type === 'thought' ? 'var(--accent-primary)' : 'var(--accent-success)',
              marginBottom: '10px',
            }}
          >
            <span
              style={{
                width: '8px',
                height: '8px',
                borderRadius: '999px',
                background: part.type === 'thought' ? 'var(--accent-primary)' : 'var(--accent-success)',
                boxShadow: `0 0 12px ${part.type === 'thought' ? 'var(--accent-primary)' : 'var(--accent-success)'}`,
              }}
            />
            {part.type === 'thought' ? 'Thinking' : 'Tool Call'}
          </div>

          {part.type === 'thought' ? (
            <div style={{ fontSize: '12.5px', color: 'var(--text-primary)', lineHeight: '1.65' }}>
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  p({ children }) {
                    return <p style={{ margin: '0 0 10px 0' }}>{children}</p>
                  },
                  h1({ children }) {
                    return <h1 style={{ fontSize: '14px', margin: '10px 0 8px', color: 'var(--text-primary)' }}>{children}</h1>
                  },
                  h2({ children }) {
                    return <h2 style={{ fontSize: '13px', margin: '10px 0 6px', color: 'var(--text-primary)' }}>{children}</h2>
                  },
                  h3({ children }) {
                    return <h3 style={{ fontSize: '12px', margin: '8px 0 6px', color: 'var(--text-primary)' }}>{children}</h3>
                  },
                  ul({ children }) {
                    return <ul style={{ margin: '8px 0 10px 18px', padding: 0 }}>{children}</ul>
                  },
                  ol({ children }) {
                    return <ol style={{ margin: '8px 0 10px 18px', padding: 0 }}>{children}</ol>
                  },
                  li({ children }) {
                    return <li style={{ margin: '4px 0' }}>{children}</li>
                  },
                  strong({ children }) {
                    return <strong style={{ color: 'var(--text-primary)' }}>{children}</strong>
                  },
                  code({ children }) {
                    return (
                      <code
                        style={{
                          padding: '1px 5px',
                          borderRadius: '4px',
                          background: 'rgba(148, 163, 184, 0.12)',
                          fontSize: '0.95em',
                        }}
                      >
                        {children}
                      </code>
                    )
                  },
                }}
              >
                {part.content}
              </ReactMarkdown>
              {isStreaming && <span style={{ animation: 'blink 1s infinite', marginLeft: '2px' }}>▌</span>}
            </div>
          ) : (
            <SyntaxHighlighter
              language="json"
              style={vscDarkPlus}
              customStyle={{
                background: 'rgba(0, 0, 0, 0.28)',
                padding: '12px',
                borderRadius: '10px',
                fontSize: '11px',
                margin: 0,
                border: '1px solid rgba(255,255,255,0.06)',
              }}
            >
              {part.content}
            </SyntaxHighlighter>
          )}
        </div>
      ))}
    </div>
  )
}
