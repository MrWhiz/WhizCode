import { useState, useEffect } from 'react'
import type { FileEntry } from '../../types'

const FileTreeItem = ({ entry, level = 0, onFileOpen }: { entry: FileEntry, level?: number, onFileOpen: (path: string, name: string) => void }) => {
    const [expanded, setExpanded] = useState(false)
    const [children, setChildren] = useState<FileEntry[]>([])

    const handleClick = async () => {
        if (entry.isDirectory) {
            if (!expanded && children.length === 0) {
                const ipc = (window as any).ipcRenderer;
                if (ipc) {
                    const res = await ipc.invoke('fs:readDirectory', entry.path);
                    setChildren(res);
                }
            }
            setExpanded(!expanded)
        } else {
            onFileOpen(entry.path, entry.name);
        }
    }

    return (
        <div style={{ marginLeft: 0 }}>
            <div
                className="history-item"
                style={{ paddingLeft: `${15 + level * 12}px` }}
                onClick={handleClick}>
                {entry.isDirectory ? (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ marginRight: 6, opacity: 0.8, transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.1s' }}>
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                ) : (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#519aba" strokeWidth="2" style={{ marginRight: 6, opacity: 0.8 }}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>
                )}
                {entry.name}
            </div>
            {expanded && entry.isDirectory && (
                <div>
                    {children.map((child, i) => (
                        <FileTreeItem key={i} entry={child} level={level + 1} onFileOpen={onFileOpen} />
                    ))}
                </div>
            )}
        </div>
    )
}

export const FileTree = ({ path, onFileOpen }: { path: string, onFileOpen: (path: string, name: string) => void }) => {
    const [files, setFiles] = useState<FileEntry[]>([])

    useEffect(() => {
        const fetchFiles = async () => {
            const ipc = (window as any).ipcRenderer;
            if (ipc) {
                const res = await ipc.invoke('fs:readDirectory', path);
                setFiles(res);
            }
        }
        fetchFiles()
    }, [path])

    return (
        <div style={{ paddingBottom: '10px' }}>
            {files.map((file, i) => (
                <FileTreeItem key={i} entry={file} level={0} onFileOpen={onFileOpen} />
            ))}
        </div>
    )
}
