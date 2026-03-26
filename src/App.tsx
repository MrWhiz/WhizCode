import React, { useCallback } from 'react'

// Components
import { TitleBar } from './components/TitleBar'
import { ActivityBar } from './components/ActivityBar'
import { FileTree } from './components/Explorer/FileTree'
import { SearchPanel } from './components/Explorer/SearchPanel'
import { SourceControlPanel } from './components/Explorer/SourceControlPanel'
import { EditorArea } from './components/Editor/EditorArea'
import { ChatPanel } from './components/Chat/ChatPanel'
import { WebPreview } from './components/Preview/WebPreview'
import { MultiTerminalPane } from './components/Terminal/MultiTerminalPane'
import SystemPerformance from './components/Explorer/SystemPerformance'

// Hooks
import { useAppState } from './hooks/useAppState'
import { useFileOperations } from './hooks/useFileOperations'
import { useAppEventListeners } from './hooks/useAppEventListeners'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useSettingsPersistence } from './hooks/useSettingsPersistence'
import { useWorkspaceInit } from './hooks/useWorkspaceInit'
import { useModelManagement } from './hooks/useModelManagement'
import { useGitStatus } from './hooks/useGitStatus'
import { useFileExistenceCheck } from './hooks/useFileExistenceCheck'
import { useAutoScroll } from './hooks/useAutoScroll'

// Types
import type { AgentStep, Message } from './types'

import { agent, dialog, workspace, fs, history, git, errorRecovery } from './lib/tauri-api'
import { loadAppState } from './lib/appState'

import { WhizLogo } from './components/Branding/WhizLogo'
import { FiPlus, FiFolderPlus, FiRotateCw, FiMinimize2, FiFolder } from 'react-icons/fi'
import './App.css'


