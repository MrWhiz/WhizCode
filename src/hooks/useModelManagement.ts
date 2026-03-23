import { useEffect, useCallback } from 'react'
import { ollama, azure } from '../lib/tauri-api'

export function useModelManagement(
  modelProvider: string,
  model: string,
  setModel: (model: string) => void,
  ollamaModels: string[],
  setOllamaModels: (models: string[]) => void,
  ollamaError: string | null,
  setOllamaError: (error: string | null) => void,
  ollamaChecking: boolean,
  setOllamaChecking: (checking: boolean) => void,
  isSettingsOpen: boolean,
  azureLoginUrl: string,
  azureUsername: string,
  azurePassword: string,
  setAzureTokenStatus: (status: any) => void
) {
  const refreshOllamaModels = useCallback(async () => {
    setOllamaChecking(true)
    setOllamaError(null)

    try {
      console.log('[FRONTEND] Checking Ollama health...')
      const healthCheck = await ollama.healthCheck()
      if (!healthCheck.healthy) {
        setOllamaError(`Ollama health check failed: ${healthCheck.error || 'Unknown error'}`)
        setOllamaModels([])
        return
      }

      console.log('[FRONTEND] Ollama is healthy, fetching models...')
      const res = await ollama.getModels()
      console.log('[FRONTEND] Received models:', res)
      setOllamaModels(res)
      setOllamaError(null)
      if (res.length > 0 && !res.includes(model)) {
        const preferred = res.find(m => m.startsWith('qwen3')) || res[0]
        setModel(preferred)
      }
    } catch (error: any) {
      console.error('[FRONTEND] Ollama connection error:', error)
      setOllamaError("Could not connect to Ollama: " + (error.message || 'Unknown error'))
      setOllamaModels([])
    } finally {
      setOllamaChecking(false)
    }
  }, [model, setModel, setOllamaModels, setOllamaError, setOllamaChecking])

  // Test Ollama connection on startup
  useEffect(() => {
    if (modelProvider === 'ollama') {
      refreshOllamaModels()
    }
  }, [modelProvider, refreshOllamaModels])

  // Refresh Ollama models when settings are opened
  useEffect(() => {
    if (isSettingsOpen && modelProvider === 'ollama') {
      refreshOllamaModels()
    }
  }, [isSettingsOpen, modelProvider, refreshOllamaModels])

  // Check Azure token
  const checkAzureToken = useCallback(async () => {
    try {
      const status = await azure.getTokenStatus()
      setAzureTokenStatus(status)
    } catch (error) {
      console.error('Error checking Azure token status:', error)
    }
  }, [setAzureTokenStatus])

  useEffect(() => {
    if (modelProvider === 'azure-gateway') {
      checkAzureToken()
    }
  }, [modelProvider, checkAzureToken])

  const handleGenerateAzureToken = useCallback(async () => {
    try {
      const result = await azure.generateToken({
        loginUrl: azureLoginUrl,
        username: azureUsername,
        password: azurePassword
      })
      if (result.success) {
        checkAzureToken()
      } else {
        alert(`Failed to generate token: ${result.error}`)
      }
    } catch (e: any) {
      alert(`Error: ${e.message}`)
    }
  }, [azureLoginUrl, azureUsername, azurePassword, checkAzureToken])

  return {
    refreshOllamaModels,
    handleGenerateAzureToken,
    checkAzureToken,
  }
}
