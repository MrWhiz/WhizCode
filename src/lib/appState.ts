/**
 * App State Persistence
 * Handles saving and loading app state including window size, workspace, and UI layout
 */

import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi'

export interface AppState {
  // Window state
  windowWidth: number
  windowHeight: number
  windowX: number
  windowY: number
  isFullscreen: boolean
  isMaximized: boolean

  // Workspace state
  workspacePath: string | null
  activeFileId: string | null

  // UI Layout state
  sidebarWidth: number
  terminalHeight: number
  chatWidth: number
  isTerminalOpen: boolean
  isChatOpen: boolean
  activeView: 'explorer' | 'search' | 'source-control' | 'brain-health' | 'specs' | null

  // Timestamp for validation
  timestamp: number
}

const STORAGE_KEY = 'whizcode_app_state'

/**
 * Get default app state
 */
export function getDefaultAppState(): AppState {
  return {
    windowWidth: 1200,
    windowHeight: 800,
    windowX: 100,
    windowY: 100,
    isFullscreen: false,
    isMaximized: false,
    workspacePath: null,
    activeFileId: null,
    sidebarWidth: 260,
    terminalHeight: 250,
    chatWidth: 400,
    isTerminalOpen: true,
    isChatOpen: true,
    activeView: 'explorer',
    timestamp: Date.now(),
  }
}

/**
 * Save app state to localStorage
 */
export function saveAppState(state: Partial<AppState>): void {
  try {
    const existing = loadAppState()
    const merged = { ...existing, ...state, timestamp: Date.now() }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(merged))
  } catch (error) {
    console.error('Failed to save app state:', error)
  }
}

/**
 * Load app state from localStorage
 */
export function loadAppState(): AppState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (!stored) {
      return getDefaultAppState()
    }

    const parsed = JSON.parse(stored) as AppState
    // Validate that we have the required fields
    if (!parsed.timestamp) {
      return getDefaultAppState()
    }

    return parsed
  } catch (error) {
    console.error('Failed to load app state:', error)
    return getDefaultAppState()
  }
}

/**
 * Clear all saved app state
 */
export function clearAppState(): void {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch (error) {
    console.error('Failed to clear app state:', error)
  }
}

/**
 * Restore window state from saved state
 */
export async function restoreWindowState(state: AppState): Promise<void> {
  try {
    const appWindow = WebviewWindow.getCurrent()
    if (!appWindow) return
    
    // Only restore if we have valid dimensions (not minimized)
    const hasValidDimensions = state.windowWidth > 0 && state.windowHeight > 0
    
    if (!hasValidDimensions) {
      // Use defaults if dimensions are invalid
      await appWindow.setSize(new PhysicalSize(1200, 800))
      await appWindow.setPosition(new PhysicalPosition(100, 100))
      return
    }

    // Restore window position and size
    if (state.windowX !== undefined && state.windowY !== undefined) {
      await appWindow.setPosition(new PhysicalPosition(state.windowX, state.windowY))
    }

    if (state.windowWidth !== undefined && state.windowHeight !== undefined) {
      await appWindow.setSize(new PhysicalSize(state.windowWidth, state.windowHeight))
    }

    // Restore fullscreen state (but not maximized, as that can cause issues)
    if (state.isFullscreen) {
      await appWindow.setFullscreen(true)
    }
  } catch (error) {
    console.error('Failed to restore window state:', error)
  }
}

/**
 * Get current window state
 */
export async function getCurrentWindowState(): Promise<Partial<AppState>> {
  try {
    const appWindow = WebviewWindow.getCurrent()
    if (!appWindow) return {}
    
    const position = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    const isFullscreen = await appWindow.isFullscreen()
    const isMaximized = await appWindow.isMaximized()

    return {
      windowX: position.x,
      windowY: position.y,
      windowWidth: size.width,
      windowHeight: size.height,
      isFullscreen,
      isMaximized,
    }
  } catch (error) {
    console.error('Failed to get window state:', error)
    return {}
  }
}

/**
 * Setup window state persistence
 * Saves window state periodically and on close
 */
export function setupWindowStatePersistence(): () => void {
  let saveInterval: ReturnType<typeof setInterval> | null = null

  // Save window state every 5 seconds
  saveInterval = setInterval(async () => {
    const windowState = await getCurrentWindowState()
    saveAppState(windowState)
  }, 5000)

  // Cleanup function
  return () => {
    if (saveInterval) {
      clearInterval(saveInterval)
    }
  }
}

/**
 * Setup UI state persistence
 * Saves UI layout state whenever it changes
 */
export function setupUIStatePersistence(
  sidebarWidth: number,
  terminalHeight: number,
  chatWidth: number,
  isTerminalOpen: boolean,
  isChatOpen: boolean,
  activeView: string | null
): void {
  saveAppState({
    sidebarWidth,
    terminalHeight,
    chatWidth,
    isTerminalOpen,
    isChatOpen,
    activeView: activeView as any,
  })
}

/**
 * Setup workspace state persistence
 */
export function setupWorkspaceStatePersistence(
  workspacePath: string | null,
  activeFileId: string | null
): void {
  saveAppState({
    workspacePath,
    activeFileId,
  })
}