function App() {
  const savedState = loadAppState()
  const latestAgentStepsRef = React.useRef<AgentStep[]>([])
  const historyLoadedWorkspaceRef = React.useRef<string | null>(null)
  const isHydratingHistoryRef = React.useRef(false)

  // Use custom hooks for state management
  const appState = useAppState()
  const {
    input, setInput,
    messages, setMessages,
    selectedImages, setSelectedImages,
    isLoading, setIsLoading,
    agentSteps, setAgentSteps,
    agentError, setAgentError,
    liveStreamingContent, setLiveStreamingContent,
    askUserPrompt, setAskUserPrompt,
    workspacePath, setWorkspacePath,
    activeFileId, setActiveFileId,
    openFiles, setOpenFiles,
    activeView, setActiveView,
    sidebarWidth, setSidebarWidth,
    isChatOpen, setIsChatOpen,
    chatWidth, setChatWidth,
    terminalHeight, setTerminalHeight,
    isTerminalOpen, setIsTerminalOpen,
    refreshKey, setRefreshKey,
    collapseAll, setCollapseAll,
    showFileFilter, setShowFileFilter,
    fileFilter, setFileFilter,
    newFileDialog, setNewFileDialog,
    gitStatus, setGitStatus,
    newFolderDialog, setNewFolderDialog,
    newItemName, setNewItemName,
    fileErrors, setFileErrors,
    modelProvider, setModelProvider,
    model, setModel,
    openaiKey, setOpenaiKey,
    geminiKey, setGeminiKey,
    bedrockRegion, setBedrockRegion,
    bedrockAccessKey, setBedrockAccessKey,
    bedrockSecretKey, setBedrockSecretKey,
    ollamaModels, setOllamaModels,
    ollamaError, setOllamaError,
    ollamaChecking, setOllamaChecking,
    isSettingsOpen, setIsSettingsOpen,
    isAboutOpen, setIsAboutOpen,
    azureLoginUrl, setAzureLoginUrl,
    azureCompletionUrl, setAzureCompletionUrl,
    azureUsername, setAzureUsername,
    azurePassword, setAzurePassword,
    azureSessionToken, setAzureSessionToken,
    azureTokenExpiresAt, setAzureTokenExpiresAt,
    azureTokenStatus, setAzureTokenStatus,
    isAutopilotMode, setIsAutopilotMode,
    contextLength, setContextLength,
    messagesEndRef,
    streamingContentRef,
    STREAMING_MSG_ID,
  } = appState

  // Menu state
  const [activeMenu, setActiveMenu] = React.useState<string | null>(null)
  const [terminalCreateRequest, setTerminalCreateRequest] = React.useState(0)

  const menus = [
    {
      name: 'File',
      items: [
        { label: 'Open Folder', action: 'open-folder', shortcut: 'Ctrl+K Ctrl+O' },
        { separator: true },
        { label: 'Exit', action: 'exit' }
      ]
    },
    {
      name: 'Edit',
      items: [
        { label: 'Undo', action: 'undo', shortcut: 'Ctrl+Z' },
        { label: 'Redo', action: 'redo', shortcut: 'Ctrl+Shift+Z' }
      ]
    },
    {
      name: 'View',
      items: [
        { label: 'Explorer', action: 'toggle-explorer', shortcut: 'Ctrl+Shift+E' },
        { label: 'Search', action: 'toggle-search', shortcut: 'Ctrl+Shift+F' },
        { label: 'Source Control', action: 'toggle-source-control', shortcut: 'Ctrl+Shift+G' },
        { label: 'Terminal', action: 'toggle-terminal', shortcut: 'Ctrl+`' }
      ]
    }
  ]

  const handleMenuAction = useCallback((action: string) => {
    setActiveMenu(null)
    switch (action) {
      case 'open-folder':
        dialog.openFolder()
          .then(result => {
            if (!result.canceled && result.filePaths.length > 0) {
              const selectedPath = result.filePaths[0]
              
              // Reset UI state FIRST
              setOpenFiles([])
              setActiveFileId(null)
              setMessages([{
                role: 'assistant',
                content: "Hello! I'm your WhizCode agent. Open a folder to get started."
              }])
              setAgentSteps([])
              setLiveStreamingContent('')
              setFileErrors({})
              
              // Set workspace path - this is the ONLY state change for workspace
              setWorkspacePath(selectedPath)
              
              // Sync with backend - this MUST happen
              workspace.setWorkspace(selectedPath)
                .catch(err => console.error('Error setting workspace:', err))
              
              setRefreshKey(prev => prev + 1)
              setActiveView('explorer')
            }
          })
          .catch(err => console.error('Error opening folder:', err))
        break
      case 'toggle-explorer':
        setActiveView(activeView === 'explorer' ? null : 'explorer')
        break
      case 'toggle-search':
        setActiveView(activeView === 'search' ? null : 'search')
        break
      case 'toggle-source-control':
        setActiveView(activeView === 'source-control' ? null : 'source-control')
        break
      case 'toggle-terminal':
        setIsTerminalOpen(prev => !prev)
        break
      case 'new-terminal':
        setIsTerminalOpen(true)
        setTerminalCreateRequest(prev => prev + 1)
        break
    }
  }, [activeView, setActiveView, setWorkspacePath, setRefreshKey, setIsTerminalOpen])

  // File operations hook
  const {
    handleFileOpen,
    handleFileSave,
    handleFileClose,
    handleFileDeleted,
    handleFileRenamed,
    handleContentChange,
    getLanguage,
    normalizePath,
    checkFileErrors,
  } = useFileOperations(openFiles, setOpenFiles, activeFileId, setActiveFileId, workspacePath)

  // Setup event listeners
  useAppEventListeners(
    setAgentSteps,
    setMessages,
    setLiveStreamingContent,
    setIsLoading,
    setAskUserPrompt,
    setRefreshKey,
    setOpenFiles,
    setWorkspacePath,
    streamingContentRef,
    STREAMING_MSG_ID
  )

  // Setup keyboard shortcuts
  useKeyboardShortcuts(
    handleFileSave,
    activeView,
    workspacePath,
    setNewFileDialog,
    setNewFolderDialog,
    setRefreshKey,
    setShowFileFilter,
    setIsTerminalOpen,
    setTerminalCreateRequest
  )

  // Persist settings
  useSettingsPersistence(
    modelProvider, model, openaiKey, geminiKey, bedrockRegion, bedrockAccessKey, bedrockSecretKey,
    azureLoginUrl, azureCompletionUrl, azureUsername, azurePassword, azureSessionToken, azureTokenExpiresAt,
    isAutopilotMode, contextLength, sidebarWidth, isChatOpen, chatWidth
  )

  // Initialize workspace
  useWorkspaceInit(
    savedState, setWorkspacePath, setRefreshKey,
    sidebarWidth, chatWidth, terminalHeight, isTerminalOpen, isChatOpen, activeView,
    workspacePath, activeFileId
  )

  // Model management
  const { refreshOllamaModels, handleGenerateAzureToken } = useModelManagement(
    modelProvider, model, setModel,
    ollamaModels, setOllamaModels,
    ollamaError, setOllamaError,
    ollamaChecking, setOllamaChecking,
    isSettingsOpen,
    azureLoginUrl, azureUsername, azurePassword,
    azureSessionToken, azureTokenExpiresAt,
    setAzureSessionToken, setAzureTokenExpiresAt,
    setAzureTokenStatus
  )

  // Git status
  useGitStatus(workspacePath, refreshKey, setGitStatus)

  // File existence check
  useFileExistenceCheck(openFiles, handleFileDeleted)

  // Auto-scroll
  useAutoScroll(messagesEndRef, messages, agentSteps)

  React.useEffect(() => {
    latestAgentStepsRef.current = agentSteps
  }, [agentSteps])

  React.useEffect(() => {
    if (!workspacePath) {
      historyLoadedWorkspaceRef.current = null
      return
    }

    if (historyLoadedWorkspaceRef.current === workspacePath) {
      return
    }

    const loadWorkspaceHistory = async () => {
      isHydratingHistoryRef.current = true
      try {
        const thread = await history.get(getWorkspaceThreadId(workspacePath))
        const savedMessages = Array.isArray(thread.messages)
          ? restoreMessagesFromHistory(thread.messages as Message[])
          : []
        if (savedMessages.length > 0) {
          setMessages(savedMessages)
        }
      } catch (error) {
        console.debug('No existing workspace history to restore:', workspacePath, error)
      } finally {
        historyLoadedWorkspaceRef.current = workspacePath
        isHydratingHistoryRef.current = false
      }
    }

    loadWorkspaceHistory()
  }, [workspacePath, setMessages])

  React.useEffect(() => {
    if (!workspacePath || isHydratingHistoryRef.current || historyLoadedWorkspaceRef.current !== workspacePath) {
      return
    }

    const persistedMessages = buildPersistedMessages(messages, isLoading, agentSteps, liveStreamingContent)
    const persistMessages = async () => {
      try {
        await history.save(
          getWorkspaceThreadId(workspacePath),
          getWorkspaceThreadTitle(workspacePath),
          persistedMessages
        )
      } catch (error) {
        console.error('Failed to persist workspace history:', error)
      }
    }

    const timeout = window.setTimeout(() => {
      persistMessages()
    }, isLoading ? 250 : 50)

    return () => window.clearTimeout(timeout)
  }, [workspacePath, messages, isLoading, agentSteps, liveStreamingContent])

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

  const handleValidation = useCallback((filePath: string, count: number) => {
    const normPath = normalizePath(filePath)
    setFileErrors(prev => {
      if (prev[normPath] === count) return prev
      const next = { ...prev }
      if (count > 0) {
        next[normPath] = count
      } else {
        delete next[normPath]
      }
      return next
    })
  }, [normalizePath, setFileErrors])

  const buildAgentModelConfig = useCallback(() => ({
    provider: modelProvider,
    model,
    openaiKey,
    geminiKey,
    bedrockRegion,
    bedrockAccessKey,
    bedrockSecretKey,
    azureLoginUrl,
    azureCompletionUrl,
    azureUsername,
    azureSessionToken: azureSessionToken.trim() && azureTokenExpiresAt > Date.now() ? azureSessionToken : '',
  }), [
    modelProvider,
    model,
    openaiKey,
    geminiKey,
    bedrockRegion,
    bedrockAccessKey,
    bedrockSecretKey,
    azureLoginUrl,
    azureCompletionUrl,
    azureUsername,
    azureSessionToken,
    azureTokenExpiresAt,
  ])

  const handleSend = async (overrideInput?: string) => {
    const textToSend = overrideInput || input
    if (!textToSend.trim() && selectedImages.length === 0 || isLoading) return
    const userMsg: Message = { role: 'user', content: textToSend, images: selectedImages.length > 0 ? [...selectedImages] : undefined }
    setMessages(prev => [...prev, userMsg])
    if (!overrideInput) setInput('')
    setSelectedImages([])
    setIsLoading(true)
    setAgentSteps([]) // Clear previous steps for new task
    setLiveStreamingContent('')

    streamingContentRef.current = ''

    try {
      if (!workspacePath) {
        setMessages(prev => [...prev, {
          role: 'assistant',
          content: '⚠️ No workspace open. Please open a folder first (File → Open Folder) to use the agent.'
        }])
        setIsLoading(false)
        return
      }

      const activeFile = openFiles.find(f => f.path === activeFileId)
      console.log('[AGENT] Executing loop with workspacePath:', workspacePath)

      const conversationHistory = messages
        .filter(m => !(m as any).__id && m.content !== 'Hello! I\'m your WhizCode agent. Open a folder to get started.')
        .slice(-20)

      const result = await agent.executeLoopStreaming({
        task: userMsg.content,
        model: buildAgentModelConfig(),
        workspacePath: workspacePath,
        activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
        conversationHistory,
        context_length: contextLength,
      })
      const finalSteps = collectResultSteps(result, latestAgentStepsRef.current)
      const response = resolveAgentResponse(result, finalSteps)
      let verificationSteps = workspacePath
        ? await buildVerificationSteps(workspacePath, finalSteps)
        : []
      let allSteps = [...finalSteps, ...verificationSteps]
      let finalResponse = appendVerificationSummary(response, verificationSteps)

      const repairOutcome = workspacePath
        ? await runAutomaticRepairPass({
            workspacePath,
            originalTask: userMsg.content,
            initialResponse: response,
            initialSteps: finalSteps,
            initialVerificationSteps: verificationSteps,
            conversationHistory: [
              ...conversationHistory,
              { role: 'user', content: userMsg.content },
              { role: 'assistant', content: finalResponse },
            ],
            modelConfig: buildAgentModelConfig(),
            activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
            contextLength,
            latestAgentStepsRef,
            setAgentSteps,
            setLiveStreamingContent,
            streamingContentRef,
          })
        : null

      if (repairOutcome) {
        allSteps = repairOutcome.steps
        verificationSteps = repairOutcome.verificationSteps
        finalResponse = repairOutcome.response
      }

      setAgentSteps([])
      setMessages(prev => {
        return [...prev, { role: 'assistant', content: finalResponse, steps: allSteps.length > 0 ? allSteps : undefined }]
      })
    } catch (err) {
      console.error('Agent error:', err)
      setMessages(prev => {
        return [...prev, { role: 'assistant', content: `Error communicating with agent: ${err instanceof Error ? err.message : String(err)}` }]
      })
    } finally {
      setIsLoading(false)
    }
  }

  const handlePermissionResponse = async (approved: boolean, stepIdx?: number) => {
    try {
      let requestId: string | undefined

      if (stepIdx !== undefined && agentSteps[stepIdx]) {
        const step = agentSteps[stepIdx]
        requestId = (step as any).requestId
        setMessages(prev => [...prev, {
          role: 'assistant',
          content: approved ? `✅ **Permission granted**: ${step.summary}` : `❌ **Permission denied**: ${step.summary}`
        }])
      }

      if (requestId) {
        await agent.sendPermissionResponse(approved, requestId)
      }
    } catch (error) {
      console.error('Error responding to permission request:', error)
    }
  }

  const handleStop = async () => {
    try {
      await agent.stop()
      setIsLoading(false)
    } catch (error) {
      console.error('Error stopping agent:', error)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: 'var(--bg-primary)' }}>
      <TitleBar 
        menus={menus}
        activeMenu={activeMenu}
        toggleMenu={(menu) => setActiveMenu(activeMenu === menu ? null : menu)}
        handleMenuHover={(menu) => activeMenu && setActiveMenu(menu)}
        handleMenuAction={handleMenuAction}
      />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <ActivityBar 
          activeView={activeView} 
          setActiveView={setActiveView}
          isChatOpen={isChatOpen}
          setIsChatOpen={setIsChatOpen}
          isTerminalOpen={isTerminalOpen}
          setIsTerminalOpen={setIsTerminalOpen}
        />

        {/* Sidebar */}
        <div style={{
          width: `${sidebarWidth}px`,
          backgroundColor: 'var(--sidebar-bg)',
          borderRight: '1px solid var(--border-color)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden'
        }}>
          {/* Sidebar Header */}
          <div style={{
            padding: '12px',
            borderBottom: '1px solid var(--border-color)',
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--text-secondary)',
            textTransform: 'uppercase',
            display: 'flex',
            alignItems: 'center',
            gap: '8px'
          }}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            <span style={{ flex: 1 }}>{workspacePath ? workspacePath.split(/[/\\]/).pop() : 'Explorer'}</span>
            <div className="sidebar-header-actions">
              <button 
                className="sidebar-action-btn" 
                onClick={() => {
                  setNewItemName('');
                  setNewFileDialog({ parentPath: workspacePath || '' });
                }} 
                title="New File"
              >
                <FiPlus />
              </button>
              <button 
                className="sidebar-action-btn" 
                onClick={() => {
                  setNewItemName('');
                  setNewFolderDialog({ parentPath: workspacePath || '' });
                }} 
                title="New Folder"
              >
                <FiFolderPlus />
              </button>
              <button 
                className="sidebar-action-btn" 
                onClick={() => setRefreshKey(prev => prev + 1)} 
                title="Refresh"
              >
                <FiRotateCw />
              </button>
              <button 
                className="sidebar-action-btn" 
                onClick={() => {
                  setCollapseAll(true);
                  setTimeout(() => setCollapseAll(false), 100);
                }} 
                title="Collapse All"
              >
                <FiMinimize2 />
              </button>
            </div>
          </div>

          {/* Sidebar Content */}
          <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
            {activeView === 'explorer' && (
              <FileTree
                path={workspacePath || ''}
                onFileOpen={handleFileOpen}
                onFileDeleted={handleFileDeleted}
                onFileRenamed={handleFileRenamed}
                refreshKey={refreshKey}
                collapseAll={collapseAll}
                fileFilter={fileFilter}
                fileErrors={fileErrors}
                gitStatus={gitStatus}
              />
            )}
            {activeView === 'search' && <SearchPanel workspacePath={workspacePath} onFileOpen={handleFileOpen} />}
            {activeView === 'source-control' && <SourceControlPanel workspacePath={workspacePath} />}
          </div>
        </div>

        {/* Sidebar resize handle */}
        <div
          onMouseDown={handleSidebarResize}
          style={{
            width: '4px',
            backgroundColor: 'transparent',
            cursor: 'col-resize',
            transition: 'background-color 0.2s ease'
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLDivElement).style.backgroundColor = 'var(--accent-color)'
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLDivElement).style.backgroundColor = 'transparent'
          }}
        />

        {/* Main content area */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minWidth: 0 }}>
          {activeView === 'preview' ? (
            <WebPreview />
          ) : (
            <EditorArea
              openFiles={openFiles}
              activeFileId={activeFileId}
              setActiveFileId={setActiveFileId}
              workspacePath={workspacePath}
              handleFileClose={handleFileClose}
              getLanguage={getLanguage}
              handleContentChange={handleContentChange}
              handleMenuAction={handleMenuAction}
              fileErrors={fileErrors}
              onValidation={handleValidation}
            />
          )}

          <MultiTerminalPane
            isOpen={isTerminalOpen}
            height={terminalHeight}
            onHeightChange={setTerminalHeight}
            workspacePath={workspacePath}
            createRequest={terminalCreateRequest}
          />
        </div>

        {/* Chat resize handle */}
        {isChatOpen && (
          <div
            onMouseDown={handleChatResize}
            style={{
              width: '4px',
              backgroundColor: 'transparent',
              cursor: 'col-resize',
              transition: 'background-color 0.2s ease'
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLDivElement).style.backgroundColor = 'var(--accent-color)'
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLDivElement).style.backgroundColor = 'transparent'
            }}
          />
        )}

        {/* Chat panel */}
        {isChatOpen && (
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
            handleReset={() => {
              setMessages([{ role: 'assistant', content: 'Hello! I\'m your WhizCode agent. Open a folder to get started.' }])
              setAgentSteps([])
            }}
            handleKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                handleSend()
              }
            }}
            getToolIcon={(tool: string) => {
              const icons: Record<string, string> = {
                'read_file': '📖',
                'write_file': '✍️',
                'execute_command': '⚙️',
                'search': '🔍',
                'analyze': '🔬',
              }
              return icons[tool] || '🔧'
            }}
            messagesEndRef={messagesEndRef}
            handlePermissionResponse={handlePermissionResponse}
            handleStop={handleStop}
            settingsProps={{
              isSettingsOpen, setIsSettingsOpen,
              modelProvider, setModelProvider, model, setModel,
              ollamaModels, ollamaChecking, ollamaError, refreshOllamaModels,
              openaiKey, setOpenaiKey, geminiKey, setGeminiKey,
              bedrockRegion, setBedrockRegion, bedrockAccessKey, setBedrockAccessKey, bedrockSecretKey, setBedrockSecretKey,
              azureLoginUrl, setAzureLoginUrl, azureCompletionUrl, setAzureCompletionUrl,
              azureUsername, setAzureUsername, azurePassword, setAzurePassword,
              azureTokenStatus, onGenerateAzureToken: handleGenerateAzureToken,
              isAutopilotMode, setIsAutopilotMode,
              contextLength, setContextLength
            }}
            liveStreamingContent={liveStreamingContent}
            selectedImages={selectedImages}
            setSelectedImages={setSelectedImages}
          />
        )}
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
          {agentSteps.length > 0 && (
            <div style={{ fontSize: '11px', opacity: 0.9 }}>
              Iteration: {Math.max(...agentSteps.map(s => s.iteration || 0))}
            </div>
          )}
          <div style={{ fontSize: '11px', opacity: 0.9 }}>
            {modelProvider === 'ollama' ? 'Ollama' :
              modelProvider === 'openai' ? 'OpenAI' :
                modelProvider === 'gemini' ? 'Gemini' : 'Bedrock'}: {model}
          </div>
        </div>
      </div>

      {/* New File Dialog */}
      {/* Removed - FileTree handles this internally */}

      {/* New Folder Dialog */}
      {/* Removed - FileTree handles this internally */}

      {/* Ask User Dialog */}
      {/* New File/Folder Modal */}
      {(newFileDialog || newFolderDialog) && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          backgroundColor: 'rgba(0, 0, 0, 0.5)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 1000
        }}>
          <div style={{
            backgroundColor: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: '8px',
            padding: '24px',
            minWidth: '300px',
            boxShadow: '0 20px 60px rgba(0, 0, 0, 0.3)'
          }}>
            <h3 style={{ marginTop: 0, marginBottom: '12px', color: 'var(--text-primary)' }}>
              New {newFileDialog ? 'File' : 'Folder'}
            </h3>
            <input 
              autoFocus
              className="file-filter-input"
              value={newItemName}
              onChange={(e) => setNewItemName(e.target.value)}
              placeholder={`Enter ${newFileDialog ? 'file' : 'folder'} name...`}
              onKeyDown={async (e) => {
                if (e.key === 'Enter' && newItemName.trim()) {
                  try {
                    const separator = workspacePath?.includes('\\') ? '\\' : '/'
                    const parent = (newFileDialog?.parentPath || newFolderDialog?.parentPath || workspacePath || '').replace(/[/\\]$/, '')
                    const itemPath = `${parent}${separator}${newItemName}`
                    if (newFileDialog) await fs.createFile(itemPath)
                    else await fs.createDirectory(itemPath)
                    setNewFileDialog(null)
                    setNewFolderDialog(null)
                    setNewItemName('')
                    setRefreshKey(prev => prev + 1)
                  } catch (err) {
                    alert('Creation failed: ' + err)
                  }
                } else if (e.key === 'Escape') {
                  setNewFileDialog(null)
                  setNewFolderDialog(null)
                }
              }}
              style={{ marginBottom: '16px' }}
            />
            <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
              <button 
                onClick={() => { setNewFileDialog(null); setNewFolderDialog(null); }}
                style={{ padding: '6px 12px', background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)', color: 'white', borderRadius: '4px' }}
              >
                Cancel
              </button>
              <button 
                onClick={async () => {
                  if (!newItemName.trim()) return
                  try {
                    const separator = workspacePath?.includes('\\') ? '\\' : '/'
                    const parent = (newFileDialog?.parentPath || newFolderDialog?.parentPath || workspacePath || '').replace(/[/\\]$/, '')
                    const itemPath = `${parent}${separator}${newItemName}`
                    if (newFileDialog) await fs.createFile(itemPath)
                    else await fs.createDirectory(itemPath)
                    setNewFileDialog(null)
                    setNewFolderDialog(null)
                    setNewItemName('')
                    setRefreshKey(prev => prev + 1)
                  } catch (err) {
                    alert('Creation failed: ' + err)
                  }
                }}
                style={{ padding: '6px 12px', background: 'var(--accent-primary)', border: 'none', color: 'white', borderRadius: '4px' }}
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {askUserPrompt && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          backgroundColor: 'rgba(0, 0, 0, 0.5)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 1000
        }}>
          <div style={{
            backgroundColor: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: '8px',
            padding: '24px',
            maxWidth: '400px',
            boxShadow: '0 20px 60px rgba(0, 0, 0, 0.3)'
          }}>
            <h3 style={{ marginTop: 0, marginBottom: '12px', color: 'var(--text-primary)' }}>Agent Question</h3>
            <p style={{ marginBottom: '20px', color: 'var(--text-secondary)' }}>{askUserPrompt.question}</p>
            <div style={{ display: 'flex', gap: '12px', justifyContent: 'flex-end' }}>
              <button 
                onClick={() => {
                  handlePermissionResponse(false)
                  setAskUserPrompt(null)
                }}
                style={{
                  padding: '8px 16px',
                  backgroundColor: 'var(--bg-tertiary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '13px'
                }}
              >
                No
              </button>
              <button 
                onClick={() => {
                  handlePermissionResponse(true)
                  setAskUserPrompt(null)
                }}
                style={{
                  padding: '8px 16px',
                  backgroundColor: 'var(--accent-primary)',
                  color: 'white',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '13px'
                }}
              >
                Yes
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function getWorkspaceThreadId(workspacePath: string): string {
  const slug = workspacePath
    .replace(/^[A-Za-z]:/, '')
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase()

  return `workspace-main-${slug || 'default'}`
}

