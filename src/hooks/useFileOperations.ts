import { useCallback } from 'react'
import type { OpenFileProps } from '../types'
import { fs, diagnostics } from '../lib/tauri-api'

export function useFileOperations(
  openFiles: OpenFileProps[],
  setOpenFiles: (files: OpenFileProps[]) => void,
  activeFileId: string | null,
  setActiveFileId: (id: string | null) => void,
  workspacePath: string | null
) {
  const handleFileOpen = useCallback(async (path: string, name: string) => {
    const existingFile = openFiles.find(f => f.path === path)
    if (existingFile) {
      setActiveFileId(path)
      return
    }
    try {
      const content = await fs.readFile(path)
      if (content !== null) {
        setOpenFiles([...openFiles, { path, name, content }])
        setActiveFileId(path)
      }
    } catch (error) {
      console.error('Error opening file:', error)
    }
  }, [openFiles, setOpenFiles, setActiveFileId])

  const handleFileSave = useCallback(async () => {
    const activeFile = openFiles.find(f => f.path === activeFileId)
    if (!activeFile) return
    try {
      await fs.writeFile(activeFile.path, activeFile.content)
    } catch (error) {
      console.error('Error saving file:', error)
    }
  }, [openFiles, activeFileId])

  const handleFileClose = useCallback((path: string, e: React.MouseEvent) => {
    e.stopPropagation()
    setOpenFiles(openFiles.filter(f => f.path !== path))
    if (activeFileId === path) {
      const remaining = openFiles.filter(f => f.path !== path)
      setActiveFileId(remaining.length > 0 ? remaining[remaining.length - 1].path : null)
    }
  }, [openFiles, activeFileId, setOpenFiles, setActiveFileId])

  const handleFileDeleted = useCallback((deletedPath: string) => {
    setOpenFiles(openFiles.filter(f => !f.path.startsWith(deletedPath)))
    if (activeFileId && (activeFileId === deletedPath || activeFileId.startsWith(deletedPath + '/'))) {
      const remaining = openFiles.filter(f => !f.path.startsWith(deletedPath))
      setActiveFileId(remaining.length > 0 ? remaining[remaining.length - 1].path : null)
    }
  }, [openFiles, activeFileId, setOpenFiles, setActiveFileId])

  const handleFileRenamed = useCallback((oldPath: string, newPath: string) => {
    setOpenFiles(openFiles.map(f => {
      if (f.path === oldPath) {
        const newName = newPath.split(/[/\\]/).pop() || f.name
        return { ...f, path: newPath, name: newName }
      } else if (f.path.startsWith(oldPath + '/')) {
        const relativePath = f.path.substring(oldPath.length)
        return { ...f, path: newPath + relativePath }
      }
      return f
    }))

    if (activeFileId === oldPath) {
      setActiveFileId(newPath)
    } else if (activeFileId && activeFileId.startsWith(oldPath + '/')) {
      const relativePath = activeFileId.substring(oldPath.length)
      setActiveFileId(newPath + relativePath)
    }
  }, [openFiles, activeFileId, setOpenFiles, setActiveFileId])

  const handleContentChange = useCallback((newContent: string | undefined) => {
    if (newContent !== undefined) {
      setOpenFiles(openFiles.map(f => f.path === activeFileId ? { ...f, content: newContent } : f))
    }
  }, [openFiles, activeFileId, setOpenFiles])

  const getLanguage = useCallback((fileName: string) => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    switch (ext) {
      case 'ts': case 'tsx': return 'typescript'
      case 'js': case 'jsx': return 'javascript'
      case 'json': return 'json'
      case 'html': return 'html'
      case 'css': return 'css'
      case 'md': return 'markdown'
      case 'py': return 'python'
      default: return 'plaintext'
    }
  }, [])

  const normalizePath = useCallback((p: string) => p.replace(/\\/g, '/').toLowerCase().replace(/^[a-z]:/, '').replace(/^\/+/, ''), [])

  const checkFileErrors = useCallback(async (filePath: string, content: string): Promise<number> => {
    if (!workspacePath) return 0
    const normFilePath = normalizePath(filePath)
    try {
      const diagnosticsPromise = diagnostics.check(normFilePath, content, getLanguage(filePath))
        .catch(error => {
          if (error?.type === 'cancelation' || error?.msg?.includes('canceled') || error?.message?.includes('canceled')) {
            return []
          }
          throw error
        })
      const timeoutPromise = new Promise<any[]>((_, reject) =>
        setTimeout(() => reject(new Error('Diagnostics timeout')), 3000)
      )
      const result = await Promise.race([diagnosticsPromise, timeoutPromise])
      return Array.isArray(result) ? result.length : 0
    } catch (error: any) {
      if (error?.message === 'Diagnostics timeout') return 0
      console.error('Error checking file diagnostics:', error)
      return 0
    }
  }, [workspacePath, normalizePath, getLanguage])

  return {
    handleFileOpen,
    handleFileSave,
    handleFileClose,
    handleFileDeleted,
    handleFileRenamed,
    handleContentChange,
    getLanguage,
    normalizePath,
    checkFileErrors,
  }
}
