import React, { useState, useRef, useEffect } from 'react'

// Components
import { TitleBar } from './components/TitleBar'
import { ActivityBar } from './components/ActivityBar'
import { FileTree } from './components/Explorer/FileTree'
import { SearchPanel } from './components/Explorer/SearchPanel'
import { SourceControlPanel } from './components/Explorer/SourceControlPanel'
import { EditorArea } from './components/Editor/EditorArea'
import { ChatPanel } from './components/Chat/ChatPanel'
import { TerminalPane } from './components/Terminal/TerminalPane'

// Types
import type { Message, AgentStep, OpenFileProps, AIProvider } from './types'

import './App.css'

function App() {
  const [input, setInput] = useState('')
  const [messages, setMessages] = useState<Message[]>([
    { role: 'assistant', content: 'Hello! I\'m your WhizCode agent. Open a folder to get started.' }
  ])
  const [isLoading, setIsLoading] = useState(false)
  const [agentSteps, setAgentSteps] = useState<AgentStep[]>([])

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const [activeMenu, setActiveMenu] = useState<string | null>(null)
  const [workspacePath, setWorkspacePath] = useState<string | null>(null)
  const [activeFileId, setActiveFileId] = useState<string | null>(null)
  const [openFiles, setOpenFiles] = useState<OpenFileProps[]>([])
  const [activeView, setActiveView] = useState<'explorer' | 'search' | 'source-control' | null>('explorer')
  const [sidebarWidth, setSidebarWidth] = useState(260)
  const [isTerminalOpen, setIsTerminalOpen] = useState(false)
  const [terminalHeight, setTerminalHeight] = useState(250)
  const [terminalKey, setTerminalKey] = useState(0)
  const [isChatOpen, setIsChatOpen] = useState(true)
  const [chatWidth, setChatWidth] = useState(400)

  // Model settings
  const [primaryModelProvider, setPrimaryModelProvider] = useState<AIProvider>(() => (localStorage.getItem('primaryModelProvider') as AIProvider) || 'ollama')
  const [primaryModel, setPrimaryModel] = useState(() => localStorage.getItem('primaryModel') || 'llama3')
  const [toolModelProvider, setToolModelProvider] = useState<AIProvider>(() => (localStorage.getItem('toolModelProvider') as AIProvider) || 'ollama')
  const [toolModel, setToolModel] = useState(() => localStorage.getItem('toolModel') || 'llama3')
  const [openaiKey, setOpenaiKey] = useState(() => localStorage.getItem('openaiKey') || '')
  const [geminiKey, setGeminiKey] = useState(() => localStorage.getItem('geminiKey') || '')
  const [ollamaModels, setOllamaModels] = useState<string[]>([])
  const [ollamaError, setOllamaError] = useState<string | null>(null)
  const [ollamaChecking, setOllamaChecking] = useState(false)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)

  // Autopilot mode
  const [isAutopilotMode, setIsAutopilotMode] = useState(() => 
    localStorage.getItem('isAutopilotMode') === 'true'
  )

  // Save settings
  useEffect(() => {
    localStorage.setItem('primaryModelProvider', primaryModelProvider)
    localStorage.setItem('primaryModel', primaryModel)
    localStorage.setItem('toolModelProvider', toolModelProvider)
    localStorage.setItem('toolModel', toolModel)
    localStorage.setItem('openaiKey', openaiKey)
    localStorage.setItem('geminiKey', geminiKey)
    localStorage.setItem('isAutopilotMode', String(isAutopilotMode))
  }, [primaryModelProvider, primaryModel, toolModelProvider, toolModel, openaiKey, geminiKey, isAutopilotMode])

  // Ollama models
  useEffect(() => {
    if (isSettingsOpen && (primaryModelProvider === 'ollama' || toolModelProvider === 'ollama')) {
      refreshOllamaModels()
    }
  }, [isSettingsOpen, primaryModelProvider, toolModelProvider])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+` to toggle terminal
      if (e.ctrlKey && e.key === '`') {
        e.preventDefault()
        setIsTerminalOpen(prev => !prev)
      }
      // Ctrl+S to save
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault()
        handleFileSave()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [activeFileId, openFiles])

  // Restore last workspace on startup
  useEffect(() => {
    const ipc = (window as any).ipcRenderer
    if (!ipc) return

    const workspaceRestoreHandler = (_event: any, workspacePath: string) => {
      setWorkspacePath(workspacePath)
    }

    ipc.on('workspace:restored', workspaceRestoreHandler)
    return () => {
      ipc.off('workspace:restored', workspaceRestoreHandler)
    }
  }, [])

  const refreshOllamaModels = async () => {
    const ipc = (window as any).ipcRenderer
    if (!ipc) return
    setOllamaChecking(true)
    setOllamaError(null)
    try {
      const res = await ipc.invoke('ollama:getModels')
      if (res.error) {
        setOllamaError("Ollama is not running.")
        setOllamaModels([])
      } else {
        setOllamaModels(res)
        setOllamaError(null)
        if (res.length > 0) {
          if (!res.includes(primaryModel)) setPrimaryModel(res[0])
          if (!res.includes(toolModel)) setToolModel(res[0])
        }
      }
    } catch {
      setOllamaError("Could not connect to Ollama.")
      setOllamaModels([])
    } finally {
      setOllamaChecking(false)
    }
  }

  // Resize handlers
  const handleSidebarResize = (e: React.MouseEvent) => {
    e.preventDefault()
    const startX = e.clientX
    const startWidth = sidebarWidth
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = startWidth + (moveEvent.clientX - startX)
      setSidebarWidth(Math.max(160, Math.min(newWidth, 600)))
    }
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  const handleTerminalResize = (e: React.MouseEvent) => {
    e.preventDefault()
    const startY = e.clientY
    const startHeight = terminalHeight
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(100, startHeight - (moveEvent.clientY - startY))
      setTerminalHeight(Math.min(newHeight, window.innerHeight - 100))
    }
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  const handleChatResize = (e: React.MouseEvent) => {
    e.preventDefault()
    const startX = e.clientX
    const startWidth = chatWidth
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(280, startWidth - (moveEvent.clientX - startX))
      setChatWidth(Math.min(newWidth, window.innerWidth - 400))
    }
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  // File operations
  const handleFileOpen = async (path: string, name: string) => {
    const existingFile = openFiles.find(f => f.path === path)
    if (existingFile) {
      setActiveFileId(path)
      return
    }
    const ipc = (window as any).ipcRenderer
    if (ipc) {
      const content = await ipc.invoke('fs:readFile', path)
      if (content !== null) {
        setOpenFiles(prev => [...prev, { path, name, content }])
        setActiveFileId(path)
      }
    }
  }

  const handleFileSave = async () => {
    const activeFile = openFiles.find(f => f.path === activeFileId)
    if (!activeFile) return
    const ipc = (window as any).ipcRenderer
    if (ipc) {
      await ipc.invoke('fs:writeFile', activeFile.path, activeFile.content)
    }
  }

  const handleFileClose = (path: string, e: React.MouseEvent) => {
    e.stopPropagation()
    setOpenFiles(prev => {
      const newFiles = prev.filter(f => f.path !== path)
      if (activeFileId === path) {
        setActiveFileId(newFiles.length > 0 ? newFiles[newFiles.length - 1].path : null)
      }
      return newFiles
    })
  }

  // Handle file system changes from explorer
  const handleFileDeleted = (deletedPath: string) => {
    setOpenFiles(prev => {
      const newFiles = prev.filter(f => !f.path.startsWith(deletedPath))
      if (activeFileId && (activeFileId === deletedPath || activeFileId.startsWith(deletedPath + '/'))) {
        setActiveFileId(newFiles.length > 0 ? newFiles[newFiles.length - 1].path : null)
      }
      return newFiles
    })
  }

  const handleFileRenamed = (oldPath: string, newPath: string) => {
    setOpenFiles(prev => prev.map(f => {
      if (f.path === oldPath) {
        const newName = newPath.split(/[/\\]/).pop() || f.name
        return { ...f, path: newPath, name: newName }
      } else if (f.path.startsWith(oldPath + '/')) {
        // Handle files inside renamed folders
        const relativePath = f.path.substring(oldPath.length)
        return { ...f, path: newPath + relativePath }
      }
      return f
    }))
    
    // Update active file ID if it was renamed
    if (activeFileId === oldPath) {
      setActiveFileId(newPath)
    } else if (activeFileId && activeFileId.startsWith(oldPath + '/')) {
      const relativePath = activeFileId.substring(oldPath.length)
      setActiveFileId(newPath + relativePath)
    }
  }

  // Check for deleted files periodically
  useEffect(() => {
    if (openFiles.length === 0) return

    const checkFiles = async () => {
      const ipc = (window as any).ipcRenderer
      if (!ipc) return

      const filesToCheck = [...openFiles]
      for (const file of filesToCheck) {
        try {
          const exists = await ipc.invoke('fs:checkFileExists', file.path)
          if (!exists) {
            handleFileDeleted(file.path)
          }
        } catch (error) {
          console.error('Error checking file existence:', error)
        }
      }
    }

    const interval = setInterval(checkFiles, 5000) // Check every 5 seconds
    return () => clearInterval(interval)
  }, [openFiles])

  const handleContentChange = (newContent: string | undefined) => {
    if (newContent !== undefined) {
      setOpenFiles(prev => prev.map(f => f.path === activeFileId ? { ...f, content: newContent } : f))
    }
  }

  const getLanguage = (fileName: string) => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    switch (ext) {
      case 'ts': case 'tsx': return 'typescript'
      case 'js': case 'jsx': return 'javascript'
      case 'json': return 'json'
      case 'html': return 'html'
      case 'css': return 'css'
      case 'md': return 'markdown'
      case 'py': return 'python'
      default: return 'plaintext'
    }
  }

  const handleSend = async () => {
    if (!input.trim() || isLoading) return
    const userMsg: Message = { role: 'user', content: input }
    setMessages(prev => [...prev, userMsg])
    setInput('')
    setIsLoading(true)
    setAgentSteps([])

    const ipc = (window as any).ipcRenderer
    const stepHandler = (_event: any, step: AgentStep) => {
      setAgentSteps(prev => {
        // Use requestId for precise matching if available
        if ((step as any).requestId) {
          const existingIdx = prev.findIndex(s => (s as any).requestId === (step as any).requestId)
          if (existingIdx >= 0) {
            const newSteps = [...prev]
            newSteps[existingIdx] = step
            return newSteps
          }
          return [...prev, step]
        }
        
        // Fallback to original logic for steps without requestId
        const existingIdx = prev.findIndex(s => 
          s.tool === step.tool && 
          s.iteration === step.iteration &&
          (s.status === 'running' || s.status === 'awaiting_permission')
        )
        
        if (existingIdx >= 0) {
          const newSteps = [...prev]
          newSteps[existingIdx] = step
          return newSteps
        }
        return [...prev, step]
      })
    }

    // Streaming: accumulate tokens into a live "thinking" message
    let streamingContent = ''
    const STREAMING_MSG_ID = '__streaming__'
    const streamHandler = (_event: any, { token }: { token: string }) => {
      streamingContent += token
      setMessages(prev => {
        const existingIdx = prev.findIndex(m => (m as any).__id === STREAMING_MSG_ID)
        const streamMsg = { role: 'assistant' as const, content: streamingContent, __id: STREAMING_MSG_ID }
        if (existingIdx >= 0) {
          const next = [...prev]
          next[existingIdx] = streamMsg
          return next
        }
        return [...prev, streamMsg]
      })
    }

    if (ipc) {
      ipc.on('agent:step', stepHandler)
      ipc.on('agent:stream', streamHandler)
    }

    try {
      if (ipc) {
        const activeFile = openFiles.find(f => f.path === activeFileId)
        const result = await ipc.invoke('execute-agent-task', {
          task: userMsg.content,
          primaryModel: { provider: primaryModelProvider, model: primaryModel },
          toolModel: { provider: toolModelProvider, model: toolModel },
          workspacePath,
          activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
          config: { openaiKey, geminiKey },
          isAutopilotMode
        })
        const response = typeof result === 'string' ? result : result?.response || 'No response'
        const steps = typeof result === 'object' ? result?.steps || [] : []
        // Replace streaming placeholder with final authoritative response
        setMessages(prev => {
          const withoutStream = prev.filter(m => (m as any).__id !== STREAMING_MSG_ID)
          return [...withoutStream, { role: 'assistant', content: response, steps: steps.length > 0 ? steps : undefined }]
        })
      }
    } catch (err) {
      setMessages(prev => {
        const withoutStream = prev.filter(m => (m as any).__id !== STREAMING_MSG_ID)
        return [...withoutStream, { role: 'assistant', content: 'Error communicating with agent.' }]
      })
    } finally {
      setIsLoading(false)
      setAgentSteps([])
      if (ipc) {
        ipc.off('agent:step', stepHandler)
        ipc.off('agent:stream', streamHandler)
      }
    }
  }

  const handlePermissionResponse = async (approved: boolean, stepIdx?: number) => {
    const ipc = (window as any).ipcRenderer
    if (ipc) {
      let requestId: string | undefined;
      
      if (stepIdx !== undefined && agentSteps[stepIdx]) {
        const step = agentSteps[stepIdx]
        requestId = (step as any).requestId; // Get the requestId from the step
        setMessages(prev => [...prev, {
          role: 'assistant',
          content: approved ? `✅ **Permission granted**: ${step.summary}` : `❌ **Permission denied**: ${step.summary}`
        }])
      }
      
      await ipc.invoke('agent:permission-response', { approved, requestId })
    }
  }

  const handleStop = async () => {
    const ipc = (window as any).ipcRenderer
    if (ipc) await ipc.invoke('agent:stop')
  }

  const handleReset = async () => {
    const ipc = (window as any).ipcRenderer
    if (ipc) await ipc.invoke('agent:reset')
    setMessages([{ role: 'assistant', content: 'Conversation reset. How can I help you now?' }])
    setAgentSteps([])
  }

  const getToolIcon = (tool: string): string => {
    switch (tool) {
      case 'read_file': return '📄'
      case 'write_file': return '✏️'
      case 'edit_file': case 'replace_lines': case 'insert_code': return '🔧'
      case 'list_directory': return '📂'
      case 'search_files': return '🔍'
      case 'run_command': return '⚡'
      case 'apply_diffs': return '🚀'
      case 'validate_project': return '🛡️'
      case 'run_tests': return '🧪'
      case 'indexing_workspace': return '📦'
      case 'continue_iterations': return '🔄'
      default: return '🛠️'
    }
  }

  const menus = [
    { name: 'File', items: [
      { label: 'Open Folder...', action: 'open-folder' },
      { label: 'Save', action: 'save', shortcut: 'Ctrl+S' },
      { separator: true },
      { label: 'Exit', action: 'exit' }
    ]},
    { name: 'View', items: [
      { label: 'Toggle Terminal', action: 'toggle-terminal', shortcut: 'Ctrl+`' },
      { label: 'Toggle Sidebar', action: 'toggle-sidebar', shortcut: 'Ctrl+B' }
    ]},
    { name: 'Terminal', items: [
      { label: 'New Terminal', action: 'new-terminal', shortcut: 'Ctrl+Shift+`' }
    ]},
    { name: 'Help', items: [{ label: 'About', action: 'about' }] }
  ]

  return (
    <div className="app-container">
      <TitleBar
        menus={menus}
        activeMenu={activeMenu}
        toggleMenu={(m) => setActiveMenu(prev => prev === m ? null : m)}
        handleMenuHover={() => {}}
        handleMenuAction={(action) => {
          setActiveMenu(null)
          const ipc = (window as any).ipcRenderer
          if (!ipc) return
          if (action === 'exit') ipc.send('app:exit')
          else if (action === 'new-terminal') setIsTerminalOpen(true)
          else if (action === 'toggle-terminal') setIsTerminalOpen(prev => !prev)
          else if (action === 'toggle-sidebar') setActiveView(prev => prev ? null : 'explorer')
          else if (action === 'open-folder') {
            ipc.invoke('dialog:openFolder').then((result: any) => {
              if (result && !result.canceled && result.filePaths?.length > 0) {
                setWorkspacePath(result.filePaths[0])
              }
            })
          } else if (action === 'save') handleFileSave()
        }}
      />

      <div className="main-content">
        <ActivityBar
          activeView={activeView}
          setActiveView={setActiveView}
          isChatOpen={isChatOpen}
          setIsChatOpen={setIsChatOpen}
        />

        {activeView && (
          <>
            <aside className="sidebar" style={{ width: `${sidebarWidth}px` }}>
              <div className="sidebar-header">
                <span>
                  {activeView === 'explorer' && 'EXPLORER'}
                  {activeView === 'search' && 'SEARCH'}
                  {activeView === 'source-control' && 'SOURCE CONTROL'}
                </span>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="12" cy="12" r="1" />
                  <circle cx="19" cy="12" r="1" />
                  <circle cx="5" cy="12" r="1" />
                </svg>
              </div>

              {activeView === 'explorer' && (
                <>
                  <div className="sidebar-section-header">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <polyline points="9 18 15 12 9 6" />
                    </svg>
                    <strong>{workspacePath ? workspacePath.split(/[/\\]/).pop()?.toUpperCase() : 'WHIZCODE'}</strong>
                  </div>
                  <div className="chat-history">
                    {workspacePath ? (
                      <FileTree 
                        path={workspacePath} 
                        onFileOpen={handleFileOpen}
                        onFileDeleted={handleFileDeleted}
                        onFileRenamed={handleFileRenamed}
                      />
                    ) : (
                      <div className="empty-state">No folder opened.</div>
                    )}
                  </div>
                </>
              )}

              {activeView === 'search' && (
                <SearchPanel workspacePath={workspacePath} onFileOpen={handleFileOpen} />
              )}

              {activeView === 'source-control' && (
                <SourceControlPanel workspacePath={workspacePath} />
              )}
            </aside>
            <div className="sidebar-resize-handle" onMouseDown={handleSidebarResize} />
          </>
        )}

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <EditorArea
            openFiles={openFiles}
            activeFileId={activeFileId}
            setActiveFileId={setActiveFileId}
            workspacePath={workspacePath}
            handleFileClose={handleFileClose}
            getLanguage={getLanguage}
            handleContentChange={handleContentChange}
            handleMenuAction={(action) => {
              const ipc = (window as any).ipcRenderer
              if (!ipc) return
              if (action === 'open-folder') {
                ipc.invoke('dialog:openFolder').then((result: any) => {
                  if (result && !result.canceled && result.filePaths?.length > 0) {
                    setWorkspacePath(result.filePaths[0])
                  }
                })
              } else if (action === 'new-terminal') setIsTerminalOpen(true)
            }}
          />

          {isTerminalOpen && (
            <div className="terminal-panel" style={{ height: `${terminalHeight}px` }}>
              <div className="terminal-resize-handle" onMouseDown={handleTerminalResize} />
              <div className="terminal-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <div style={{ 
                    fontSize: '11px', 
                    fontWeight: 600, 
                    color: 'var(--text-secondary)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.5px',
                    padding: '0 4px'
                  }}>
                    Terminal
                  </div>
                </div>
                <div className="terminal-actions">
                  <button
                    className="terminal-action-btn"
                    onClick={async () => {
                      const ipc = (window as any).ipcRenderer
                      if (ipc) {
                        await ipc.invoke('terminal:reset')
                        setTerminalKey(k => k + 1)
                      }
                    }}
                    title="Kill Terminal (Restart)"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <polyline points="23 4 23 10 17 10" />
                      <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
                    </svg>
                  </button>
                  <button
                    className="terminal-action-btn"
                    onClick={() => setIsTerminalOpen(false)}
                    title="Hide Terminal (Ctrl+`)"
                    style={{ fontSize: '18px', lineHeight: '1' }}
                  >
                    ×
                  </button>
                </div>
              </div>
              <div className="terminal-content">
                <TerminalPane key={terminalKey} />
              </div>
            </div>
          )}
        </div>

        <ChatPanel
          chatWidth={chatWidth}
          handleChatResize={handleChatResize}
          isChatOpen={isChatOpen}
          setIsChatOpen={setIsChatOpen}
          workspacePath={workspacePath}
          messages={messages}
          isLoading={isLoading}
          agentSteps={agentSteps}
          input={input}
          setInput={setInput}
          handleSend={handleSend}
          handleReset={handleReset}
          handleKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend() } }}
          getToolIcon={getToolIcon}
          messagesEndRef={messagesEndRef}
          handlePermissionResponse={handlePermissionResponse}
          handleStop={handleStop}
          settingsProps={{
            isSettingsOpen, setIsSettingsOpen,
            primaryModelProvider, setPrimaryModelProvider, primaryModel, setPrimaryModel,
            toolModelProvider, setToolModelProvider, toolModel, setToolModel,
            ollamaModels, ollamaChecking, ollamaError, refreshOllamaModels, openaiKey, setOpenaiKey, geminiKey, setGeminiKey,
            isAutopilotMode, setIsAutopilotMode
          }}
        />
      </div>

      {/* Status Bar */}
      <div style={{
        height: '22px',
        backgroundColor: 'var(--status-bar)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 12px',
        fontSize: '12px',
        color: 'white',
        borderTop: '1px solid var(--border-color)'
      }}>
        <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
          {workspacePath && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M3 3h7l2 2h9v14H3V3z" />
              </svg>
              <span>{workspacePath.split(/[/\\]/).pop()}</span>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
          {/* Current iteration and tool model */}
          {agentSteps.length > 0 && (
            <div style={{ fontSize: '11px', opacity: 0.9 }}>
              Iteration: {Math.max(...agentSteps.map(s => s.iteration || 0))} • 
              Tool: {toolModelProvider === 'ollama' ? 'Ollama' : toolModelProvider === 'openai' ? 'OpenAI' : 'Gemini'}: {toolModel}
            </div>
          )}
          
          {/* Primary model */}
          <div style={{ fontSize: '11px', opacity: 0.9 }}>
            {primaryModelProvider === 'ollama' ? 'Ollama' : primaryModelProvider === 'openai' ? 'OpenAI' : 'Gemini'}: {primaryModel}
          </div>
        </div>
      </div>
    </div>
  )
}

export default App
