import { useState, useEffect, useCallback } from 'react'
import type { FileEntry } from '../../types'

interface ContextMenu {
    x: number
    y: number
    entry: FileEntry
}

const FileTreeItem = ({ entry, level = 0, onFileOpen, refreshKey, onContextMenu, collapseAll = false, fileFilter = '', fileErrors = {} }: { 
    entry: FileEntry, 
    level?: number, 
    onFileOpen: (path: string, name: string) => void, 
    refreshKey: number,
    onContextMenu: (e: React.MouseEvent, entry: FileEntry) => void,
    collapseAll?: boolean,
    fileFilter?: string,
    fileErrors?: Record<string, number>
}) => {
    const [expanded, setExpanded] = useState(false)
    const [children, setChildren] = useState<FileEntry[]>([])

    const fetchChildren = useCallback(async () => {
        const ipc = (window as any).ipcRenderer;
        if (ipc && entry.isDirectory) {
            const res = await ipc.invoke('fs:readDirectory', entry.path);
            setChildren(res);
        }
    }, [entry.path, entry.isDirectory])

    // Handle collapse all
    useEffect(() => {
        if (collapseAll) {
            setExpanded(false);
        }
    }, [collapseAll])

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

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault()
        e.stopPropagation()
        onContextMenu(e, entry)
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
                onClick={handleClick}
                onContextMenu={handleContextMenu}>
                {entry.isDirectory ? (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill={expanded ? '#dcb67a' : '#c09553'} stroke="none">
                        <path d="M2 6a2 2 0 012-2h5l2 2h9a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                    </svg>
                ) : (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={getFileIcon(entry.name)} strokeWidth="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                    </svg>
                )}
                {entry.isDirectory && (
                    <svg className="explorer-icon explorer-chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                        style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', marginLeft: '2px' }}>
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                )}
                <span className="explorer-item-name" style={fileErrors[entry.path] && fileErrors[entry.path] > 0 ? { color: '#f48771' } : {}}>
                    {entry.name}
                </span>
                {!entry.isDirectory && fileErrors[entry.path] && fileErrors[entry.path] > 0 && (
                    <span style={{ 
                        marginLeft: '6px', 
                        padding: '2px 6px', 
                        backgroundColor: '#f48771', 
                        color: '#fff', 
                        borderRadius: '3px', 
                        fontSize: '10px',
                        fontWeight: 'bold'
                    }}>
                        {fileErrors[entry.path]}
                    </span>
                )}
            </div>
            {expanded && entry.isDirectory && (
                <div className="explorer-children">
                    {children
                        .filter(child => !fileFilter || child.name.toLowerCase().includes(fileFilter.toLowerCase()))
                        .map((child) => (
                        <FileTreeItem key={child.path} entry={child} level={level + 1} onFileOpen={onFileOpen} refreshKey={refreshKey} onContextMenu={onContextMenu} collapseAll={collapseAll} fileFilter={fileFilter} fileErrors={fileErrors} />
                    ))}
                </div>
            )}
        </div>
    )
}

