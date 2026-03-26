import { useEffect, useRef } from 'react'
import type { AgentStep } from '../types'
import { agent, events, fs, workspace } from '../lib/tauri-api'

function isMeaningfulString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0
}

function isRichLogReplacement(previousLogs?: string[], nextLogs?: string[]): boolean {
  if (!previousLogs || previousLogs.length === 0 || !nextLogs || nextLogs.length === 0) {
    return false
  }

  if (nextLogs.length !== 1) {
    return false
  }

  const [onlyLog] = nextLogs
  return previousLogs.length > 1 && /^Status:\s+/m.test(onlyLog) && /\nLogs:\n/m.test(onlyLog)
}

function mergeAgentStep(previous: AgentStep, incoming: AgentStep): AgentStep {
  const merged: AgentStep = {
    ...previous,
    ...incoming,
  }

  merged.summary = isMeaningfulString(incoming.summary) ? incoming.summary : previous.summary
  merged.result = isMeaningfulString(incoming.result) ? incoming.result : previous.result
  merged.requestId = isMeaningfulString(incoming.requestId) ? incoming.requestId : previous.requestId
  merged.persona = isMeaningfulString(incoming.persona) ? incoming.persona : previous.persona
  merged.data = incoming.data ?? previous.data

  if (incoming.logs && incoming.logs.length > 0) {
    merged.logs = isRichLogReplacement(previous.logs, incoming.logs) ? previous.logs : incoming.logs
  } else {
    merged.logs = previous.logs
  }

  return merged
}

function appendStreamChunk(current: string, chunk: string): string {
  if (!chunk) return current

  const normalizedChunk = chunk.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  if (!current) return normalizedChunk

  const searchWindow = Math.min(current.length, normalizedChunk.length)
  for (let overlap = searchWindow; overlap > 0; overlap--) {
    if (current.endsWith(normalizedChunk.slice(0, overlap))) {
      const candidate = normalizedChunk.slice(overlap)
      return candidate ? current + candidate : current
    }
  }

  if (current.endsWith(normalizedChunk)) {
    return current
  }

  return current + normalizedChunk
}

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
    const streamFlushTimerRef: { current: ReturnType<typeof setTimeout> | null } = { current: null }

    const setupListeners = async () => {
      try {
        unlistenStep = await agent.events.onAgentStep((step: AgentStep) => {
          setAgentSteps(prev => {
            // Always update by requestId first (most reliable)
            if ((step as any).requestId) {
              const existingIdx = prev.findIndex(s => (s as any).requestId === (step as any).requestId)
              if (existingIdx >= 0) {
                const newSteps = [...prev]
                newSteps[existingIdx] = mergeAgentStep(newSteps[existingIdx], step)
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
              newSteps[existingIdx] = mergeAgentStep(newSteps[existingIdx], step)
              return newSteps
            }
            return [...prev, step]
          })
        })

        unlistenStream = await agent.events.onAgentStream(({ token }: { token: string }) => {
          streamingContentRef.current = appendStreamChunk(streamingContentRef.current, token)

          // Render the first token immediately so the UI feels responsive,
          // then keep coalescing subsequent tokens to avoid excess rerenders.
          if (!streamFlushTimerRef.current) {
            setLiveStreamingContent(streamingContentRef.current)
          }

          if (!streamFlushTimerRef.current) {
            streamFlushTimerRef.current = setTimeout(() => {
              streamFlushTimerRef.current = null
              setLiveStreamingContent(streamingContentRef.current)
            }, 100)
          }
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
      if (streamFlushTimerRef.current) {
        clearTimeout(streamFlushTimerRef.current)
        streamFlushTimerRef.current = null
        setLiveStreamingContent(streamingContentRef.current)
      }
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
        unlistenFileChanged = await events.onFileChanged(({ path, kind, old_path, content }) => {
          const refreshDelay = kind === 'modify' ? 300 : 50

          if (refreshTimeoutRef.current) {
            clearTimeout(refreshTimeoutRef.current)
          }
          refreshTimeoutRef.current = setTimeout(() => {
            setRefreshKey(prev => prev + 1)
          }, refreshDelay)

          if (kind === 'rename' && old_path) {
            setOpenFiles(prev => prev.map(file => {
              if (file.path === old_path) {
                const newName = path.split(/[/\\]/).pop() || file.name
                return { ...file, path, name: newName }
              }
              if (file.path.startsWith(old_path + '/')
                || file.path.startsWith(old_path + '\\')) {
                return { ...file, path: path + file.path.slice(old_path.length) }
              }
              return file
            }))
            return
          }

          if (kind === 'delete') {
            setOpenFiles(prev => prev.filter(file =>
              file.path !== path
              && !file.path.startsWith(path + '/')
              && !file.path.startsWith(path + '\\')
            ))
            return
          }

          setOpenFiles(prev => {
            const fileExists = prev.some(f => f.path === path)
            if (!fileExists) {
              return prev
            }

            if (content !== undefined) {
              return prev.map(f => f.path === path ? { ...f, content } : f)
            }

            fs.readFile(path).then(newContent => {
              setOpenFiles(current =>
                current.map(f => f.path === path ? { ...f, content: newContent } : f)
              )
            }).catch(err => console.error('Failed to reload changed file:', path, err))

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
