/**
 * Tauri API Wrapper
 * Provides type-safe access to Tauri commands
 */

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// File System Types
export interface FileEntry {
  name: string
  path: string
  isDirectory: boolean
  size?: number
}

export interface SearchResult {
  file: string
  line: number
  content: string
}

export interface FuzzyResult {
  path: string
  score: number
}

// Terminal Types
export interface ShellInfo {
  name: string
  path: string
}

// System Types
export interface SystemInfo {
  platform: string
  arch: string
  cpu_count: number
  memory_gb: number
}

// Workspace Types
export interface WorkspaceInfo {
  path: string
}

// Error handling
export class TauriError extends Error {
  code: string
  details?: string

  constructor(code: string, message: string, details?: string) {
    super(message)
    this.code = code
    this.details = details
    this.name = 'TauriError'
  }
}

// File System Commands
export const fs = {
  async readFile(path: string): Promise<string> {
    try {
      return await invoke('read_file', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async writeFile(path: string, content: string): Promise<void> {
    try {
      return await invoke('write_file', { path, content })
    } catch (error) {
      throw handleError(error)
    }
  },

  async readDirectory(path: string): Promise<FileEntry[]> {
    try {
      return await invoke('read_directory', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async readDirectoryRecursive(
    path: string,
    maxFiles?: number
  ): Promise<FileEntry[]> {
    try {
      return await invoke('read_directory_recursive', { path, maxFiles })
    } catch (error) {
      throw handleError(error)
    }
  },

  async createFile(path: string): Promise<void> {
    try {
      return await invoke('create_file', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async createDirectory(path: string): Promise<void> {
    try {
      return await invoke('create_directory', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async deleteFile(path: string): Promise<void> {
    try {
      return await invoke('delete_file', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async deleteDirectory(path: string): Promise<void> {
    try {
      return await invoke('delete_directory', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async renameFile(oldPath: string, newPath: string): Promise<void> {
    try {
      return await invoke('rename_file', { oldPath, newPath })
    } catch (error) {
      throw handleError(error)
    }
  },

  async checkFileExists(path: string): Promise<boolean> {
    try {
      return await invoke('check_file_exists', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async revealInExplorer(path: string): Promise<void> {
    try {
      return await invoke('reveal_in_folder', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async openTerminal(path: string): Promise<void> {
    try {
      return await invoke('open_terminal_at', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async watchDirectory(path: string): Promise<void> {
    try {
      return await invoke('watch_directory', { path })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Vector search
export const vectorSearch = {
  async indexWorkspaceFull(workspacePath: string, embeddingModel?: string): Promise<void> {
    try {
      return await invoke('vector_index_workspace_full', {
        workspacePath: workspacePath,
        embeddingModel: embeddingModel ?? 'nomic-embed-text',
      })
    } catch (error) {
      // Non-fatal - indexing failure shouldn't block the user
      console.warn('[VectorSearch] Indexing failed:', error)
    }
  },

  async getFileTree(workspacePath: string, maxFiles?: number): Promise<string> {
    try {
      return await invoke('vector_get_file_tree', {
        workspacePath: workspacePath,
        maxFiles: maxFiles ?? 300,
      })
    } catch (error) {
      return ''
    }
  },

  async semanticSearch(query: string, limit?: number): Promise<any[]> {
    try {
      return await invoke('vector_semantic_search', {
        query: { query, limit: limit ?? 10 },
      })
    } catch (error) {
      return []
    }
  },

  async getIndexStats(): Promise<any> {
    try {
      return await invoke('vector_get_index_stats')
    } catch (error) {
      return null
    }
  },

  async updateFile(path: string): Promise<void> {
    try {
      return await invoke('vector_update_file', { path })
    } catch (error) {
      console.warn('[VectorSearch] updateFile failed:', error)
    }
  },

  async clearIndex(): Promise<void> {
    try {
      return await invoke('vector_clear_index')
    } catch (error) {
      console.warn('[VectorSearch] clearIndex failed:', error)
    }
  },
}

// Search Commands
export const search = {
  async searchFiles(
    pattern: string,
    includeGlob?: string
  ): Promise<SearchResult[]> {
    try {
      return await invoke('search_files', { pattern, includeGlob })
    } catch (error) {
      throw handleError(error)
    }
  },

  async fuzzyFindFile(
    query: string,
    maxResults?: number
  ): Promise<FuzzyResult[]> {
    try {
      return await invoke('fuzzy_find_file', { query, maxResults })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Terminal Commands
export const terminal = {
  async createTerminal(shellType: string, cwd?: string): Promise<string> {
    try {
      return await invoke('terminal_create', { 
        config: {
          shellType,
          cwd: cwd || null
        }
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async writeToTerminal(terminalId: string, data: string): Promise<void> {
    try {
      return await invoke('terminal_write', { terminalId, data })
    } catch (error) {
      throw handleError(error)
    }
  },

  async resizeTerminal(
    terminalId: string,
    cols: number,
    rows: number
  ): Promise<void> {
    try {
      return await invoke('terminal_resize', { terminalId, cols, rows })
    } catch (error) {
      throw handleError(error)
    }
  },

  async closeTerminal(terminalId: string): Promise<void> {
    try {
      return await invoke('terminal_close', { id: terminalId })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getAvailableShells(): Promise<ShellInfo[]> {
    try {
      return await invoke('terminal_get_available_shells')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getDefaultShell(): Promise<string> {
    try {
      return await invoke('terminal_get_default_shell')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// System Commands
export const system = {
  async getSystemInfo(): Promise<SystemInfo> {
    try {
      return await invoke('get_system_info')
    } catch (error) {
      throw handleError(error)
    }
  },

  async openExternal(url: string): Promise<void> {
    try {
      return await invoke('open_external', { url })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getWorkspacePath(): Promise<string | null> {
    try {
      const w: WorkspaceInfo | null = await invoke('get_workspace')
      return w ? w.path : null
    } catch (error) {
      console.warn('Failed to get workspace path:', error)
      return null
    }
  },
}

// Workspace Commands
export const workspace = {
  async setWorkspace(path: string): Promise<void> {
    try {
      return await invoke('set_workspace', { path })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getWorkspace(): Promise<WorkspaceInfo | null> {
    try {
      return await invoke('get_workspace')
    } catch (error) {
      throw handleError(error)
    }
  },

  events: {
    async onWorkspaceRestored(callback: (workspacePath: string) => void): Promise<() => void> {
      const unlisten = await listen('workspace:restored', (event) => {
        callback(event.payload as string)
      })
      return unlisten
    },
  },
}

// Event Listeners
export const events = {
  async onFileChanged(callback: (data: { path: string; content?: string }) => void): Promise<() => void> {
    const unlisten = await listen('file:changed', (event) => {
      callback(event.payload as { path: string; content?: string })
    })
    return unlisten
  },

  async onFileAdded(callback: (path: string) => void): Promise<() => void> {
    const unlisten = await listen('file:added', (event) => {
      callback(event.payload as string)
    })
    return unlisten
  },

  async onFileDeleted(callback: (path: string) => void): Promise<() => void> {
    const unlisten = await listen('file:deleted', (event) => {
      callback(event.payload as string)
    })
    return unlisten
  },

  async onTerminalData(
    terminalId: string,
    callback: (data: string) => void
  ): Promise<() => void> {
    const unlisten = await listen(`terminal:data:${terminalId}`, (event) => {
      callback(event.payload as string)
    })
    return unlisten
  },

  async onTerminalExit(
    terminalId: string,
    callback: (code: number) => void
  ): Promise<() => void> {
    const unlisten = await listen(`terminal:exit:${terminalId}`, (event) => {
      callback(event.payload as number)
    })
    return unlisten
  },
}

// Error handling helper
function handleError(error: unknown): TauriError {
  if (error instanceof TauriError) {
    return error
  }

  // Handle Tauri cancellation or specific result objects
  if (error && typeof error === 'object') {
    const err = error as any
    if (err.type === 'cancelation' || err.msg?.includes('canceled')) {
      const te = new TauriError('CANCELATION', err.msg || 'Operation canceled')
      ;(te as any).type = 'cancelation'
      ;(te as any).msg = err.msg
      return te
    }
    
    if (err.message) {
      return new TauriError('TAURI_ERROR', err.message, JSON.stringify(err))
    }
  }

  if (typeof error === 'string') {
    return new TauriError('UNKNOWN_ERROR', error)
  }

  if (error instanceof Error) {
    return new TauriError('ERROR', error.message)
  }

  return new TauriError('UNKNOWN_ERROR', 'An unknown error occurred')
}

// Diagnostics Commands
export const diagnostics = {
  async check(filePath: string, content: string, language: string): Promise<any[]> {
    try {
      const report = await invoke<any>('diagnostics_check_file', { filePath, content, language })
      return report.diagnostics || []
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Git Commands
export const git = {
  async getStatus(workspacePath: string): Promise<any> {
    try {
      return await invoke('git_status', { path: workspacePath })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Dialog Commands
export const dialog = {
  async openFolder(): Promise<{ canceled: boolean; filePaths: string[] }> {
    try {
      const result = await invoke<string | null>('dialog_open_folder')
      if (result) {
        return { canceled: false, filePaths: [result] }
      } else {
        return { canceled: true, filePaths: [] }
      }
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Agent Commands
export const agent = {
  async executeTask(options: any): Promise<any> {
    try {
      return await invoke('execute_agent_task', {
        task: options.task,
        model: options.model,
        workspacePath: options.workspacePath,
        activeFile: options.activeFile,
        config: options.config,
        isAutopilotMode: options.isAutopilotMode,
        images: options.images
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async executeLoop(options: any): Promise<any> {
    try {
      return await invoke('execute_agent_loop', {
        task: options.task,
        model: options.model,
        workspacePath: options.workspacePath,
        activeFile: options.activeFile,
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async executeLoopStreaming(options: any): Promise<any> {
    try {
      const wsPath = options.workspacePath || options.workspace_path || null
      const activeFile = options.activeFile || null
      // FIX #1: Send conversation history so the LLM has multi-turn memory
      const conversationHistory = (options.conversationHistory || []).map((m: any) => ({
        role: m.role === 'user' ? 'user' : 'assistant',
        content: typeof m.content === 'string' ? m.content : String(m.content ?? ''),
      }))
      console.log('[API] Invoking execute_agent_loop_streaming, workspacePath =', wsPath, 'history turns =', conversationHistory.length)
      return await invoke('execute_agent_loop_streaming', {
        task: options.task,
        model: options.model,
        workspacePath: wsPath,
        activeFile: activeFile,
        conversationHistory: conversationHistory,
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async stop(): Promise<void> {
    try {
      return await invoke('agent_stop')
    } catch (error) {
      throw handleError(error)
    }
  },

  async reset(): Promise<void> {
    try {
      return await invoke('agent_reset')
    } catch (error) {
      throw handleError(error)
    }
  },

  async sendPermissionResponse(approved: boolean, requestId?: string): Promise<void> {
    try {
      return await invoke('agent_permission_response', { approved, requestId })
    } catch (error) {
      throw handleError(error)
    }
  },

  events: {
    async onAgentStep(callback: (step: any) => void): Promise<() => void> {
      const unlisten = await listen('agent:step', (event) => {
        callback(event.payload as any)
      })
      return unlisten
    },

    async onAgentStream(callback: (data: { token: string }) => void): Promise<() => void> {
      const unlisten = await listen('agent:stream', (event) => {
        callback(event.payload as { token: string })
      })
      return unlisten
    },

    // FIX #9: mid-task ask_user clarification requests
    async onAgentAskUser(callback: (data: { question: string; requestId: string }) => void): Promise<() => void> {
      const unlisten = await listen('agent:step', (event) => {
        const step = event.payload as any
        if (step?.tool === 'ask_user' && step?.status === 'awaiting_permission') {
          callback({ question: step.summary, requestId: step.requestId || '' })
        }
      })
      return unlisten
    },
  }, // end events
} // end agent

// Ollama Commands
export const ollama = {
  async healthCheck(): Promise<{ healthy: boolean; error?: string }> {
    try {
      return await invoke('ollama_health_check')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getModels(): Promise<string[]> {
    try {
      return await invoke('ollama_get_models')
    } catch (error) {
      throw handleError(error)
    }
  },

  async pullModel(modelName: string): Promise<void> {
    try {
      return await invoke('ollama_pull_model', { modelName })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Azure Commands
export const azure = {
  async getTokenStatus(): Promise<{ hasToken: boolean; expiresIn?: number }> {
    try {
      return await invoke('azure_get_token_status')
    } catch (error) {
      throw handleError(error)
    }
  },

  async generateToken(options: { loginUrl: string; username: string; password: string }): Promise<{ success: boolean; error?: string }> {
    try {
      return await invoke('azure_generate_token', { 
        loginUrl: options.loginUrl, 
        username: options.username, 
        password: options.password 
      })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// AI/Learning Commands
export const ai = {
  async getLearningInsights(): Promise<any> {
    try {
      return await invoke('ai_get_learning_insights')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getLearningMetrics(): Promise<any> {
    try {
      return await invoke('ai_get_learning_metrics')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getCodeMetrics(workspacePath: string): Promise<any> {
    try {
      return await invoke('ai_get_code_metrics', { workspacePath })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getContextMemoryStats(): Promise<any> {
    try {
      return await invoke('ai_get_context_memory_stats')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Vector/Index Commands
export const vector = {
  async getIndexStats(): Promise<any> {
    try {
      return await invoke('vector_get_index_stats')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Cache Commands
export const cache = {
  async getStats(): Promise<any> {
    try {
      return await invoke('cache_get_stats')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Error Recovery Commands
export const errorRecovery = {
  async getStatistics(): Promise<any> {
    try {
      return await invoke('error_recovery_get_statistics')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// MCP Commands
export const mcp = {
  async getMarketplace(): Promise<any> {
    try {
      return await invoke('mcp_get_marketplace')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Specs Commands
export const specs = {
  async list(): Promise<any[]> {
    try {
      return await invoke('specs_list')
    } catch (error) {
      throw handleError(error)
    }
  },

  async get(slug: string): Promise<any> {
    try {
      return await invoke('specs_get', { slug })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Export all APIs
export const planner = {
  async createPlan(context: any): Promise<any> {
    try {
      return await invoke('create_plan', context)
    } catch (error) {
      throw handleError(error)
    }
  },
}

export const subAgents = {
  async listAll(): Promise<any[]> {
    try {
      return await invoke('list_sub_agents')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getConfig(agentName: string): Promise<any> {
    try {
      return await invoke('get_sub_agent_config', { agentName })
    } catch (error) {
      throw handleError(error)
    }
  },

  async invoke(agentName: string, taskDescription: string): Promise<string> {
    try {
      return await invoke('invoke_sub_agent', { agentName, taskDescription })
    } catch (error) {
      throw handleError(error)
    }
  },
}

export const learning = {
  async analyzePatterns(): Promise<any[]> {
    try {
      return await invoke('learning_analyze_patterns')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getRecommendations(taskType: string): Promise<string[]> {
    try {
      return await invoke('learning_get_recommendations', { taskType })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getMetrics(): Promise<any> {
    try {
      return await invoke('learning_get_metrics')
    } catch (error) {
      throw handleError(error)
    }
  },

  async recordInteraction(userRequest: string, agentResponse: string, toolsUsed: string[], success: boolean, durationMs: number): Promise<void> {
    try {
      return await invoke('learning_record_interaction', { userRequest, agentResponse, toolsUsed, success, durationMs })
    } catch (error) {
      throw handleError(error)
    }
  },
}

export const contextMemory = {
  async recordPattern(pattern: string, context: string, language: string, projectType: string): Promise<void> {
    try {
      return await invoke('context_memory_record_pattern', { pattern, context, language, projectType })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getPatterns(context: string, language?: string): Promise<any[]> {
    try {
      return await invoke('context_memory_get_patterns', { context, language })
    } catch (error) {
      throw handleError(error)
    }
  },

  async recordPreference(key: string, value: any, confidence: number): Promise<void> {
    try {
      return await invoke('context_memory_record_preference', { key, value, confidence })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getPreference(key: string): Promise<any> {
    try {
      return await invoke('context_memory_get_preference', { key })
    } catch (error) {
      throw handleError(error)
    }
  },

  async recordError(errorType: string, context: string, solution: string, success: boolean): Promise<void> {
    try {
      return await invoke('context_memory_record_error', { errorType, context, solution, success })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getSimilarErrors(errorType: string): Promise<any[]> {
    try {
      return await invoke('context_memory_get_similar_errors', { errorType })
    } catch (error) {
      throw handleError(error)
    }
  },

  async recordStrategy(taskType: string, strategy: string, tools: string[], duration: number): Promise<void> {
    try {
      return await invoke('context_memory_record_strategy', { taskType, strategy, tools, duration })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getBestStrategies(taskType: string): Promise<any[]> {
    try {
      return await invoke('context_memory_get_best_strategies', { taskType })
    } catch (error) {
      throw handleError(error)
    }
  },
}

export const hooks = {
  async listAll(): Promise<any[]> {
    try {
      return await invoke('hooks_list_all')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getEnabled(): Promise<any[]> {
    try {
      return await invoke('hooks_get_enabled')
    } catch (error) {
      throw handleError(error)
    }
  },

  async add(hook: any): Promise<void> {
    try {
      return await invoke('hooks_add', hook)
    } catch (error) {
      throw handleError(error)
    }
  },

  async remove(hookId: string): Promise<void> {
    try {
      return await invoke('hooks_remove', { hookId })
    } catch (error) {
      throw handleError(error)
    }
  },

  async update(hook: any): Promise<void> {
    try {
      return await invoke('hooks_update', hook)
    } catch (error) {
      throw handleError(error)
    }
  },

  async getForEvent(eventType: string): Promise<any[]> {
    try {
      return await invoke('hooks_get_for_event', { eventType })
    } catch (error) {
      throw handleError(error)
    }
  },

  async triggerFileEvent(eventType: string, filePath: string): Promise<any[]> {
    try {
      return await invoke('hooks_trigger_file_event', { eventType, filePath })
    } catch (error) {
      throw handleError(error)
    }
  },

  async triggerToolEvent(eventType: string, toolName: string): Promise<any[]> {
    try {
      return await invoke('hooks_trigger_tool_event', { eventType, toolName })
    } catch (error) {
      throw handleError(error)
    }
  },
}

export const codeIntelligence = {
  async analyzeWorkspace(workspacePath: string): Promise<any> {
    try {
      return await invoke('code_intelligence_analyze_workspace', { workspacePath })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getSymbolInfo(workspacePath: string, symbolName: string): Promise<any> {
    try {
      return await invoke('code_intelligence_get_symbol_info', { workspacePath, symbolName })
    } catch (error) {
      throw handleError(error)
    }
  },

  async findRelatedSymbols(workspacePath: string, symbolName: string): Promise<any[]> {
    try {
      return await invoke('code_intelligence_find_related_symbols', { workspacePath, symbolName })
    } catch (error) {
      throw handleError(error)
    }
  },

  async suggestRefactoring(workspacePath: string, filePath: string): Promise<string[]> {
    try {
      return await invoke('code_intelligence_suggest_refactoring', { workspacePath, filePath })
    } catch (error) {
      throw handleError(error)
    }
  },

  async getMetrics(workspacePath: string): Promise<any> {
    try {
      return await invoke('code_intelligence_get_metrics', { workspacePath })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Advanced Tools Commands
export const advancedTools = {
  async editFile(args: { path: string; startLine?: number; endLine?: number; content: string }): Promise<any> {
    try {
      return await invoke('execute_edit_file', {
        path: args.path,
        startLine: args.startLine,
        endLine: args.endLine,
        content: args.content,
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async gitOperation(args: { operation: string; path?: string; message?: string; branch?: string }): Promise<any> {
    try {
      return await invoke('execute_git_operation', {
        operation: args.operation,
        path: args.path,
        message: args.message,
        branch: args.branch,
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async npmOperation(args: { operation: string; package?: string; version?: string }): Promise<any> {
    try {
      return await invoke('execute_npm_operation', {
        operation: args.operation,
        package: args.package,
        version: args.version,
      })
    } catch (error) {
      throw handleError(error)
    }
  },

  async dockerOperation(args: { operation: string; container?: string; image?: string; args?: string[] }): Promise<any> {
    try {
      return await invoke('execute_docker_operation', {
        operation: args.operation,
        container: args.container,
        image: args.image,
        args: args.args,
      })
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Tool Cache Commands
export const toolCache = {
  async get(tool: string, args: any): Promise<string | null> {
    try {
      return await invoke('tool_cache_get', { tool, args })
    } catch (error) {
      throw handleError(error)
    }
  },

  async clear(): Promise<void> {
    try {
      return await invoke('tool_cache_clear')
    } catch (error) {
      throw handleError(error)
    }
  },

  async getStats(): Promise<any> {
    try {
      return await invoke('tool_cache_get_stats')
    } catch (error) {
      throw handleError(error)
    }
  },
}

// Custom Tools Commands
export const customTools = {
  async register(tool: any): Promise<void> {
    try {
      return await invoke('register_custom_tool', tool)
    } catch (error) {
      throw handleError(error)
    }
  },

  async unregister(name: string): Promise<void> {
    try {
      return await invoke('unregister_custom_tool', { name })
    } catch (error) {
      throw handleError(error)
    }
  },

  async list(): Promise<any[]> {
    try {
      return await invoke('list_custom_tools')
    } catch (error) {
      throw handleError(error)
    }
  },

  async execute(name: string, args: string[]): Promise<string> {
    try {
      return await invoke('execute_custom_tool', { name, args })
    } catch (error) {
      throw handleError(error)
    }
  },
}

export default {
  fs,
  search,
  terminal,
  system,
  workspace,
  events,
  diagnostics,
  git,
  dialog,
  agent,
  ollama,
  azure,
  ai,
  vector,
  cache,
  errorRecovery,
  mcp,
  specs,
  planner,
  subAgents,
  learning,
  contextMemory,
  hooks,
  codeIntelligence,
  advancedTools,
  toolCache,
  customTools,
}

// Planning Commands
export const planning = {
  async createExecutionPlan(userRequest: string, workspacePath?: string): Promise<any> {
    try {
      return await invoke('create_execution_plan', { 
        userRequest, 
        workspacePath 
      })
    } catch (error) {
      throw handleError(error)
    }
  },
}
