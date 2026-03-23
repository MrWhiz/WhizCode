import { useState, useRef, useCallback } from 'react'
import type { Message, AgentStep, OpenFileProps, AIProvider } from '../types'
import { loadAppState } from '../lib/appState'

export function useAppState() {
  const savedState = loadAppState()

  // Chat state
  const [input, setInput] = useState('')
  const [messages, setMessages] = useState<Message[]>([
    { role: 'assistant', content: 'Hello! I\'m your WhizCode agent. Open a folder to get started.' }
  ])
  const [selectedImages, setSelectedImages] = useState<string[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [agentSteps, setAgentSteps] = useState<AgentStep[]>([])
  const [agentError, setAgentError] = useState<string | null>(null)
  const [liveStreamingContent, setLiveStreamingContent] = useState('')
  const [askUserPrompt, setAskUserPrompt] = useState<{ question: string; requestId: string } | null>(null)

  // Workspace state
  const [workspacePath, setWorkspacePath] = useState<string | null>(null)
  const [activeFileId, setActiveFileId] = useState<string | null>(savedState.activeFileId)
  const [openFiles, setOpenFiles] = useState<OpenFileProps[]>([])
  const [activeView, setActiveView] = useState<'explorer' | 'search' | 'source-control' | 'preview' | null>(savedState.activeView as any)
  const [sidebarWidth, setSidebarWidth] = useState(savedState.sidebarWidth)
  const [isChatOpen, setIsChatOpen] = useState(savedState.isChatOpen)
  const [chatWidth, setChatWidth] = useState(savedState.chatWidth)

  // Explorer state
  const [refreshKey, setRefreshKey] = useState(0)
  const [collapseAll, setCollapseAll] = useState(false)
  const [showFileFilter, setShowFileFilter] = useState(false)
  const [fileFilter, setFileFilter] = useState('')
  const [newFileDialog, setNewFileDialog] = useState<{ parentPath: string } | null>(null)
  const [gitStatus, setGitStatus] = useState<{ branch: string, changes: { file: string, status: string }[] } | null>(null)
  const [newFolderDialog, setNewFolderDialog] = useState<{ parentPath: string } | null>(null)
  const [newItemName, setNewItemName] = useState('')
  const [fileErrors, setFileErrors] = useState<Record<string, number>>({})

  // Model settings
  const [modelProvider, setModelProvider] = useState<AIProvider>(() => (localStorage.getItem('modelProvider') as AIProvider) || 'ollama')
  const [model, setModel] = useState(() => {
    const savedModel = localStorage.getItem('model');
    if (savedModel) return savedModel;
    const savedProvider = localStorage.getItem('modelProvider') as AIProvider;
    switch (savedProvider) {
      case 'openai': return 'gpt-4o';
      case 'gemini': return 'gemini-1.5-pro';
      case 'bedrock': return 'anthropic.claude-3-5-sonnet-20241022-v2:0';
      case 'azure-gateway': return 'gpt-4o';
      default: return 'qwen3:latest';
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
  const [isAboutOpen, setIsAboutOpen] = useState(false)

  // Azure Gateway settings
  const [azureLoginUrl, setAzureLoginUrl] = useState(() => localStorage.getItem('azureLoginUrl') || '')
  const [azureEmbeddingUrl, setAzureEmbeddingUrl] = useState(() => localStorage.getItem('azureEmbeddingUrl') || '')
  const [azureCompletionUrl, setAzureCompletionUrl] = useState(() => localStorage.getItem('azureCompletionUrl') || '')
  const [azureUsername, setAzureUsername] = useState(() => localStorage.getItem('azureUsername') || '')
  const [azurePassword, setAzurePassword] = useState(() => localStorage.getItem('azurePassword') || '')
  const [azureTokenStatus, setAzureTokenStatus] = useState<{ hasToken: boolean; timeLeft?: number; expires?: number }>({ hasToken: false })

  // Autopilot mode
  const [isAutopilotMode, setIsAutopilotMode] = useState(() =>
    localStorage.getItem('isAutopilotMode') === 'true'
  )

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const streamingContentRef = useRef('')
  const activeMenuRef = useRef<string | null>(null)

  return {
    // Chat
    input, setInput,
    messages, setMessages,
    selectedImages, setSelectedImages,
    isLoading, setIsLoading,
    agentSteps, setAgentSteps,
    agentError, setAgentError,
    liveStreamingContent, setLiveStreamingContent,
    askUserPrompt, setAskUserPrompt,
    // Workspace
    workspacePath, setWorkspacePath,
    activeFileId, setActiveFileId,
    openFiles, setOpenFiles,
    activeView, setActiveView,
    sidebarWidth, setSidebarWidth,
    isChatOpen, setIsChatOpen,
    chatWidth, setChatWidth,
    // Explorer
    refreshKey, setRefreshKey,
    collapseAll, setCollapseAll,
    showFileFilter, setShowFileFilter,
    fileFilter, setFileFilter,
    newFileDialog, setNewFileDialog,
    gitStatus, setGitStatus,
    newFolderDialog, setNewFolderDialog,
    newItemName, setNewItemName,
    fileErrors, setFileErrors,
    // Model settings
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
    // Azure
    azureLoginUrl, setAzureLoginUrl,
    azureEmbeddingUrl, setAzureEmbeddingUrl,
    azureCompletionUrl, setAzureCompletionUrl,
    azureUsername, setAzureUsername,
    azurePassword, setAzurePassword,
    azureTokenStatus, setAzureTokenStatus,
    // Autopilot
    isAutopilotMode, setIsAutopilotMode,
    // Refs
    messagesEndRef,
    streamingContentRef,
    activeMenuRef,
    STREAMING_MSG_ID: '__streaming__',
  }
}
