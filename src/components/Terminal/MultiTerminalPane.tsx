import React, { useEffect, useMemo, useRef, useState } from 'react'
import { TerminalPane } from './TerminalPane'
import type { TerminalType } from '../../types'
import { terminal as terminalApi } from '../../lib/tauri-api'

interface TerminalRecord {
  id: string
  type: TerminalType
  name: string
  cwd: string
  createdAt: number
  clearToken: number
  status?: 'starting' | 'ready' | 'error'
  error?: string
}

interface MultiTerminalPaneProps {
  isOpen: boolean
  height: number
  onHeightChange: (height: number) => void
  workspacePath: string | null
  createRequest: number
}

const DEFAULT_SHELL_ICON: Record<TerminalType, string> = {
  bash: '⌁',
  cmd: '⌘',
  powershell: '⚡',
  zsh: '⌁',
  sh: '⌁',
}

export const MultiTerminalPane = ({
  isOpen,
  height,
  onHeightChange,
  workspacePath,
  createRequest,
}: MultiTerminalPaneProps) => {
  const [terminals, setTerminals] = useState<TerminalRecord[]>([])
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null)
  const [availableShells, setAvailableShells] = useState<string[]>([])
  const [showShellMenu, setShowShellMenu] = useState(false)
  const [hoverResizeHandle, setHoverResizeHandle] = useState(false)
  const [menuPosition, setMenuPosition] = useState({ top: 0, right: 0 })
  const resizeHandleRef = useRef<HTMLDivElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const lastCreateRequestRef = useRef(0)
  const defaultShellRef = useRef<TerminalType>('powershell')

  useEffect(() => {
    terminalApi.getDefaultShell()
      .then((defaultShell) => {
        defaultShellRef.current = defaultShell as TerminalType
      })
      .catch(() => {
        defaultShellRef.current = 'bash'
      })

      terminalApi.getAvailableShells()
      .then((shells) => setAvailableShells(shells))
      .catch(() => {
        setAvailableShells(['powershell', 'cmd'])
      })
  }, [])

  const shellOptions = useMemo(() => {
    const shells = availableShells.length > 0 ? availableShells : [defaultShellRef.current]
    return Array.from(new Set(shells)) as TerminalType[]
  }, [availableShells])

  const createTerminal = async (type: TerminalType) => {
    const cwd = workspacePath || undefined
    const pendingId = `pending-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
    const pendingName = `${type} - starting...`

    setTerminals(prev => [...prev, {
      id: pendingId,
      type,
      name: pendingName,
      cwd: cwd || '',
      createdAt: Date.now(),
      clearToken: 0,
      status: 'starting',
    }])
    setActiveTerminalId(pendingId)
    setShowShellMenu(false)

    try {
      const id = await terminalApi.createTerminal(type, cwd)
      setTerminals(prev => {
        const next = prev.filter(terminal => terminal.id !== pendingId)
        const nextName = `${type} - ${next.filter(terminal => terminal.type === type).length + 1}`
        return [
          ...next,
          {
            id,
            type,
            name: nextName,
            cwd: cwd || '',
            createdAt: Date.now(),
            clearToken: 0,
            status: 'ready',
          },
        ]
      })
      setActiveTerminalId(id)
    } catch (error) {
      console.error('Failed to create terminal:', error)
      setTerminals(prev => prev.map(terminal => (
        terminal.id === pendingId
          ? {
              ...terminal,
              status: 'error',
              error: error instanceof Error ? error.message : String(error),
              name: `${type} - failed`,
            }
          : terminal
      )))
      setActiveTerminalId(prev => {
        if (prev !== pendingId) return prev
        return terminals.find(terminal => terminal.id !== pendingId)?.id || null
      })
    }
  }

  const closeTerminal = async (id: string, e?: React.MouseEvent) => {
    e?.stopPropagation()
    try {
      await terminalApi.closeTerminal(id)
    } catch (error) {
      console.error('Failed to close terminal:', error)
    }

    setTerminals(prev => {
      const next = prev.filter(t => t.id !== id)
      if (activeTerminalId === id) {
        setActiveTerminalId(next[0]?.id || null)
      }
      return next
    })
  }

  const clearTerminal = (id: string) => {
    setTerminals(prev => prev.map(terminal => (
      terminal.id === id
        ? { ...terminal, clearToken: terminal.clearToken + 1 }
        : terminal
    )))
  }

  const splitTerminal = async () => {
    const active = terminals.find(terminal => terminal.id === activeTerminalId)
    if (!active) {
      await createTerminal(defaultShellRef.current)
      return
    }
    await createTerminal(active.type)
  }

  useEffect(() => {
    if (isOpen && workspacePath && terminals.length === 0) {
      createTerminal(defaultShellRef.current)
    }
  }, [isOpen, workspacePath, terminals.length])

  useEffect(() => {
    if (createRequest <= lastCreateRequestRef.current) {
      return
    }
    lastCreateRequestRef.current = createRequest
    if (!isOpen || !workspacePath) {
      return
    }
    createTerminal(defaultShellRef.current)
  }, [createRequest, isOpen, workspacePath])

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

  useEffect(() => {
    if (showShellMenu && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect()
      setMenuPosition({
        top: rect.bottom + 6,
        right: Math.max(8, window.innerWidth - rect.right),
      })
    }
  }, [showShellMenu])

  const handleResize = (e: React.MouseEvent) => {
    e.preventDefault()
    const startY = e.clientY
    const startHeight = height

    const onMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(120, startHeight - (moveEvent.clientY - startY))
      onHeightChange(Math.min(newHeight, window.innerHeight - 140))
    }

    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  const activeTerminal = terminals.find(terminal => terminal.id === activeTerminalId) || terminals[0] || null

  const headerShellLabel = activeTerminal
    ? `${DEFAULT_SHELL_ICON[activeTerminal.type] || '⌁'} ${activeTerminal.name}`
    : 'Terminal'

  return (
    <div style={{ display: isOpen ? 'flex' : 'none', flexDirection: 'column', minHeight: 0, minWidth: 0 }}>
      <div
        ref={resizeHandleRef}
        className="terminal-resize-handle"
        onMouseDown={handleResize}
        onMouseEnter={() => setHoverResizeHandle(true)}
        onMouseLeave={() => setHoverResizeHandle(false)}
        title="Drag to resize terminal"
        style={{
          height: '4px',
          background: hoverResizeHandle ? 'var(--accent-primary)' : 'var(--border-color)',
          cursor: 'ns-resize',
          transition: 'background 0.2s',
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
          overflow: 'hidden',
          minHeight: 0,
          minWidth: 0,
        }}
      >
        <div
          className="terminal-tabs"
          style={{
            display: 'flex',
            alignItems: 'center',
            minHeight: '34px',
            background: 'linear-gradient(180deg, rgba(255,255,255,0.02), rgba(0,0,0,0.08))',
            borderBottom: '1px solid var(--border-color)',
            padding: '0 8px',
            gap: '4px',
            overflowX: 'auto',
            overflowY: 'hidden',
            position: 'relative',
          }}
        >
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            minWidth: 0,
            color: 'var(--text-secondary)',
            fontSize: 12,
            fontWeight: 600,
            paddingRight: 8,
          }}>
            <span style={{
              display: 'inline-flex',
              width: 18,
              height: 18,
              borderRadius: 4,
              alignItems: 'center',
              justifyContent: 'center',
              background: 'rgba(0,0,0,0.2)',
              color: 'var(--accent-primary)',
              fontSize: 11,
            }}>$_</span>
            <span style={{ whiteSpace: 'nowrap' }}>{headerShellLabel}</span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 4, overflowX: 'auto', flex: 1 }}>
            {terminals.map(terminal => (
              <div
                key={terminal.id}
                onClick={() => setActiveTerminalId(terminal.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '4px 10px',
                  background: activeTerminalId === terminal.id ? 'rgba(0,0,0,0.28)' : 'transparent',
                  border: activeTerminalId === terminal.id ? '1px solid var(--accent-primary)' : '1px solid transparent',
                  borderRadius: '4px 4px 0 0',
                  cursor: 'pointer',
                  fontSize: '12px',
                  color: 'var(--text-primary)',
                  whiteSpace: 'nowrap',
                  transition: 'all 0.2s',
                  flexShrink: 0,
                }}
              >
                <span style={{ fontSize: '10px' }}>{DEFAULT_SHELL_ICON[terminal.type] || '⌁'}</span>
                <span>{terminal.name}</span>
                <button
                  onClick={(e) => closeTerminal(terminal.id, e)}
                  title="Close Terminal"
                  style={{
                    background: 'none',
                    border: 'none',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    fontSize: '14px',
                    lineHeight: 1,
                    padding: '0 2px',
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginLeft: 'auto', position: 'relative' }}>
            <button
              onClick={() => clearTerminal(activeTerminalId || '')}
              disabled={!activeTerminalId}
              className="terminal-action-btn"
              title="Clear Terminal"
            >
              Clear
            </button>
            <button
              onClick={() => activeTerminalId && closeTerminal(activeTerminalId)}
              disabled={!activeTerminalId}
              className="terminal-action-btn"
              title="Kill Terminal"
            >
              Kill
            </button>
            <button
              onClick={() => splitTerminal()}
              className="terminal-action-btn"
              title="New Terminal"
              style={{ fontWeight: 600 }}
            >
              +
            </button>

            <button
              ref={buttonRef}
              onClick={() => setShowShellMenu(!showShellMenu)}
              style={{
                background: 'transparent',
                border: '1px solid var(--border-color)',
                color: 'var(--text-secondary)',
                cursor: 'pointer',
                padding: '4px 10px',
                borderRadius: '4px',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                transition: 'all 0.2s',
              }}
              title="Choose Shell"
            >
              <span>{activeTerminal ? DEFAULT_SHELL_ICON[activeTerminal.type] || '⌁' : '⌁'}</span>
              <span style={{ fontSize: '10px' }}>▼</span>
            </button>

            {showShellMenu && (
              <div
                ref={menuRef}
                style={{
                  position: 'fixed',
                  background: 'var(--vscode-bg)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '6px',
                  zIndex: 10000,
                  minWidth: '180px',
                  boxShadow: '0 12px 30px rgba(0,0,0,0.45)',
                  top: `${menuPosition.top}px`,
                  right: `${menuPosition.right}px`,
                  overflow: 'hidden',
                }}
              >
                {shellOptions.map(type => (
                  <button
                    key={type}
                    type="button"
                    onMouseDown={(e) => {
                      e.preventDefault()
                      e.stopPropagation()
                      createTerminal(type)
                    }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '10px',
                      width: '100%',
                      padding: '10px 12px',
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-primary)',
                      cursor: 'pointer',
                      textAlign: 'left',
                      fontSize: '12px',
                    }}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.04)'
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'transparent'
                    }}
                  >
                    <span style={{ color: 'var(--accent-primary)', fontWeight: 700 }}>{DEFAULT_SHELL_ICON[type] || '⌁'}</span>
                    <span style={{ textTransform: 'capitalize' }}>{type}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div style={{ flex: 1, overflow: 'hidden', background: '#1e1e1e', minHeight: 0, minWidth: 0 }}>
          {activeTerminal ? (
            <TerminalPane
              key={activeTerminal.id}
              terminalId={activeTerminal.id}
              clearToken={activeTerminal.clearToken}
              isVisible={isOpen}
            />
          ) : (
            <div style={{
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--text-secondary)',
              fontSize: 13,
            }}>
              No terminal sessions yet
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