function getWorkspaceThreadTitle(workspacePath: string): string {
  const parts = workspacePath.split(/[/\\]/).filter(Boolean)
  return parts[parts.length - 1] || 'Workspace Chat'
}

function buildPersistedMessages(
  messages: Message[],
  isLoading: boolean,
  agentSteps: AgentStep[],
  liveStreamingContent: string
): Message[] {
  const baseMessages = messages.filter(message => !(message as any).__whizcodeDraft)

  if (!isLoading) {
    return baseMessages
  }

  const hasLiveTaskState = agentSteps.length > 0 || liveStreamingContent.trim().length > 0
  if (!hasLiveTaskState) {
    return baseMessages
  }

  const draftMessage: Message & { __whizcodeDraft: true; interruptedAt: number } = {
    role: 'assistant',
    content: liveStreamingContent.trim() || 'Task was in progress when the app closed. The latest live logs are preserved below.',
    steps: agentSteps.length > 0 ? [...agentSteps] : undefined,
    __whizcodeDraft: true,
    interruptedAt: Date.now(),
  }

  return [...baseMessages, draftMessage]
}

function restoreMessagesFromHistory(messages: Message[]): Message[] {
  return messages.map(message => {
    if (!(message as any).__whizcodeDraft) {
      return message
    }

    const restoredContent = (message.content || '').trim()
    return {
      role: 'assistant',
      content: restoredContent
        ? `Recovered interrupted task:\n\n${restoredContent}`
        : 'Recovered interrupted task. The latest available logs are preserved below.',
      steps: Array.isArray(message.steps) ? message.steps : undefined,
    }
  })
}

