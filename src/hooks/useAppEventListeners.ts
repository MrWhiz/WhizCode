import { useEffect, useRef } from 'react'
import type { AgentStep } from '../types'
import { agent, events, fs, workspace } from '../lib/tauri-api'

export function useAppEventListeners(
  setAgentSteps: (steps: AgentStep[] | ((prev: AgentStep[]) => AgentStep[])) => void,
  setMessages: (msg: any[] | ((prev: any[]) => any[])) => void,
  setLiveStreamingContent: (content: string) => void,
  setIsLoading: (loading: boolean) => void,
  setAskUserPrompt: (prompt: any) => void,
  setRefreshKey: (key: number | ((prev: number) => number)) => void,
  setOpenFiles: (files: any[] | ((prev: any[]) => any[])) => void,
  setWorkspacePath: (path: string | null) => void,
  streamingContentRef: React.MutableRefObject<string>,
  STREAMING_MSG_ID: string
) {
  useEffect(() => {
    let unlistenStep: (() => void) | null = null
    let unlistenStream: (() => void) | null = null
    let unlistenError: (() => void) | null = null

    const setupListeners = async () => {
      try {
        unlistenStep = await agent.events.onAgentStep((step: AgentStep) => {
          setAgentSteps(prev => {
            // Always update by requestId first (most reliable)
            if ((step as any).requestId) {
              const existingIdx = prev.findIndex(s => (s as any).requestId === (step as any).requestId)
              if (existingIdx >= 0) {
                const newSteps = [...prev]
                newSteps[existingIdx] = step
                return newSteps
              }
              return [...prev, step]
            }

            // For steps without requestId, update by tool+iteration, regardless of summary change
            // This ensures failed tools update the existing step instead of creating a duplicate
            const existingIdx = prev.findIndex(s =>
              s.tool === step.tool &&
              s.iteration === step.iteration
            )

            if (existingIdx >= 0) {
              const newSteps = [...prev]
              newSteps[existingIdx] = step
              return newSteps
            }
            return [...prev, step]
          })
        })

        unlistenStream = await agent.events.onAgentStream(({ token }: { token: string }) => {
          streamingContentRef.current += token
          const currentContent = streamingContentRef.current
          setLiveStreamingContent(currentContent)
        })

        unlistenError = await (window as any).__TAURI_INVOKE__?.('listen', {
          event: 'agent:error',
          handler: (event: any) => {
            const errorData = event.payload
            const errorMsg = errorData?.error || 'Unknown agent error'
            console.error('[AGENT ERROR]', errorMsg)
            setIsLoading(false)
            setMessages(prev => [...prev, { role: 'assistant', content: `⚠️ Error: ${errorMsg}` }])
          }
        }).catch(() => {})
      } catch (error) {
        console.error('Failed to setup Tauri event listeners:', error)
      }
    }

    setupListeners()

    let unlistenAskUser: (() => void) | null = null
    agent.events.onAgentAskUser((data) => {
      setAskUserPrompt(data)
    }).then(fn => { unlistenAskUser = fn }).catch(() => {})

    return () => {
      if (unlistenStep) unlistenStep()
      if (unlistenStream) unlistenStream()
      if (unlistenError) unlistenError()
      if (unlistenAskUser) unlistenAskUser()
    }
  }, [setAgentSteps, setMessages, setLiveStreamingContent, setIsLoading, setAskUserPrompt, streamingContentRef, STREAMING_MSG_ID])

  // Workspace restored listener
  useEffect(() => {
    let unlistenWorkspaceRestored: (() => void) | null = null

    const setupWorkspaceListener = async () => {
      try {
        unlistenWorkspaceRestored = await workspace.events.onWorkspaceRestored((incomingWorkspacePath: string) => {
          // Check if ANY ignore flag is set (they have timestamps in the name)
          let shouldIgnore = false
          for (let i = 0; i < sessionStorage.length; i++) {
            const key = sessionStorage.key(i)
            if (key && key.startsWith('_ignoreWorkspaceRestored_')) {
              shouldIgnore = true
              break
            }
          }
          
          if (shouldIgnore) {
            // Skip this event - we're setting a new workspace
            return
          }
          
          // Update workspace
          setWorkspacePath(incomingWorkspacePath)
        })
      } catch (error) {
        console.error('Failed to setup workspace restored listener:', error)
      }
    }

    setupWorkspaceListener()

    return () => {
      if (unlistenWorkspaceRestored) unlistenWorkspaceRestored()
    }
  }, [setWorkspacePath])

  // File changed listener with debounce
  const refreshTimeoutRef = useRef<any>(null)

  useEffect(() => {
    let unlistenFileChanged: (() => void) | null = null

    const setupFileChangeListener = async () => {
      try {
        unlistenFileChanged = await events.onFileChanged(({ path, content }) => {
          // Debounce refreshKey increment to prevent flickering
          if (refreshTimeoutRef.current) {
            clearTimeout(refreshTimeoutRef.current)
          }
          refreshTimeoutRef.current = setTimeout(() => {
            setRefreshKey(prev => prev + 1)
          }, 1000)

          setOpenFiles(prev => {
            const fileExists = prev.some(f => f.path === path)
            if (fileExists) {
              if (content !== undefined) {
                return prev.map(f => f.path === path ? { ...f, content } : f)
              } else {
                fs.readFile(path).then(newContent => {
                  setOpenFiles(current => 
                    current.map(f => f.path === path ? { ...f, content: newContent } : f)
                  )
                }).catch(err => console.error('Failed to reload changed file:', path, err))
                return prev
              }
            }
            return prev
          })
        })
      } catch (error) {
        console.error('Failed to setup file changed listener:', error)
      }
    }

    setupFileChangeListener()

    return () => {
      if (unlistenFileChanged) unlistenFileChanged()
      if (refreshTimeoutRef.current) clearTimeout(refreshTimeoutRef.current)
    }
  }, [setRefreshKey, setOpenFiles])
}
