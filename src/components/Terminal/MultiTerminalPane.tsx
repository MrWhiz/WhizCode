import React, { useState, useRef, useEffect } from 'react'
import { TerminalPane } from './TerminalPane'
import type { TerminalType } from '../../types'

interface Terminal {
  id: string
  type: TerminalType
  name: string
  createdAt: number
}

interface MultiTerminalPaneProps {
  isOpen: boolean
  height: number
  onHeightChange: (height: number) => void
}

export const MultiTerminalPane = ({ isOpen, height, onHeightChange }: MultiTerminalPaneProps) => {
  const [terminals, setTerminals] = useState<Terminal[]>([])
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null)
  const [showShellMenu, setShowShellMenu] = useState(false)
  const [hoverResizeHandle, setHoverResizeHandle] = useState(false)
  const [menuPosition, setMenuPosition] = useState({ top: 0, right: 0 })
  const resizeHandleRef = useRef<HTMLDivElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)

  const ipc = (window as any).ipcRenderer

  // Initialize with default terminal
  useEffect(() => {
    if (isOpen && terminals.length === 0) {
      if (ipc) {
        ipc.invoke('terminal:getDefaultShell').then((defaultShell: TerminalType) => {
          createNewTerminal(defaultShell);
        }).catch(() => {
          createNewTerminal('bash');
        });
      } else {
        createNewTerminal('bash');
      }
    }
  }, [isOpen])

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowShellMenu(false)
      }
    }

    if (showShellMenu) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showShellMenu])

  // Calculate menu position when it opens
  useEffect(() => {
    if (showShellMenu && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect()
      setMenuPosition({
        top: rect.bottom + 4,
        right: window.innerWidth - rect.right
      })
    }
  }, [showShellMenu])

  const createNewTerminal = (type: TerminalType) => {
    const id = `terminal_${Date.now()}`
    const newTerminal: Terminal = {
      id,
      type,
      name: `${type} - ${terminals.length + 1}`,
      createdAt: Date.now()
    }

    setTerminals(prev => [...prev, newTerminal])
    setActiveTerminalId(id)
    setShowShellMenu(false)

    // Notify backend
    if (ipc) {
      ipc.send('terminal:create', { id, type })
    }
  }

  const closeTerminal = (id: string, e: React.MouseEvent) => {
    e.stopPropagation()

    if (ipc) {
      ipc.send('terminal:close', id)
    }

    setTerminals(prev => prev.filter(t => t.id !== id))

    if (activeTerminalId === id) {
      const remaining = terminals.filter(t => t.id !== id)
      setActiveTerminalId(remaining.length > 0 ? remaining[0].id : null)
    }
  }

  const handleResize = (e: React.MouseEvent) => {
    e.preventDefault()
    const startY = e.clientY
    const startHeight = height

    const onMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(100, startHeight - (moveEvent.clientY - startY))
      onHeightChange(Math.min(newHeight, window.innerHeight - 100))
    }

    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  if (!isOpen) return null

  return (
    <>
      <div
        ref={resizeHandleRef}
        className="terminal-resize-handle"
        onMouseDown={handleResize}
        onMouseEnter={() => setHoverResizeHandle(true)}
        onMouseLeave={() => setHoverResizeHandle(false)}
        style={{
          height: '4px',
          background: hoverResizeHandle ? 'var(--accent-primary)' : 'var(--border-color)',
          cursor: 'ns-resize',
          transition: 'background 0.2s'
        }}
      />

      <div
        className="terminal-pane"
        style={{
          height: `${height}px`,
          display: 'flex',
          flexDirection: 'column',
          background: 'var(--vscode-bg)',
          borderTop: '1px solid var(--border-color)',
          overflow: 'hidden'
        }}
      >
        {/* Terminal Tabs */}
        <div
          className="terminal-tabs"
          style={{
            display: 'flex',
            alignItems: 'center',
            height: '32px',
            background: 'var(--vscode-bg)',
            borderBottom: '1px solid var(--border-color)',
            padding: '0 8px',
            gap: '4px',
            overflowX: 'auto',
            overflowY: 'hidden',
            position: 'relative'
          }}
        >
          {terminals.map(terminal => (
            <div
              key={terminal.id}
              onClick={() => setActiveTerminalId(terminal.id)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '4px 12px',
                background: activeTerminalId === terminal.id ? 'rgba(0,0,0,0.3)' : 'transparent',
                border: activeTerminalId === terminal.id ? '1px solid var(--accent-primary)' : '1px solid transparent',
                borderRadius: '4px 4px 0 0',
                cursor: 'pointer',
                fontSize: '12px',
                color: 'var(--text-primary)',
                whiteSpace: 'nowrap',
                transition: 'all 0.2s',
                flexShrink: 0
              }}
            >
              <span style={{ fontSize: '10px' }}>
                {terminal.type === 'bash' && '🐚'}
                {terminal.type === 'cmd' && '⌘'}
                {terminal.type === 'powershell' && '⚡'}
                {terminal.type === 'zsh' && '🐚'}
                {terminal.type === 'sh' && '🐚'}
              </span>
              <span>{terminal.name}</span>
              <button
                onClick={(e) => closeTerminal(terminal.id, e)}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.color = 'var(--text-secondary)'
                }}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'var(--text-secondary)',
                  cursor: 'pointer',
                  fontSize: '12px',
                  padding: '0 4px',
                  display: 'flex',
                  alignItems: 'center'
                }}
              >
                ×
              </button>
            </div>
          ))}

          {/* Add Terminal Button */}
          <div style={{ position: 'relative', marginLeft: 'auto', flexShrink: 0 }}>
            <button
              ref={buttonRef}
              onClick={() => setShowShellMenu(!showShellMenu)}
              style={{
                background: 'transparent',
                border: '1px solid var(--border-color)',
                color: 'var(--text-secondary)',
                cursor: 'pointer',
                padding: '4px 8px',
                borderRadius: '4px',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                transition: 'all 0.2s'
              }}
              title="New Terminal"
            >
              <span>+</span>
              <span style={{ fontSize: '10px' }}>▼</span>
            </button>

            {/* Shell Type Menu */}
            {showShellMenu && (
              <div
                ref={menuRef}
                style={{
                  position: 'fixed',
                  background: 'var(--vscode-bg)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  zIndex: 10000,
                  minWidth: '150px',
                  boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
                  top: `${menuPosition.top}px`,
                  right: `${menuPosition.right}px`
                }}
              >
                {(['bash', 'cmd', 'powershell', 'zsh', 'sh'] as TerminalType[]).map(type => (
                  <button
                    key={type}
                    onClick={() => createNewTerminal(type)}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'rgba(0,0,0,0.2)'
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'transparent'
                    }}
                    style={{
                      display: 'block',
                      width: '100%',
                      padding: '8px 12px',
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-primary)',
                      cursor: 'pointer',
                      textAlign: 'left',
                      fontSize: '12px',
                      transition: 'background 0.2s'
                    }}
                  >
                    {type === 'bash' && '🐚 Bash'}
                    {type === 'cmd' && '⌘ Command Prompt'}
                    {type === 'powershell' && '⚡ PowerShell'}
                    {type === 'zsh' && '🐚 Zsh'}
                    {type === 'sh' && '🐚 Shell'}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Terminal Content */}
        <div
          style={{
            flex: 1,
            overflow: 'hidden',
            background: '#1e1e1e'
          }}
        >
          {activeTerminalId && (
            <TerminalPane key={activeTerminalId} terminalId={activeTerminalId} />
          )}
        </div>
      </div>
    </>
  )
}
