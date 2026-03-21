/**
 * Hook for using Tauri IPC in React components
 * Provides a familiar interface similar to Electron's ipcRenderer
 */

import { useEffect, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'

interface IpcRenderer {
  invoke: (channel: string, ...args: any[]) => Promise<any>
  send: (channel: string, ...args: any[]) => void
  on: (channel: string, listener: (event: any, ...args: any[]) => void) => void
  off: (channel: string, listener: (event: any, ...args: any[]) => void) => void
}

/**
 * Hook that provides Tauri IPC interface compatible with Electron code
 * Allows gradual migration of components
 */
export function useTauriIpc(): IpcRenderer {
  const listenersRef = useRef<Map<string, { listener: Function; unlisten: UnlistenFn }[]>>(new Map())

  // Cleanup listeners on unmount
  useEffect(() => {
    return () => {
      listenersRef.current.forEach((listeners) => {
        listeners.forEach(({ unlisten }) => {
          unlisten()
        })
      })
      listenersRef.current.clear()
    }
  }, [])

  const ipcInvoke = useCallback(async (channel: string, ...args: any[]) => {
    try {
      // Convert channel name from Electron format to Tauri format
      const tauriChannel = convertChannelName(channel)
      
      // Handle different argument patterns
      if (args.length === 0) {
        return await invoke(tauriChannel)
      } else if (args.length === 1 && typeof args[0] === 'object') {
        // Single object argument
        return await invoke(tauriChannel, args[0])
      } else {
        // Multiple arguments - combine into object
        const params = convertArgsToObject(channel, args)
        return await invoke(tauriChannel, params)
      }
    } catch (error) {
      console.error(`IPC invoke error on channel ${channel}:`, error)
      throw error
    }
  }, [])

  const ipcSend = useCallback((channel: string, ...args: any[]) => {
    // Fire-and-forget - just invoke without awaiting
    ipcInvoke(channel, ...args).catch(err => {
      console.error(`IPC send error on channel ${channel}:`, err)
    })
  }, [ipcInvoke])

  const ipcOn = useCallback((channel: string, listener: (event: any, ...args: any[]) => void) => {
    const tauriChannel = convertChannelName(channel)
    
    listen(tauriChannel, (event) => {
      // Call listener with event and payload
      listener(event, event.payload)
    }).then((unlisten) => {
      // Store unlisten function for cleanup
      if (!listenersRef.current.has(channel)) {
        listenersRef.current.set(channel, [])
      }
      listenersRef.current.get(channel)!.push({ listener, unlisten })
    }).catch(err => {
      console.error(`IPC on error for channel ${channel}:`, err)
    })
  }, [])

  const ipcOff = useCallback((channel: string, listener: (event: any, ...args: any[]) => void) => {
    const listeners = listenersRef.current.get(channel)
    if (!listeners) return

    const index = listeners.findIndex(l => l.listener === listener)
    if (index >= 0) {
      const { unlisten } = listeners[index]
      unlisten()
      listeners.splice(index, 1)
    }
  }, [])

  return {
    invoke: ipcInvoke,
    send: ipcSend,
    on: ipcOn,
    off: ipcOff,
  }
}

/**
 * Convert Electron IPC channel names to Tauri command names
 * Examples:
 *   'fs:readFile' → 'read_file'
 *   'terminal:create' → 'terminal_create'
 *   'agent:step' → 'agent_step'
 */
function convertChannelName(channel: string): string {
  // Remove colon and convert to snake_case
  return channel.replace(':', '_').replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '')
}

/**
 * Convert multiple arguments to object based on channel
 */
function convertArgsToObject(channel: string, args: any[]): Record<string, any> {
  // Map of channels to their parameter names
  const paramMaps: Record<string, string[]> = {
    'fs:readFile': ['path'],
    'fs:writeFile': ['path', 'content'],
    'fs:readDirectory': ['path'],
    'fs:createFile': ['path'],
    'fs:createDirectory': ['path'],
    'fs:delete': ['path'],
    'fs:rename': ['oldPath', 'newPath'],
    'terminal:keystroke': ['data', 'terminalId'],
    'terminal:resize': ['cols', 'rows', 'terminalId'],
    'terminal:create': ['id', 'type'],
    'terminal:close': ['id'],
    'diagnostics:check': ['filePath', 'workspacePath', 'content'],
    'git:status': ['workspacePath'],
    'search:files': ['pattern', 'includeGlob'],
    'search:fuzzyFind': ['query', 'maxResults'],
  }

  const paramNames = paramMaps[channel] || []
  const result: Record<string, any> = {}

  paramNames.forEach((name, index) => {
    if (index < args.length) {
      result[name] = args[index]
    }
  })

  return result
}

/**
 * Hook for listening to specific IPC events
 */
export function useTauriEvent<T = any>(
  channel: string,
  callback: (data: T) => void,
  dependencies: any[] = []
) {
  const ipc = useTauriIpc()

  useEffect(() => {
    ipc.on(channel, (_event, data) => {
      callback(data)
    })

    return () => {
      ipc.off(channel, (_event, data) => {
        callback(data)
      })
    }
  }, [channel, callback, ipc, ...dependencies])
}

/**
 * Hook for invoking IPC commands with loading state
 */
export function useTauriInvoke<T = any>(
  channel: string,
  args?: any[],
  dependencies: any[] = []
) {
  const ipc = useTauriIpc()
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<Error | null>(null)
  const [data, setData] = React.useState<T | null>(null)

  const invoke = useCallback(async (...invokeArgs: any[]) => {
    setLoading(true)
    setError(null)
    try {
      const result = await ipc.invoke(channel, ...(invokeArgs.length > 0 ? invokeArgs : args || []))
      setData(result)
      return result
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err))
      setError(error)
      throw error
    } finally {
      setLoading(false)
    }
  }, [ipc, channel, args])

  return { invoke, loading, error, data }
}

// Re-export React for the hook
import React from 'react'
