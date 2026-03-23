import { useEffect } from 'react'

export function useKeyboardShortcuts(
  handleFileSave: () => void,
  activeView: string | null,
  workspacePath: string | null,
  setNewFileDialog: (dialog: { parentPath: string } | null) => void,
  setNewFolderDialog: (dialog: { parentPath: string } | null) => void,
  setRefreshKey: (key: number | ((prev: number) => number)) => void,
  setShowFileFilter: (show: boolean) => void
) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+S to save
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault()
        handleFileSave()
      }
      // Ctrl+N to create new file (when explorer is focused)
      if (e.ctrlKey && e.key === 'n' && activeView === 'explorer' && workspacePath) {
        e.preventDefault()
        setNewFileDialog({ parentPath: workspacePath })
      }
      // Ctrl+Shift+N to create new folder (when explorer is focused)
      if (e.ctrlKey && e.shiftKey && e.key === 'N' && activeView === 'explorer' && workspacePath) {
        e.preventDefault()
        setNewFolderDialog({ parentPath: workspacePath })
      }
      // F5 to refresh explorer
      if (e.key === 'F5' && activeView === 'explorer') {
        e.preventDefault()
        setRefreshKey(prev => prev + 1)
      }
      // Ctrl+P for file filter
      if (e.ctrlKey && e.key === 'p' && activeView === 'explorer') {
        e.preventDefault()
        setShowFileFilter(true)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleFileSave, activeView, workspacePath, setNewFileDialog, setNewFolderDialog, setRefreshKey, setShowFileFilter])
}
