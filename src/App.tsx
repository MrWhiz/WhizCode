import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react'

// Components
import { TitleBar } from './components/TitleBar'
import { ActivityBar } from './components/ActivityBar'
import { FileTree } from './components/Explorer/FileTree'
import { SearchPanel } from './components/Explorer/SearchPanel'
import { SourceControlPanel } from './components/Explorer/SourceControlPanel'
import { EditorArea } from './components/Editor/EditorArea'
import { TerminalPane } from './components/Terminal/TerminalPane'
import { MultiTerminalPane } from './components/Terminal/MultiTerminalPane'
import { ChatPanel } from './components/Chat/ChatPanel'
import { AIInsightsPanel } from './components/Explorer/AIInsightsPanel'

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

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, agentSteps])
  const [workspacePath, setWorkspacePath] = useState<string | null>(null)
  const [activeFileId, setActiveFileId] = useState<string | null>(null)
  const [openFiles, setOpenFiles] = useState<OpenFileProps[]>([])
  const [activeView, setActiveView] = useState<'explorer' | 'search' | 'source-control' | 'ai-insights' | null>('explorer')
  const [sidebarWidth, setSidebarWidth] = useState(260)
  const [isTerminalOpen, setIsTerminalOpen] = useState(false)
  const [terminalHeight, setTerminalHeight] = useState(250)
  const [terminalKey, setTerminalKey] = useState(0)
  const [isChatOpen, setIsChatOpen] = useState(true)
  const [chatWidth, setChatWidth] = useState(400)

  // Explorer state
  const [refreshKey, setRefreshKey] = useState(0)
  const [collapseAll, setCollapseAll] = useState(false)
  const [showFileFilter, setShowFileFilter] = useState(false)
  const [fileFilter, setFileFilter] = useState('')
  const [newFileDialog, setNewFileDialog] = useState<{ parentPath: string } | null>(null)
  const [newFolderDialog, setNewFolderDialog] = useState<{ parentPath: string } | null>(null)
  const [newItemName, setNewItemName] = useState('')
  const [fileErrors, setFileErrors] = useState<Record<string, number>>({})

  // Model settings
  const [modelProvider, setModelProvider] = useState<AIProvider>(() => (localStorage.getItem('modelProvider') as AIProvider) || 'ollama')
  const [model, setModel] = useState(() => {
    const savedModel = localStorage.getItem('model');
    if (savedModel) return savedModel;
    
    // Set better defaults based on provider
    const savedProvider = localStorage.getItem('modelProvider') as AIProvider;
    switch (savedProvider) {
      case 'openai': return 'gpt-4o';
      case 'gemini': return 'gemini-1.5-pro';
      case 'bedrock': return 'anthropic.claude-3-5-sonnet-20241022-v2:0';
      default: return 'qwen2.5-coder:latest';
    }
  })
  const [openaiKey, setOpenaiKey] = useState(() => localStorage.getItem('openaiKey') || '')
  const [geminiKey, setGeminiKey] = useState(() => localStorage.getItem('geminiKey') || '')
  const [bedrockRegion, setBedrockRegion] = useState(() => localStorage.getItem('bedrockRegion') || 'us-east-1')
  const [bedrockAccessKey, setBedrockAccessKey] = useState(() => localStorage.getItem('bedrockAccessKey') || '')
  const [bedrockSecretKey, setBedrockSecretKey] = useState(() => localStorage.getItem('bedrockSecretKey') || '')
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
    localStorage.setItem('modelProvider', modelProvider)
    localStorage.setItem('model', model)
    localStorage.setItem('openaiKey', openaiKey)
    localStorage.setItem('geminiKey', geminiKey)
    localStorage.setItem('bedrockRegion', bedrockRegion)
    localStorage.setItem('bedrockAccessKey', bedrockAccessKey)
    localStorage.setItem('bedrockSecretKey', bedrockSecretKey)
    localStorage.setItem('isAutopilotMode', String(isAutopilotMode))
  }, [modelProvider, model, openaiKey, geminiKey, bedrockRegion, bedrockAccessKey, bedrockSecretKey, isAutopilotMode])

  // Test Ollama connection on startup
  useEffect(() => {
    if (modelProvider === 'ollama') {
      refreshOllamaModels()
    }
  }, [modelProvider])

  // Refresh Ollama models when settings are opened
  useEffect(() => {
    if (isSettingsOpen && modelProvider === 'ollama') {
      refreshOllamaModels()
    }
  }, [isSettingsOpen])

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
      // Ctrl+N to create new file (when explorer is focused)
      if (e.ctrlKey && e.key === 'n' && activeView === 'explorer' && workspacePath) {
        e.preventDefault()
        setNewFileDialog({ parentPath: workspacePath })
      }
      // Ctrl+Shift+N to create new folder (when explorer is focused)
      if (e.ctrlKey && e.shiftKey && e.key === 'N' && activeView === 'explorer' && workspacePath) {
        e.preventDefault()
        setNewFolderDialog({ parentPath: workspacePath })
      }
      // F5 to refresh explorer
      if (e.key === 'F5' && activeView === 'explorer') {
        e.preventDefault()
        setRefreshKey(prev => prev + 1)
      }
      // Ctrl+P for file filter
      if (e.ctrlKey && e.key === 'p' && activeView === 'explorer') {
        e.preventDefault()
        setShowFileFilter(true)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [activeFileId, openFiles, activeView, workspacePath])

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

  // Listen for file changes from the backend (when agent updates files)
  useEffect(() => {
    const ipc = (window as any).ipcRenderer
    if (!ipc) return

    const fileChangeHandler = (_event: any, { path, content }: { path: string; content: string }) => {
      // Update the file in openFiles if it's currently open
      setOpenFiles(prev => {
        const fileExists = prev.some(f => f.path === path)
        if (fileExists) {
          return prev.map(f => f.path === path ? { ...f, content } : f)
        }
        return prev
      })
    }

    ipc.on('file:changed', fileChangeHandler)
    return () => {
      ipc.off('file:changed', fileChangeHandler)
    }
  }, [])

  const refreshOllamaModels = async () => {
    const ipc = (window as any).ipcRenderer
    if (!ipc) return
    setOllamaChecking(true)
    setOllamaError(null)
    
    try {
      console.log('[FRONTEND] Checking Ollama health...')
      // First do a health check
      const healthCheck = await ipc.invoke('ollama:healthCheck')
      if (!healthCheck.healthy) {
        setOllamaError(`Ollama health check failed: ${healthCheck.error}`)
        setOllamaModels([])
        return
      }
      
      console.log('[FRONTEND] Ollama is healthy, fetching models...')
      // Then get models
      const res = await ipc.invoke('ollama:getModels')
      if (res.error) {
        setOllamaError(res.error)
        setOllamaModels([])
      } else {
        console.log('[FRONTEND] Received models:', res)
        setOllamaModels(res)
        setOllamaError(null)
        if (res.length > 0 && !res.includes(model)) {
          setModel(res[0])
        }
      }
    } catch (error: any) {
      console.error('[FRONTEND] Ollama connection error:', error)
      setOllamaError("Could not connect to Ollama: " + (error.message || 'Unknown error'))
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

  const normalizePath = (p: string) => p.replace(/\\/g, '/').toLowerCase().replace(/^[a-z]:/, '').replace(/^\/+/, '');

  // Check for errors in a file
  const checkFileErrors = async (filePath: string, content: string): Promise<number> => {
    const ipc = (window as any).ipcRenderer
    if (!ipc || !workspacePath) return 0

    const normFilePath = normalizePath(filePath);
    const normWorkspacePath = normalizePath(workspacePath);

    try {
      // Add timeout to prevent hanging
      const timeoutPromise = new Promise<any[]>((_, reject) => 
        setTimeout(() => reject(new Error('Diagnostics timeout')), 3000)
      );
      
      const diagnostics = await Promise.race([
        ipc.invoke('diagnostics:check', normFilePath, normWorkspacePath, content),
        timeoutPromise
      ]);
      
      const count = Array.isArray(diagnostics) ? diagnostics.length : 0;
      console.log(`[APP] File ${normFilePath.split('/').pop()} has ${count} errors`);
      return count;
    } catch (error) {
      console.error('Error checking file diagnostics:', error)
      return 0
    }
  }

  // Relying on onValidation for all open files

  const handleValidation = useCallback((filePath: string, count: number) => {
    const normPath = normalizePath(filePath);
    setFileErrors(prev => {
      if (prev[normPath] === count) return prev;
      const next = { ...prev };
      if (count > 0) {
        next[normPath] = count;
      } else {
        delete next[normPath];
      }
      return next;
    });
  }, []);

  const handleSend = async (overrideInput?: string) => {
    const textToSend = overrideInput || input;
    if (!textToSend.trim() || isLoading) return
    const userMsg: Message = { role: 'user', content: textToSend }
    setMessages(prev => [...prev, userMsg])
    if (!overrideInput) setInput('')
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
          model: { 
            provider: modelProvider, 
            model: model,
            openaiKey,
            geminiKey,
            bedrockRegion,
            bedrockAccessKey,
            bedrockSecretKey
          },
          workspacePath,
          activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
          config: { openaiKey, geminiKey },
          isAutopilotMode
        })
        const response = typeof result === 'string' ? result : result?.response || 'No response'
        const steps = typeof result === 'object' ? result?.steps || [] : []
        // Clear live steps before adding final message
        setAgentSteps([])
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

  const handleFixError = (filePath: string, line: number, message: string) => {
    const fileName = filePath.split(/[/\\]/).pop() || filePath;
    const fixPrompt = `I have an error in \`${fileName}\` at line ${line}:\n\n\`\`\`\n${message}\n\`\`\`\n\nPlease help me fix it.`;
    handleSend(fixPrompt);
    setIsChatOpen(true);
  }

  const handleCreateNewFile = async () => {
    if (!newFileDialog || !newItemName.trim()) return
    
    const ipc = (window as any).ipcRenderer
    if (!ipc) return

    try {
      const separator = newFileDialog.parentPath.includes('\\') ? '\\' : '/'
      const fullPath = newFileDialog.parentPath + separator + newItemName.trim()
      
      await ipc.invoke('fs:createFile', fullPath)
      // Automatically open newly created files
      handleFileOpen(fullPath, newItemName.trim())
      
      setRefreshKey(prev => prev + 1)
      setNewFileDialog(null)
      setNewItemName('')
    } catch (error) {
      console.error('Create file failed:', error)
      alert('Failed to create file: ' + (error as Error).message)
    }
  }

  const handleCreateNewFolder = async () => {
    if (!newFolderDialog || !newItemName.trim()) return
    
    const ipc = (window as any).ipcRenderer
    if (!ipc) return

    try {
      const separator = newFolderDialog.parentPath.includes('\\') ? '\\' : '/'
      const fullPath = newFolderDialog.parentPath + separator + newItemName.trim()
      
      await ipc.invoke('fs:createDirectory', fullPath)
      
      setRefreshKey(prev => prev + 1)
      setNewFolderDialog(null)
      setNewItemName('')
    } catch (error) {
      console.error('Create folder failed:', error)
      alert('Failed to create folder: ' + (error as Error).message)
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
                  {activeView === 'ai-insights' && 'AI INSIGHTS'}
                </span>
                <div className="sidebar-header-actions">
                  {activeView === 'explorer' && workspacePath && (
                    <>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          // Trigger refresh by incrementing a key
                          setRefreshKey(prev => prev + 1);
                        }}
                        title="Refresh Explorer"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <polyline points="23 4 23 10 17 10"></polyline>
                          <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                        </svg>
                      </button>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          setCollapseAll(prev => !prev);
                        }}
                        title="Collapse All"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <polyline points="6 9 12 15 18 9"></polyline>
                        </svg>
                      </button>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          setShowFileFilter(prev => !prev);
                          if (showFileFilter) setFileFilter('');
                        }}
                        title="Filter Files"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"></polygon>
                        </svg>
                      </button>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          const ipc = (window as any).ipcRenderer;
                          if (ipc) {
                            ipc.invoke('dialog:openFolder').then((result: any) => {
                              if (result && !result.canceled && result.filePaths?.length > 0) {
                                setWorkspacePath(result.filePaths[0]);
                              }
                            });
                          }
                        }}
                        title="Open Folder"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                        </svg>
                      </button>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          setNewFileDialog({ parentPath: workspacePath });
                        }}
                        title="New File"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                          <polyline points="14 2 14 8 20 8"></polyline>
                          <line x1="12" y1="18" x2="12" y2="12"></line>
                          <line x1="9" y1="15" x2="15" y2="15"></line>
                        </svg>
                      </button>
                      <button 
                        className="sidebar-action-btn" 
                        onClick={() => {
                          setNewFolderDialog({ parentPath: workspacePath });
                        }}
                        title="New Folder"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                          <line x1="12" y1="11" x2="12" y2="17"></line>
                          <line x1="9" y1="14" x2="15" y2="14"></line>
                        </svg>
                      </button>
                    </>
                  )}
                  <button className="sidebar-action-btn" title="More Actions">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <circle cx="12" cy="12" r="1" />
                      <circle cx="19" cy="12" r="1" />
                      <circle cx="5" cy="12" r="1" />
                    </svg>
                  </button>
                </div>
              </div>

              {activeView === 'explorer' && (
                <>
                  <div className="sidebar-section-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <polyline points="9 18 15 12 9 6" />
                      </svg>
                      <strong>{workspacePath ? workspacePath.split(/[/\\]/).pop()?.toUpperCase() : 'WHIZCODE'}</strong>
                    </div>
                    {Object.keys(fileErrors).length > 0 && (
                      <span style={{ 
                        backgroundColor: '#ff3333', 
                        color: 'white', 
                        borderRadius: '10px', 
                        padding: '1px 6px', 
                        fontSize: '11px',
                        marginRight: '8px'
                      }}>
                        {Object.values(fileErrors).reduce((a, b) => a + b, 0)}
                      </span>
                    )}
                  </div>
                  {showFileFilter && (
                    <div className="file-filter">
                      <input
                        type="text"
                        placeholder="Filter files..."
                        value={fileFilter}
                        onChange={(e) => setFileFilter(e.target.value)}
                        className="file-filter-input"
                        autoFocus
                      />
                    </div>
                  )}
                  <div className="chat-history">
                    {workspacePath ? (
                      <FileTree 
                        path={workspacePath} 
                        onFileOpen={handleFileOpen}
                        onFileDeleted={handleFileDeleted}
                        onFileRenamed={handleFileRenamed}
                        refreshKey={refreshKey}
                        collapseAll={collapseAll}
                        fileFilter={fileFilter}
                        fileErrors={fileErrors}
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

              {activeView === 'ai-insights' && (
                <AIInsightsPanel />
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
            fileErrors={fileErrors}
            onFixError={handleFixError}
            onValidation={handleValidation}
          />

          <MultiTerminalPane 
            isOpen={isTerminalOpen}
            height={terminalHeight}
            onHeightChange={setTerminalHeight}
          />
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
            modelProvider, setModelProvider, model, setModel,
            ollamaModels, ollamaChecking, ollamaError, refreshOllamaModels, 
            openaiKey, setOpenaiKey, geminiKey, setGeminiKey,
            bedrockRegion, setBedrockRegion, bedrockAccessKey, setBedrockAccessKey, bedrockSecretKey, setBedrockSecretKey,
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
          {/* Current iteration and model */}
          {agentSteps.length > 0 && (
            <div style={{ fontSize: '11px', opacity: 0.9 }}>
              Iteration: {Math.max(...agentSteps.map(s => s.iteration || 0))}
            </div>
          )}
          
          {/* Model info */}
          <div style={{ fontSize: '11px', opacity: 0.9 }}>
            {modelProvider === 'ollama' ? 'Ollama' : 
             modelProvider === 'openai' ? 'OpenAI' : 
             modelProvider === 'gemini' ? 'Gemini' : 'Bedrock'}: {model}
          </div>
        </div>
      </div>

      {/* New File Dialog */}
      {newFileDialog && (
        <div className="modal-overlay">
          <div className="modal-dialog">
            <h3>Create New File</h3>
            <input
              type="text"
              value={newItemName}
              onChange={(e) => setNewItemName(e.target.value)}
              placeholder="Enter file name..."
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleCreateNewFile()
                if (e.key === 'Escape') {
                  setNewFileDialog(null)
                  setNewItemName('')
                }
              }}
            />
            <div className="modal-buttons">
              <button onClick={handleCreateNewFile} disabled={!newItemName.trim()}>
                Create
              </button>
              <button onClick={() => {
                setNewFileDialog(null)
                setNewItemName('')
              }}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* New Folder Dialog */}
      {newFolderDialog && (
        <div className="modal-overlay">
          <div className="modal-dialog">
            <h3>Create New Folder</h3>
            <input
              type="text"
              value={newItemName}
              onChange={(e) => setNewItemName(e.target.value)}
              placeholder="Enter folder name..."
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleCreateNewFolder()
                if (e.key === 'Escape') {
                  setNewFolderDialog(null)
                  setNewItemName('')
                }
              }}
            />
            <div className="modal-buttons">
              <button onClick={handleCreateNewFolder} disabled={!newItemName.trim()}>
                Create
              </button>
              <button onClick={() => {
                setNewFolderDialog(null)
                setNewItemName('')
              }}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