function collectResultSteps(result: any, liveSteps: AgentStep[]): AgentStep[] {
  const steps = result?.steps || []
  const toolCalls = result?.tool_calls || []

  const toolSteps = toolCalls.map((call: any, idx: number) => ({
    tool: call.tool,
    iteration: idx + 1,
    status: 'done' as const,
    summary: `Executed ${call.tool} with args: ${JSON.stringify(call.args)}`,
  }))

  return liveSteps.length > 0
    ? [...liveSteps]
    : [...toolSteps, ...steps]
}

function resolveAgentResponse(result: any, steps: AgentStep[]): string {
  const rawResponse = typeof result?.response === 'string' ? result.response.trim() : ''
  if (rawResponse.length > 0 && rawResponse !== 'No response') {
    return rawResponse
  }

  const failedStep = [...steps].reverse().find(step =>
    step.status === 'failed' || step.status === 'error'
  )

  if (failedStep?.result?.trim()) {
    return `Task failed: ${failedStep.result.trim()}`
  }

  if (failedStep?.summary?.trim()) {
    return failedStep.summary.trim()
  }

  return 'No response'
}

function shouldRunVerification(steps: AgentStep[]): boolean {
  return steps.some(step => [
    'write_file',
    'edit_file',
    'run_command',
    'create_file',
    'delete_file',
    'rename_file',
    'git',
  ].includes(step.tool))
}

