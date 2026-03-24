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
import type { Message } from './types'

import { agent, dialog, workspace, fs } from './lib/tauri-api'
import { loadAppState } from './lib/appState'

import { WhizLogo } from './components/Branding/WhizLogo'
import { FiPlus, FiFolderPlus, FiRotateCw, FiMinimize2, FiFolder } from 'react-icons/fi'
import './App.css'


function App() {
  const savedState = loadAppState()

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
    azureEmbeddingUrl, setAzureEmbeddingUrl,
    azureCompletionUrl, setAzureCompletionUrl,
    azureUsername, setAzureUsername,
    azurePassword, setAzurePassword,
    azureTokenStatus, setAzureTokenStatus,
    isAutopilotMode, setIsAutopilotMode,
    contextLength, setContextLength,
    messagesEndRef,
    streamingContentRef,
    STREAMING_MSG_ID,
  } = appState

  // Menu state
  const [activeMenu, setActiveMenu] = React.useState<string | null>(null)

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
        { label: 'Source Control', action: 'toggle-source-control', shortcut: 'Ctrl+Shift+G' }
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
              
              // 1. Reset everything first for a clean state
              setWorkspacePath(null) // Temporarily clear to force sub-components to unmount/reset
              setOpenFiles([])
              setActiveFileId(null)
              setMessages([{
                role: 'assistant',
                content: "Hello! I'm your WhizCode agent. Open a folder to get started."
              }])
              setAgentSteps([])
              setLiveStreamingContent('')
              setFileErrors({})
              
              // 2. Set new workspace
              setTimeout(() => {
                  setWorkspacePath(selectedPath)
                  workspace.setWorkspace(selectedPath).catch(err => console.error('Error setting workspace:', err))
                  setRefreshKey(prev => prev + 1)
                  setActiveView('explorer')
              }, 50)
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
    }
  }, [activeView, setActiveView, setWorkspacePath, setRefreshKey])

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
    setShowFileFilter
  )

  // Persist settings
  useSettingsPersistence(
    modelProvider, model, openaiKey, geminiKey, bedrockRegion, bedrockAccessKey, bedrockSecretKey,
    azureLoginUrl, azureEmbeddingUrl, azureCompletionUrl, azureUsername, azurePassword,
    isAutopilotMode, contextLength, sidebarWidth, isChatOpen, chatWidth
  )

  // Initialize workspace
  useWorkspaceInit(
    savedState, setWorkspacePath, setRefreshKey,
    sidebarWidth, chatWidth, isChatOpen, activeView,
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
    setAzureTokenStatus
  )

  // Git status
  useGitStatus(workspacePath, refreshKey, setGitStatus)

  // File existence check
  useFileExistenceCheck(openFiles, handleFileDeleted)

  // Auto-scroll
  useAutoScroll(messagesEndRef, messages, agentSteps)

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
        model: {
          provider: modelProvider,
          model: model,
        },
        workspacePath: workspacePath,
        activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
        conversationHistory,
        context_length: contextLength,
      })
      const response = result?.response || 'No response'
      const steps = result?.steps || []
      const toolCalls = result?.tool_calls || []
      
      const toolSteps = toolCalls.map((call: any, idx: number) => ({
        tool: call.tool,
        iteration: idx + 1,
        status: 'done' as const,
        summary: `Executed ${call.tool} with args: ${JSON.stringify(call.args)}`
      }))
      
      setAgentSteps([])
      setMessages(prev => {
        const withoutStream = prev.filter(m => (m as any).__id !== STREAMING_MSG_ID)
        return [...withoutStream, { role: 'assistant', content: response, steps: [...toolSteps, ...steps].length > 0 ? [...toolSteps, ...steps] : undefined }]
      })
    } catch (err) {
      console.error('Agent error:', err)
      setMessages(prev => {
        const withoutStream = prev.filter(m => (m as any).__id !== STREAMING_MSG_ID)
        return [...withoutStream, { role: 'assistant', content: `Error communicating with agent: ${err instanceof Error ? err.message : String(err)}` }]
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
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
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
              azureLoginUrl, setAzureLoginUrl, azureEmbeddingUrl, setAzureEmbeddingUrl, azureCompletionUrl, setAzureCompletionUrl,
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

export default App
