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
  azureSessionToken: string,
  azureTokenExpiresAt: number,
  setAzureSessionToken: (token: string) => void,
  setAzureTokenExpiresAt: (expiresAt: number) => void,
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

  const computeAzureTokenStatus = useCallback(() => {
    const expiresAt = Number(azureTokenExpiresAt) || 0
    const token = azureSessionToken.trim()
    const now = Date.now()
    const hasToken = Boolean(token) && expiresAt > now

    if (!hasToken && (token || expiresAt)) {
      setAzureSessionToken('')
      setAzureTokenExpiresAt(0)
    }

    setAzureTokenStatus({
      hasToken,
      expires: hasToken ? expiresAt : undefined,
      timeLeft: hasToken ? Math.max(0, Math.ceil((expiresAt - now) / 3_600_000)) : undefined,
    })
  }, [azureSessionToken, azureTokenExpiresAt, setAzureTokenStatus])

  useEffect(() => {
    if (modelProvider === 'azure-gateway') {
      computeAzureTokenStatus()
    }
  }, [modelProvider, computeAzureTokenStatus])

  useEffect(() => {
    computeAzureTokenStatus()
  }, [computeAzureTokenStatus])

  useEffect(() => {
    if (modelProvider !== 'azure-gateway') {
      return
    }

    const timer = window.setInterval(() => {
      computeAzureTokenStatus()
    }, 60_000)

    return () => window.clearInterval(timer)
  }, [modelProvider, computeAzureTokenStatus])

  const handleGenerateAzureToken = useCallback(async () => {
    try {
      const result = await azure.generateToken({
        loginUrl: azureLoginUrl,
        username: azureUsername,
        password: azurePassword
      })
      if (result.success) {
        const expiresAt = result.expiresAt || (Date.now() + 24 * 60 * 60 * 1000)
        setAzureSessionToken(result.token || '')
        setAzureTokenExpiresAt(result.token ? expiresAt : 0)
        setAzureTokenStatus({
          hasToken: Boolean(result.token),
          expires: result.token ? expiresAt : undefined,
          timeLeft: result.token ? Math.max(0, Math.ceil((expiresAt - Date.now()) / 3_600_000)) : undefined,
        })
        if (!result.token) {
          alert('Token generation succeeded, but no session token was returned by the login endpoint.')
        }
      } else {
        alert(`Failed to generate token: ${result.error}`)
      }
    } catch (e: any) {
      alert(`Error: ${e.message}`)
    }
  }, [azureLoginUrl, azureUsername, azurePassword, setAzureSessionToken, setAzureTokenExpiresAt, setAzureTokenStatus])

  return {
    refreshOllamaModels,
    handleGenerateAzureToken,
    computeAzureTokenStatus,
  }
}