export const FileTree = ({ 
    path, 
    onFileOpen, 
    onFileDeleted, 
    onFileRenamed,
    refreshKey: externalRefreshKey = 0,
    collapseAll = false,
    fileFilter = '',
    fileErrors = {}
}: { 
    path: string, 
    onFileOpen: (path: string, name: string) => void,
    onFileDeleted?: (deletedPath: string) => void,
    onFileRenamed?: (oldPath: string, newPath: string) => void,
    refreshKey?: number,
    collapseAll?: boolean,
    fileFilter?: string,
    fileErrors?: Record<string, number>
}) => {
    const [files, setFiles] = useState<FileEntry[]>([])
    const [refreshKey, setRefreshKey] = useState(0)
    const [contextMenu, setContextMenu] = useState<ContextMenu | null>(null)
    const [newItemDialog, setNewItemDialog] = useState<{ type: 'file' | 'folder', parentPath: string } | null>(null)
    const [newItemName, setNewItemName] = useState('')

    const fetchFiles = useCallback(async () => {
        const ipc = (window as any).ipcRenderer;
        if (ipc) {
            const res = await ipc.invoke('fs:readDirectory', path);
            setFiles(res);
        }
    }, [path])

    useEffect(() => {
        fetchFiles()
    }, [fetchFiles, externalRefreshKey])

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

    // Close context menu when clicking elsewhere
    useEffect(() => {
        const handleClick = () => setContextMenu(null)
        document.addEventListener('click', handleClick)
        return () => document.removeEventListener('click', handleClick)
    }, [])

    const handleContextMenu = (e: React.MouseEvent, entry: FileEntry) => {
        setContextMenu({
            x: e.clientX,
            y: e.clientY,
            entry
        })
    }

    const handleMenuAction = async (action: string) => {
        if (!contextMenu) return
        const ipc = (window as any).ipcRenderer
        if (!ipc) return

        const { entry } = contextMenu
        setContextMenu(null)

        try {
            switch (action) {
                case 'open':
                    if (!entry.isDirectory) {
                        onFileOpen(entry.path, entry.name)
                    }
                    break

                case 'newFile':
                    setNewItemDialog({ 
                        type: 'file', 
                        parentPath: entry.isDirectory ? entry.path : entry.path.substring(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')))
                    })
                    break

                case 'newFolder':
                    setNewItemDialog({ 
                        type: 'folder', 
                        parentPath: entry.isDirectory ? entry.path : entry.path.substring(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')))
                    })
                    break

                case 'rename':
                    const newName = prompt('Enter new name:', entry.name)
                    if (newName && newName !== entry.name && newName.trim()) {
                        const parentPath = entry.path.substring(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')))
                        const newPath = parentPath + (parentPath.endsWith('/') || parentPath.endsWith('\\') ? '' : '/') + newName.trim()
                        await ipc.invoke('fs:rename', entry.path, newPath)
                        onFileRenamed?.(entry.path, newPath)
                        setRefreshKey(prev => prev + 1)
                    }
                    break

                case 'delete':
                    if (confirm(`Are you sure you want to delete "${entry.name}"?`)) {
                        await ipc.invoke('fs:delete', entry.path)
                        onFileDeleted?.(entry.path)
                        setRefreshKey(prev => prev + 1)
                    }
                    break

                case 'copy':
                    await navigator.clipboard.writeText(entry.path)
                    break

                case 'copyPath':
                    await navigator.clipboard.writeText(entry.path)
                    break

                case 'copyRelativePath':
                    const relativePath = entry.path.replace(path, '').replace(/^[/\\]/, '')
                    await navigator.clipboard.writeText(relativePath)
                    break

                case 'revealInExplorer':
                    await ipc.invoke('fs:revealInExplorer', entry.path)
                    break

                case 'openInTerminal':
                    const terminalPath = entry.isDirectory ? entry.path : entry.path.substring(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')))
                    await ipc.invoke('terminal:openAt', terminalPath)
                    break
            }
        } catch (error) {
            console.error('Context menu action failed:', error)
            alert('Operation failed: ' + (error as Error).message)
        }
    }

    const handleCreateItem = async () => {
        if (!newItemDialog || !newItemName.trim()) return
        
        const ipc = (window as any).ipcRenderer
        if (!ipc) return

        try {
            const separator = newItemDialog.parentPath.includes('\\') ? '\\' : '/'
            const fullPath = newItemDialog.parentPath + separator + newItemName.trim()
            
            if (newItemDialog.type === 'file') {
                await ipc.invoke('fs:createFile', fullPath)
                // Automatically open newly created files
                onFileOpen(fullPath, newItemName.trim())
            } else {
                await ipc.invoke('fs:createDirectory', fullPath)
            }
            
            setRefreshKey(prev => prev + 1)
            setNewItemDialog(null)
            setNewItemName('')
        } catch (error) {
            console.error('Create item failed:', error)
            alert('Failed to create item: ' + (error as Error).message)
        }
    }

    const handleRootContextMenu = (e: React.MouseEvent) => {
        // Only handle if the target is the explorer-tree itself, not a child element
        if (e.target === e.currentTarget) {
            e.preventDefault()
            setContextMenu({
                x: e.clientX,
                y: e.clientY,
                entry: { path: path, name: '', isDirectory: true } as FileEntry
            })
        }
    }

    return (
        <div className="explorer-tree" onContextMenu={handleRootContextMenu} style={{ minHeight: '100%', position: 'relative' }}>
            {files
                .filter(file => !fileFilter || file.name.toLowerCase().includes(fileFilter.toLowerCase()))
                .map((file) => (
                <FileTreeItem 
                    key={file.path} 
                    entry={file} 
                    level={0} 
                    onFileOpen={onFileOpen} 
                    refreshKey={refreshKey} 
                    onContextMenu={handleContextMenu}
                    collapseAll={collapseAll}
                    fileFilter={fileFilter}
                    fileErrors={fileErrors}
                />
            ))}
            
            {/* Add some empty space at the bottom to make right-clicking easier */}
            <div 
                className="explorer-empty-space"
                onContextMenu={(e) => {
                    e.preventDefault()
                    setContextMenu({
                        x: e.clientX,
                        y: e.clientY,
                        entry: { path: path, name: '', isDirectory: true } as FileEntry
                    })
                }} 
            />
            
            {/* Context Menu */}
            {contextMenu && (
                <div 
                    className="context-menu"
                    style={{ 
                        position: 'fixed', 
                        left: contextMenu.x, 
                        top: contextMenu.y,
                        zIndex: 1000
                    }}
                    onClick={(e) => e.stopPropagation()}
                >
                    {!contextMenu.entry.isDirectory && contextMenu.entry.name && (
                        <>
                            <div className="context-menu-item" onClick={() => handleMenuAction('open')}>
                                <span className="context-menu-icon">📄</span>
                                Open
                            </div>
                            <div className="context-menu-separator"></div>
                        </>
                    )}
                    
                    <div className="context-menu-item" onClick={() => handleMenuAction('newFile')}>
                        <span className="context-menu-icon">📄</span>
                        New File
                    </div>
                    <div className="context-menu-item" onClick={() => handleMenuAction('newFolder')}>
                        <span className="context-menu-icon">📁</span>
                        New Folder
                    </div>
                    
                    {contextMenu.entry.name && (
                        <>
                            <div className="context-menu-separator"></div>
                            
                            <div className="context-menu-item" onClick={() => handleMenuAction('rename')}>
                                <span className="context-menu-icon">✏️</span>
                                Rename
                            </div>
                            <div className="context-menu-item" onClick={() => handleMenuAction('delete')}>
                                <span className="context-menu-icon">🗑️</span>
                                Delete
                            </div>
                            
                            <div className="context-menu-separator"></div>
                            
                            <div className="context-menu-item" onClick={() => handleMenuAction('copyPath')}>
                                <span className="context-menu-icon">📋</span>
                                Copy Path
                            </div>
                            <div className="context-menu-item" onClick={() => handleMenuAction('copyRelativePath')}>
                                <span className="context-menu-icon">📋</span>
                                Copy Relative Path
                            </div>
                            
                            <div className="context-menu-separator"></div>
                            
                            <div className="context-menu-item" onClick={() => handleMenuAction('revealInExplorer')}>
                                <span className="context-menu-icon">🔍</span>
                                Reveal in Explorer
                            </div>
                            <div className="context-menu-item" onClick={() => handleMenuAction('openInTerminal')}>
                                <span className="context-menu-icon">⚡</span>
                                Open in Terminal
                            </div>
                        </>
                    )}
                </div>
            )}

            {/* New Item Dialog */}
            {newItemDialog && (
                <div className="modal-overlay">
                    <div className="modal-dialog">
                        <h3>Create New {newItemDialog.type === 'file' ? 'File' : 'Folder'}</h3>
                        <input
                            type="text"
                            value={newItemName}
                            onChange={(e) => setNewItemName(e.target.value)}
                            placeholder={`Enter ${newItemDialog.type} name...`}
                            autoFocus
                            onKeyDown={(e) => {
                                if (e.key === 'Enter') handleCreateItem()
                                if (e.key === 'Escape') {
                                    setNewItemDialog(null)
                                    setNewItemName('')
                                }
                            }}
                        />
                        <div className="modal-buttons">
                            <button onClick={handleCreateItem} disabled={!newItemName.trim()}>
                                Create
                            </button>
                            <button onClick={() => {
                                setNewItemDialog(null)
                                setNewItemName('')
                            }}>
                                Cancel
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    )
}
