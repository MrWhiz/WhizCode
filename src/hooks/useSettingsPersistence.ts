import { useEffect } from 'react'
import type { AIProvider } from '../types'

export function useSettingsPersistence(
  modelProvider: AIProvider,
  model: string,
  openaiKey: string,
  geminiKey: string,
  bedrockRegion: string,
  bedrockAccessKey: string,
  bedrockSecretKey: string,
  azureLoginUrl: string,
  azureEmbeddingUrl: string,
  azureCompletionUrl: string,
  azureUsername: string,
  azurePassword: string,
  isAutopilotMode: boolean,
  contextLength: number,
  sidebarWidth: number,
  isChatOpen: boolean,
  chatWidth: number
) {
  useEffect(() => {
    localStorage.setItem('modelProvider', modelProvider)
    localStorage.setItem('model', model)
    localStorage.setItem('openaiKey', openaiKey)
    localStorage.setItem('geminiKey', geminiKey)
    localStorage.setItem('bedrockRegion', bedrockRegion)
    localStorage.setItem('bedrockAccessKey', bedrockAccessKey)
    localStorage.setItem('bedrockSecretKey', bedrockSecretKey)
    localStorage.setItem('azureLoginUrl', azureLoginUrl)
    localStorage.setItem('azureEmbeddingUrl', azureEmbeddingUrl)
    localStorage.setItem('azureCompletionUrl', azureCompletionUrl)
    localStorage.setItem('azureUsername', azureUsername)
    localStorage.setItem('azurePassword', azurePassword)
    localStorage.setItem('isAutopilotMode', String(isAutopilotMode))
    localStorage.setItem('contextLength', String(contextLength))
    localStorage.setItem('sidebarWidth', String(sidebarWidth))
    localStorage.setItem('isChatOpen', String(isChatOpen))
    localStorage.setItem('chatWidth', String(chatWidth))
  }, [
    modelProvider, model, openaiKey, geminiKey, bedrockRegion, bedrockAccessKey, bedrockSecretKey,
    isAutopilotMode, contextLength, azureLoginUrl, azureEmbeddingUrl, azureCompletionUrl, azureUsername, azurePassword,
    sidebarWidth, isChatOpen, chatWidth
  ])
}
