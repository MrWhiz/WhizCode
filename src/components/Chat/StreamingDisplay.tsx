import React, { useMemo, useState, useEffect } from 'react'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'

interface StreamingDisplayProps {
  content: string
  isStreaming: boolean
}

export const StreamingDisplay: React.FC<StreamingDisplayProps> = ({ content, isStreaming }) => {
  const [displayContent, setDisplayContent] = useState('')

  // Update display content as it streams
  useEffect(() => {
    setDisplayContent(content)
  }, [content])

  const parsed = useMemo(() => {
    const parts: Array<{ type: 'thought' | 'tool', content: string }> = []
    
    try {
      // Try to parse as JSON
      // Handle incomplete JSON while streaming by finding the last complete object
      let jsonStr = displayContent.trim()
      
      // If it looks like JSON, try to parse it
      if (jsonStr.startsWith('{')) {
        // Find the last complete JSON object
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
            
            // Extract thought if present
            if (obj.thought && typeof obj.thought === 'string') {
              parts.push({ type: 'thought', content: obj.thought })
            }
            
            // Extract tool call if present
            if (obj.tool && typeof obj.tool === 'string') {
              const toolObj = { tool: obj.tool, args: obj.args || {} }
              parts.push({ type: 'tool', content: JSON.stringify(toolObj, null, 2) })
            }
          } catch (e) {
            // JSON parsing failed, skip
          }
        }
      }
    } catch (e) {
      // Silently fail
    }
    
    return parts
  }, [displayContent])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      {parsed.map((part, idx) => (
        <div key={idx} style={{
          borderLeft: '2px solid',
          borderColor: part.type === 'thought' ? 'var(--accent-primary)' : 'var(--accent-success)',
          paddingLeft: '12px',
          paddingTop: '8px',
          paddingBottom: '8px'
        }}>
          <div style={{
            fontSize: '11px',
            fontWeight: 600,
            color: part.type === 'thought' ? 'var(--accent-primary)' : 'var(--accent-success)',
            marginBottom: '6px',
            textTransform: 'uppercase'
          }}>
            {part.type === 'thought' ? '💭 Thinking' : '🔧 Tool Call'}
          </div>
          
          {part.type === 'thought' ? (
            <div style={{
              fontSize: '12px',
              color: 'var(--text-primary)',
              fontStyle: 'italic',
              lineHeight: '1.5'
            }}>
              {part.content}
              {isStreaming && <span style={{ animation: 'blink 1s infinite' }}>▌</span>}
            </div>
          ) : (
            <SyntaxHighlighter
              language="json"
              style={vscDarkPlus}
              customStyle={{
                background: 'rgba(0, 0, 0, 0.3)',
                padding: '8px',
                borderRadius: '4px',
                fontSize: '11px',
                margin: 0
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
