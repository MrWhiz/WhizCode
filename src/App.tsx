import React, { useState, useRef, useEffect } from 'react'

// Components
import { TitleBar } from './components/Layout/TitleBar'
import { ActivityBar } from './components/Layout/ActivityBar'
import { FileTree } from './components/Explorer/FileTree'
import { EditorArea } from './components/Editor/EditorArea'
import { ChatPanel } from './components/Chat/ChatPanel'
import { TerminalPane } from './components/Terminal/TerminalPane'

// Types
import type { Message, AgentStep, OpenFileProps, AIProvider } from './types'

import './App.css'

function App() {
  const [input, setInput] = useState('')
  const [messages, setMessages] = useState<Message[]>([
    { role: 'assistant', content: 'Hello! I\'m your WhizCode agent. Open a folder to get started — I\'ll read your project files and help you code, debug, and build.' }
  ])
  const [isLoading, setIsLoading] = useState(false)
  const [agentSteps, setAgentSteps] = useState<AgentStep[]>([])

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const [activeMenu, setActiveMenu] = useState<string | null>(null)
  const [workspacePath, setWorkspacePath] = useState<string | null>(null)
  const [activeFileId, setActiveFileId] = useState<string | null>(null)
  const [openFiles, setOpenFiles] = useState<OpenFileProps[]>([])
  const [isSidebarOpen, setIsSidebarOpen] = useState(true)
  const [sidebarWidth, setSidebarWidth] = useState(250)
  const [isTerminalOpen, setIsTerminalOpen] = useState(false)
  const [terminalHeight, setTerminalHeight] = useState(250)

  // Chat Panel
  const [isChatOpen, setIsChatOpen] = useState(true)
  const [chatWidth, setChatWidth] = useState(400)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)

  // AI Settings Persistence (Planner)
  const [plannerProvider, setPlannerProvider] = useState<AIProvider>(() => (localStorage.getItem('plannerProvider') as AIProvider) || 'ollama')
  const [plannerModel, setPlannerModel] = useState(() => localStorage.getItem('plannerModel') || 'llama3')

  // AI Settings Persistence (Executor)
  const [executorProvider, setExecutorProvider] = useState<AIProvider>(() => (localStorage.getItem('executorProvider') as AIProvider) || 'ollama')
  const [executorModel, setExecutorModel] = useState(() => localStorage.getItem('executorModel') || 'llama3')

  const [openaiKey, setOpenaiKey] = useState(() => localStorage.getItem('openaiKey') || '')
  const [geminiKey, setGeminiKey] = useState(() => localStorage.getItem('geminiKey') || '')

  const [ollamaModels, setOllamaModels] = useState<string[]>([])
  const [ollamaError, setOllamaError] = useState<string | null>(null)
  const [ollamaChecking, setOllamaChecking] = useState(false)

  // Save Settings on Change
  useEffect(() => {
    localStorage.setItem('plannerProvider', plannerProvider)
    localStorage.setItem('plannerModel', plannerModel)
    localStorage.setItem('executorProvider', executorProvider)
    localStorage.setItem('executorModel', executorModel)
    localStorage.setItem('openaiKey', openaiKey)
    localStorage.setItem('geminiKey', geminiKey)
  }, [plannerProvider, plannerModel, executorProvider, executorModel, openaiKey, geminiKey])

  // Handle Ollama Models
  useEffect(() => {
    if (isSettingsOpen && (plannerProvider === 'ollama' || executorProvider === 'ollama')) {
      refreshOllamaModels();
    }
  }, [isSettingsOpen, plannerProvider, executorProvider]);

  const refreshOllamaModels = async () => {
    const ipc = (window as any).ipcRenderer;
    if (!ipc) return;

    setOllamaChecking(true);
    setOllamaError(null);
    try {
      const res = await ipc.invoke('ollama:getModels');
      if (res.error) {
        setOllamaError("Ollama is not running. Please start the Ollama desktop app.");
        setOllamaModels([]);
      } else {
        setOllamaModels(res);
        setOllamaError(null);
        if (res.length > 0) {
          if (!res.includes(plannerModel)) setPlannerModel(res[0]);
          if (!res.includes(executorModel)) setExecutorModel(res[0]);
        }
      }
    } catch {
      setOllamaError("Could not connect to Ollama.");
      setOllamaModels([]);
    } finally {
      setOllamaChecking(false);
    }
  };

  // Resize Handlers
  const handleSidebarResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth;
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = startWidth + (moveEvent.clientX - startX);
      setSidebarWidth(Math.max(160, Math.min(newWidth, 600)));
    };
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  };

  const handleTerminalResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = terminalHeight;
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(100, startHeight - (moveEvent.clientY - startY));
      setTerminalHeight(Math.min(newHeight, window.innerHeight - 100));
    };
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  };


  const handleChatResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = chatWidth;
    const onMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(280, startWidth - (moveEvent.clientX - startX));
      setChatWidth(Math.min(newWidth, window.innerWidth - 400));
    };
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  };

  // Fyle Operations
  const handleFileOpen = async (path: string, name: string) => {
    const existingFile = openFiles.find(f => f.path === path)
    if (existingFile) {
      setActiveFileId(path)
      return
    }
    const ipc = (window as any).ipcRenderer;
    if (ipc) {
      const content = await ipc.invoke('fs:readFile', path);
      if (content !== null) {
        setOpenFiles(prev => [...prev, { path, name, content }]);
        setActiveFileId(path);
      }
    }
  }

  const handleFileSave = async () => {
    const activeFile = openFiles.find(f => f.path === activeFileId)
    if (!activeFile) return;
    const ipc = (window as any).ipcRenderer;
    if (ipc) {
      const success = await ipc.invoke('fs:writeFile', activeFile.path, activeFile.content);
      if (success) console.log('File saved');
    }
  }

  const handleFileClose = (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setOpenFiles(prev => {
      const newFiles = prev.filter(f => f.path !== path);
      if (activeFileId === path) {
        setActiveFileId(newFiles.length > 0 ? newFiles[newFiles.length - 1].path : null);
      }
      return newFiles;
    });
  }

  const handleContentChange = (newContent: string | undefined) => {
    if (newContent !== undefined) {
      setOpenFiles(prev => prev.map(f => f.path === activeFileId ? { ...f, content: newContent } : f))
    }
  }

  const getLanguage = (fileName: string) => {
    const ext = fileName.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts': case 'tsx': return 'typescript';
      case 'js': case 'jsx': return 'javascript';
      case 'json': return 'json';
      case 'html': return 'html';
      case 'css': return 'css';
      case 'md': return 'markdown';
      case 'py': return 'python';
      default: return 'plaintext';
    }
  }

  // UI Utilities
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  useEffect(() => {
    scrollToBottom()
  }, [messages, agentSteps])

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (!(e.target as Element).closest('.menu-item')) setActiveMenu(null)
    }
    window.addEventListener('click', handleClickOutside)
    return () => window.removeEventListener('click', handleClickOutside)
  }, [])

  const handleMenuAction = async (action: string) => {
    setActiveMenu(null)
    const ipc = (window as any).ipcRenderer;
    if (!ipc) return;

    if (action === 'exit') ipc.send('app:exit');
    else if (action === 'new-terminal') setIsTerminalOpen(true);
    else if (action === 'open-folder') {
      const result = await ipc.invoke('dialog:openFolder');
      if (result && !result.canceled && result.filePaths?.length > 0) {
        setWorkspacePath(result.filePaths[0]);
      }
    } else if (action === 'save') {
      await handleFileSave();
    }
  }

  // Agent Logic
  const handleSend = async () => {
    if (!input.trim() || isLoading) return
    const userMsg: Message = { role: 'user', content: input }
    setMessages(prev => [...prev, userMsg])
    setInput('')
    setIsLoading(true)
    setAgentSteps([])

    const ipc = (window as any).ipcRenderer;
    const stepHandler = (_event: any, step: AgentStep) => {
      setAgentSteps(prev => {
        const existingIdx = prev.findIndex(s =>
          s.tool === step.tool &&
          (step.iteration !== undefined ? s.iteration === step.iteration : s.status === 'running')
        );
        if (existingIdx >= 0) {
          const newSteps = [...prev];
          newSteps[existingIdx] = step;
          return newSteps;
        }
        return [...prev, step];
      });
    };

    if (ipc) ipc.on('agent:step', stepHandler);

    try {
      if (ipc) {
        const activeFile = openFiles.find(f => f.path === activeFileId);
        const result = await ipc.invoke('execute-agent-task', {
          task: userMsg.content,
          planner: { provider: plannerProvider, model: plannerModel },
          executor: { provider: executorProvider, model: executorModel },
          workspacePath,
          activeFile: activeFile ? { path: activeFile.path, content: activeFile.content } : null,
          config: { openaiKey, geminiKey }
        })
        const response = typeof result === 'string' ? result : result?.response || 'No response';
        const steps = typeof result === 'object' ? result?.steps || [] : [];
        setMessages(prev => [...prev, { role: 'assistant', content: response, steps: steps.length > 0 ? steps : undefined }])
      }
    } catch (err) {
      setMessages(prev => [...prev, { role: 'assistant', content: 'Error communicating with agent.' }])
    } finally {
      setIsLoading(false)
      setAgentSteps([])
      if (ipc) ipc.off('agent:step', stepHandler);
    }
  }

  const handlePermissionResponse = async (approved: boolean, _stepIdx?: number) => {
    const ipc = (window as any).ipcRenderer;
    if (ipc) {
      await ipc.invoke('agent:permission-response', { approved });
    }
  }

  const handleStop = async () => {
    const ipc = (window as any).ipcRenderer;
    if (ipc) {
      await ipc.invoke('agent:stop');
    }
  }

  const handleReset = async () => {
    const ipc = (window as any).ipcRenderer;
    if (ipc) await ipc.invoke('agent:reset');
    setMessages([{ role: 'assistant', content: 'Conversation reset. How can I help you now?' }]);
    setAgentSteps([]);
  }

  const getToolIcon = (tool: string): string => {
    switch (tool) {
      case 'read_file': return '📄';
      case 'write_file': return '✏️';
      case 'edit_file': case 'replace_lines': case 'insert_code': return '🔧';
      case 'list_directory': return '📂';
      case 'search_files': return '🔍';
      case 'run_command': return '⚡';
      case 'apply_diffs': return '🚀';
      case 'validate_project': return '🛡️';
      case 'run_tests': return '🧪';
      case 'indexing_workspace': return '📦';
      case 'planning': return '📋';
      default: return '🛠️';
    }
  }

  const menus = [
    { name: 'File', items: [{ label: 'Open Folder...', action: 'open-folder' }, { label: 'Save', action: 'save', shortcut: 'Ctrl+S' }, { separator: true }, { label: 'Exit', action: 'exit' }] },
    { name: 'Terminal', items: [{ label: 'New Terminal', action: 'new-terminal' }] },
    { name: 'Help', items: [{ label: 'About WhizCode', action: 'about' }] }
  ];

  return (
    <div className="app-container">
      <TitleBar
        menus={menus}
        activeMenu={activeMenu}
        toggleMenu={(m) => setActiveMenu(prev => prev === m ? null : m)}
        handleMenuHover={(m) => activeMenu && setActiveMenu(m)}
        handleMenuAction={handleMenuAction}
      />

      <div className="main-content">
        <ActivityBar
          isChatOpen={isChatOpen}
          setIsChatOpen={setIsChatOpen}
          isSidebarOpen={isSidebarOpen}
          setIsSidebarOpen={setIsSidebarOpen}
        />

        {isSidebarOpen && (
          <>
            <aside className="sidebar" style={{ width: `${sidebarWidth}px` }}>
              <div className="sidebar-header">
                <span>EXPLORER</span>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="1"></circle><circle cx="19" cy="12" r="1"></circle><circle cx="5" cy="12" r="1"></circle></svg>
              </div>
              <div className="sidebar-section-header" onClick={() => { }}>
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="9 18 15 12 9 6"></polyline></svg>
                <strong>{workspacePath ? workspacePath.split(/[/\\]/).pop()?.toUpperCase() : 'WHIZCODE'}</strong>
              </div>
              <div className="chat-history">
                {workspacePath ? <FileTree path={workspacePath} onFileOpen={handleFileOpen} /> : <div className="empty-state">No folder opened.</div>}
              </div>
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
            handleMenuAction={handleMenuAction}
          />

          {isTerminalOpen && (
            <div style={{ height: `${terminalHeight}px`, display: 'flex', flexDirection: 'column', borderTop: '1px solid var(--vscode-bg-secondary)' }}>
              <div style={{ height: '4px', cursor: 'row-resize', backgroundColor: 'var(--vscode-hover)' }} onMouseDown={handleTerminalResize} />
              <div className="tabs"><div className="tab active">Terminal</div></div>
              <div style={{ flex: 1, background: '#1e1e1e', padding: 8 }}><TerminalPane /></div>
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
          handleKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); } }}
          getToolIcon={getToolIcon}
          messagesEndRef={messagesEndRef}
          handlePermissionResponse={handlePermissionResponse}
          handleStop={handleStop}
          settingsProps={{
            isSettingsOpen, setIsSettingsOpen,
            plannerProvider, setPlannerProvider, plannerModel, setPlannerModel,
            executorProvider, setExecutorProvider, executorModel, setExecutorModel,
            ollamaModels, ollamaChecking, ollamaError, refreshOllamaModels, openaiKey, setOpenaiKey, geminiKey, setGeminiKey
          }}
        />
      </div>
    </div>
  )
}

export default App