function getFailedReviewStep(verificationSteps: AgentStep[]): AgentStep | undefined {
  return verificationSteps.find(step => step.tool === 'review' && step.status === 'failed')
}

function summarizeReviewFindings(reviewStep: AgentStep): string[] {
  return (reviewStep.logs || [])
    .filter(log => typeof log === 'string' && log.trim().length > 0)
    .slice(0, 5)
}

function remapRepairSteps(steps: AgentStep[], iterationOffset: number): AgentStep[] {
  return steps.map((step, index) => ({
    ...step,
    iteration: (step.iteration || index + 1) + iterationOffset,
    requestId: step.requestId ? `repair-${step.requestId}` : `repair-step-${iterationOffset + index + 1}`,
  }))
}

async function buildVerificationSteps(workspacePath: string, steps: AgentStep[]): Promise<AgentStep[]> {
  if (!shouldRunVerification(steps)) {
    return []
  }

  try {
    const report = await git.reviewWorkingTree(workspacePath)
    const findingsText = report.findings.length > 0
      ? report.findings
          .slice(0, 10)
          .map(finding => `${finding.severity.toUpperCase()} ${finding.file}:${finding.line} ${finding.message}${finding.suggestion ? ` (${finding.suggestion})` : ''}`)
      : ['No review findings in changed files.']

    const reviewStep: AgentStep = {
      tool: 'review',
      status: report.findings.length > 0 ? 'failed' : 'completed',
      summary: report.findings.length > 0
        ? `Review found ${report.findings.length} issue(s) across ${report.files_reviewed} file(s)`
        : `Review passed for ${report.files_reviewed} file(s)`,
      logs: findingsText,
      persona: 'reviewer',
      planPhase: 'summary',
    }

    if (report.findings.length === 0) {
      return [reviewStep]
    }

    const primaryFinding = report.findings[0]
    const recovery = await errorRecovery.handle(
      `${primaryFinding.severity}: ${primaryFinding.message}`,
      'review',
      workspacePath
    )

    const recoveryLogs = [
      recovery.message,
      ...(recovery.suggested_action ? [`Suggested action: ${recovery.suggested_action}`] : []),
      ...recovery.fallback_recommendations.map(item => `Fallback: ${item}`),
    ]

    const recoveryStep: AgentStep = {
      tool: 'recovery',
      status: 'alternative',
      summary: 'Recovery guidance prepared from review findings',
      logs: recoveryLogs,
      persona: 'reviewer',
      planPhase: 'summary',
    }

    return [reviewStep, recoveryStep]
  } catch (error) {
    console.debug('Automatic verification skipped:', error)
    return []
  }
}

