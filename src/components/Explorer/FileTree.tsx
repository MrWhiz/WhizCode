import { useState, useEffect, useCallback } from 'react'
import type { FileEntry } from '../../types'

const FileTreeItem = ({ entry, level = 0, onFileOpen, refreshKey }: { entry: FileEntry, level?: number, onFileOpen: (path: string, name: string) => void, refreshKey: number }) => {
    const [expanded, setExpanded] = useState(false)
    const [children, setChildren] = useState<FileEntry[]>([])

    const fetchChildren = useCallback(async () => {
        const ipc = (window as any).ipcRenderer;
        if (ipc && entry.isDirectory) {
            const res = await ipc.invoke('fs:readDirectory', entry.path);
            setChildren(res);
        }
    }, [entry.path, entry.isDirectory])

    // Re-fetch children when refreshKey changes (file system changed)
    useEffect(() => {
        if (expanded && entry.isDirectory) {
            fetchChildren();
        }
    }, [refreshKey, expanded, entry.isDirectory, fetchChildren])

    const handleClick = async () => {
        if (entry.isDirectory) {
            if (!expanded) {
                await fetchChildren();
            }
            setExpanded(!expanded)
        } else {
            onFileOpen(entry.path, entry.name);
        }
    }

    const getFileIcon = (name: string) => {
        const ext = name.split('.').pop()?.toLowerCase();
        const iconColors: Record<string, string> = {
            ts: '#3178c6', tsx: '#3178c6',
            js: '#f7df1e', jsx: '#f7df1e',
            json: '#cb8742', css: '#563d7c',
            html: '#e34c26', md: '#519aba',
            py: '#3776ab', go: '#00add8',
            rs: '#dea584', yaml: '#cb4a32',
            yml: '#cb4a32', toml: '#9c4121',
            sh: '#89e051', bat: '#c1f12e',
            env: '#ecd53f', lock: '#555',
            gitignore: '#f05033',
        };
        return iconColors[ext || ''] || '#519aba';
    }

    return (
        <div>
            <div
                className={`explorer-item${expanded ? ' expanded' : ''}`}
                style={{ paddingLeft: `${12 + level * 16}px` }}
                onClick={handleClick}>
                {entry.isDirectory ? (
                    <svg className="explorer-icon explorer-chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                        style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}>
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                ) : (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={getFileIcon(entry.name)} strokeWidth="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                    </svg>
                )}
                {entry.isDirectory && (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill={expanded ? '#dcb67a' : '#c09553'} stroke="none">
                        <path d="M2 6a2 2 0 012-2h5l2 2h9a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                    </svg>
                )}
                <span className="explorer-item-name">{entry.name}</span>
            </div>
            {expanded && entry.isDirectory && (
                <div className="explorer-children">
                    {children.map((child) => (
                        <FileTreeItem key={child.path} entry={child} level={level + 1} onFileOpen={onFileOpen} refreshKey={refreshKey} />
                    ))}
                </div>
            )}
        </div>
    )
}

export const FileTree = ({ path, onFileOpen }: { path: string, onFileOpen: (path: string, name: string) => void }) => {
    const [files, setFiles] = useState<FileEntry[]>([])
    const [refreshKey, setRefreshKey] = useState(0)

    const fetchFiles = useCallback(async () => {
        const ipc = (window as any).ipcRenderer;
        if (ipc) {
            const res = await ipc.invoke('fs:readDirectory', path);
            setFiles(res);
        }
    }, [path])

    useEffect(() => {
        fetchFiles()
    }, [fetchFiles])

    // Listen for file system changes from main process
    useEffect(() => {
        const ipc = (window as any).ipcRenderer;
        if (!ipc) return;

        const handler = () => {
            // Refresh the tree when files/dirs are added or removed
            fetchFiles();
            setRefreshKey(prev => prev + 1);
        };

        ipc.on('fs:directoryChanged', handler);
        return () => {
            ipc.off('fs:directoryChanged', handler);
        };
    }, [fetchFiles])

    return (
        <div className="explorer-tree">
            {files.map((file) => (
                <FileTreeItem key={file.path} entry={file} level={0} onFileOpen={onFileOpen} refreshKey={refreshKey} />
            ))}
        </div>
    )
}
