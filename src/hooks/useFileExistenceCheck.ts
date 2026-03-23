import { useEffect } from 'react'
import type { OpenFileProps } from '../types'
import { fs } from '../lib/tauri-api'

export function useFileExistenceCheck(
  openFiles: OpenFileProps[],
  handleFileDeleted: (path: string) => void
) {
  useEffect(() => {
    if (openFiles.length === 0) return

    const checkFiles = async () => {
      const filesToCheck = [...openFiles]
      for (const file of filesToCheck) {
        try {
          const exists = await fs.checkFileExists(file.path)
          if (!exists) {
            handleFileDeleted(file.path)
          }
        } catch (error) {
          console.error('Error checking file existence:', error)
        }
      }
    }

    const interval = setInterval(checkFiles, 5000)
    return () => clearInterval(interval)
  }, [openFiles, handleFileDeleted])
}