function appendVerificationSummary(response: string, verificationSteps: AgentStep[]): string {
  if (verificationSteps.length === 0) {
    return response
  }

  const reviewStep = verificationSteps.find(step => step.tool === 'review')
  if (!reviewStep) {
    return response
  }

  return `${response}\n\nVerification: ${reviewStep.summary}`
}

async function runAutomaticRepairPass(params: {
  workspacePath: string
  originalTask: string
  initialResponse: string
  initialSteps: AgentStep[]
  initialVerificationSteps: AgentStep[]
  conversationHistory: Array<{ role: string; content: string }>
  modelConfig: Record<string, unknown>
  activeFile: { path: string; content: string } | null
  contextLength: number
  latestAgentStepsRef: React.MutableRefObject<AgentStep[]>
  setAgentSteps: (steps: AgentStep[] | ((prev: AgentStep[]) => AgentStep[])) => void
  setLiveStreamingContent: (content: string) => void
  streamingContentRef: React.MutableRefObject<string>
}): Promise<{ steps: AgentStep[]; verificationSteps: AgentStep[]; response: string } | null> {
  const failedReview = getFailedReviewStep(params.initialVerificationSteps)
  if (!failedReview) {
    return null
  }

  const findings = summarizeReviewFindings(failedReview)
  if (findings.length === 0) {
    return null
  }

  const repairRequest = [
    `Automatic repair pass for the previous task: ${params.originalTask}`,
    'Address the verification findings below with the smallest safe set of code changes.',
    'Do not make unrelated refactors. When you finish, stop and summarize what you changed.',
    '',
    'Verification findings:',
    ...findings.map(item => `- ${item}`),
  ].join('\n')

  params.setAgentSteps([])
  params.streamingContentRef.current = ''
  params.setLiveStreamingContent('')

  try {
    const repairResult = await agent.executeLoopStreaming({
      task: repairRequest,
      model: params.modelConfig,
      workspacePath: params.workspacePath,
      activeFile: params.activeFile,
      conversationHistory: params.conversationHistory,
      context_length: params.contextLength,
    })

    const repairResponse = repairResult?.response || 'Automatic repair pass completed.'
    const repairSteps = remapRepairSteps(
      collectResultSteps(repairResult, params.latestAgentStepsRef.current),
      Math.max(...params.initialSteps.map(step => step.iteration || 0), 0),
    )
    const combinedSteps = [...params.initialSteps, ...params.initialVerificationSteps, ...repairSteps]
    const reverifiedSteps = await buildVerificationSteps(params.workspacePath, combinedSteps)
    const reviewAfterRepair = getFailedReviewStep(reverifiedSteps)
    const repairNote = reviewAfterRepair
      ? `Auto-repair attempted one corrective pass, but ${reviewAfterRepair.summary.toLowerCase()}.`
      : 'Auto-repair applied one corrective pass and verification passed.'
    const response = `${appendVerificationSummary(params.initialResponse, reverifiedSteps)}\n\n${repairNote}\nRepair summary: ${repairResponse}`

    return {
      steps: [...combinedSteps, ...reverifiedSteps],
      verificationSteps: reverifiedSteps,
      response,
    }
  } catch (error) {
    const failureMessage = error instanceof Error ? error.message : String(error)
    const failureStep: AgentStep = {
      tool: 'repair',
      status: 'failed',
      summary: 'Automatic repair pass failed',
      logs: [failureMessage],
      persona: 'reviewer',
      planPhase: 'summary',
      iteration: Math.max(...params.initialSteps.map(step => step.iteration || 0), 0) + 1,
      requestId: 'repair-failed',
    }

    return {
      steps: [...params.initialSteps, ...params.initialVerificationSteps, failureStep],
      verificationSteps: params.initialVerificationSteps,
      response: `${appendVerificationSummary(params.initialResponse, params.initialVerificationSteps)}\n\nAuto-repair attempted one corrective pass but failed: ${failureMessage}`,
    }
  }
}

export default App
