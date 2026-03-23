import { useEffect } from 'react'
import { workspace, fs } from '../lib/tauri-api'
import { setupWindowStatePersistence, setupUIStatePersistence, setupWorkspaceStatePersistence, restoreWindowState, clearAppState } from '../lib/appState'

export function useWorkspaceInit(
  savedState: any,
  setWorkspacePath: (path: string | null) => void,
  setRefreshKey: (key: number | ((prev: number) => number)) => void,
  sidebarWidth: number,
  chatWidth: number,
  isChatOpen: boolean,
  activeView: string | null,
  workspacePath: string | null,
  activeFileId: string | null
) {
  // Setup window state restoration and persistence
  useEffect(() => {
    const initializeWindowState = async () => {
      try {
        await restoreWindowState(savedState)
      } catch (error) {
        console.error('Failed to restore window state:', error)
      }
    }

    initializeWindowState()
    const cleanup = setupWindowStatePersistence()
    return cleanup
  }, [savedState])

  // Restore saved workspace on app startup
  useEffect(() => {
    const restoreSavedWorkspace = async () => {
      if (savedState.workspacePath) {
        try {
          console.log('[APP] Attempting to restore workspace:', savedState.workspacePath)
          await workspace.setWorkspace(savedState.workspacePath)
          console.log('[APP] Successfully restored workspace:', savedState.workspacePath)
          
          const wsInfo = await workspace.getWorkspace()
          if (!wsInfo) {
            throw new Error('Workspace was not properly set')
          }
          console.log('[APP] Verified workspace is set:', wsInfo.path)
          
          setWorkspacePath(savedState.workspacePath)
          
          await new Promise(resolve => setTimeout(resolve, 200))
          setRefreshKey(prev => prev + 1)
        } catch (error) {
          console.error('[APP] Failed to restore saved workspace:', savedState.workspacePath, error)
          clearAppState()
        }
      }
    }

    restoreSavedWorkspace()
  }, [savedState, setWorkspacePath, setRefreshKey])

  // Persist UI layout state whenever it changes
  useEffect(() => {
    setupUIStatePersistence(
      sidebarWidth,
      0,
      chatWidth,
      false,
      isChatOpen,
      activeView
    )
  }, [sidebarWidth, chatWidth, isChatOpen, activeView])

  // Persist workspace state whenever it changes
  useEffect(() => {
    setupWorkspaceStatePersistence(workspacePath, activeFileId)
  }, [workspacePath, activeFileId])

  // Start filesystem watcher when workspace changes
  useEffect(() => {
    if (!workspacePath) return
    fs.watchDirectory(workspacePath).catch(() => {})
  }, [workspacePath])
}
