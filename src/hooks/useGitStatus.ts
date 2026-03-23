import { useEffect } from 'react'
import { git } from '../lib/tauri-api'

export function useGitStatus(
  workspacePath: string | null,
  refreshKey: number,
  setGitStatus: (status: any) => void
) {
  useEffect(() => {
    const fetchGitStatus = async () => {
      if (!workspacePath) return
      try {
        const res = await git.getStatus(workspacePath)
        setGitStatus(res)
      } catch (error: any) {
        if (error?.type === 'cancelation' || error?.msg?.includes('canceled')) {
          return
        }
        console.error('Error fetching git status:', error)
      }
    }
    fetchGitStatus()
  }, [workspacePath, refreshKey, setGitStatus])
}
