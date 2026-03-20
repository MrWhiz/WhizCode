import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import type { FileEntry } from '../../types'

interface ContextMenu {
    x: number
    y: number
    entry: FileEntry
}

const FileTreeItem = ({ 
    entry, 
    level = 0, 
    onFileOpen, 
    refreshKey, 
    onContextMenu, 
    collapseAll = false, 
    fileFilter = '', 
    fileErrors = {}, 
    gitStatus = null 
}: { 
    entry: FileEntry, 
    level?: number, 
    onFileOpen: (path: string, name: string) => void, 
    refreshKey: number,
    onContextMenu: (e: React.MouseEvent, entry: FileEntry) => void,
    collapseAll?: boolean,
    fileFilter?: string,
    fileErrors?: Record<string, number>,
    gitStatus?: { branch: string, changes: { file: string, status: string }[] } | null
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

    useEffect(() => {
        if (collapseAll) {
            setExpanded(false);
        }
    }, [collapseAll])

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

    const normalize = (p: string) => p.replace(/\\/g, '/').toLowerCase().replace(/^[a-z]:/, '').replace(/^\/+/, '').trim();
    const normEntryPath = normalize(entry.path);

    const errorPaths = Object.keys(fileErrors);
    const hasError = errorPaths.some(p => {
        const normP = normalize(p);
        return normP === normEntryPath || 
               normP.startsWith(normEntryPath + '/') || 
               (normP.endsWith(normEntryPath) && normP.length > normEntryPath.length && normP[normP.length - normEntryPath.length - 1] === '/');
    });
    
    const totalErrorCount = errorPaths
        .filter(p => {
            const normP = normalize(p);
            return normP === normEntryPath || normP.startsWith(normEntryPath + '/');
        })
        .reduce((sum, p) => sum + (fileErrors[p] || 0), 0);

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

    const getGitStatus = () => {
        if (!gitStatus || !gitStatus.changes) return null;
        
        const changes = gitStatus.changes;
        const entryPath = entry.path.replace(/\\/g, '/');
        const lowerEntryEdge = entryPath.toLowerCase();
        
        const match = changes.find(c => {
            const normGitPath = c.file.replace(/\\/g, '/').replace(/\/$/, '').trim();
            const lowerGit = normGitPath.toLowerCase();
            return lowerEntryEdge.endsWith('/' + lowerGit) || lowerEntryEdge === lowerGit;
        });
        
        if (match) return match.status;

        if (entry.isDirectory) {
            const lowerName = entry.name.toLowerCase();
            const hasChangesInside = changes.some(c => {
                const normGitPath = c.file.replace(/\\/g, '/').toLowerCase();
                return normGitPath.includes('/' + lowerName + '/') || normGitPath.startsWith(lowerName + '/');
            });
            if (hasChangesInside) return 'M'; 
        }
        
        return null;
    }

    const gStatus = getGitStatus();
    const statusColor = gStatus === 'M' ? '#e2c08d' : (gStatus === 'A' || gStatus === '??' ? '#73c991' : undefined);

    return (
        <div className="file-tree-item-container">
            <div
                className={`explorer-item${expanded ? ' expanded' : ''}`}
                style={{ paddingLeft: `${12 + level * 16}px` }}
                onClick={handleClick}
                onContextMenu={handleContextMenu}>
                {entry.isDirectory ? (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill={hasError ? '#ff3333' : (statusColor || (expanded ? '#dcb67a' : '#c09553'))} stroke="none">
                        <path d="M2 6a2 2 0 012-2h5l2 2h9a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                    </svg>
                ) : (
                    <svg className="explorer-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" strokeWidth="2"
                         stroke={hasError ? '#ff3333' : (statusColor || getFileIcon(entry.name))}>
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                    </svg>
                )}
                {entry.isDirectory && (
                    <svg className="explorer-icon explorer-chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" strokeWidth="2"
                        stroke={hasError ? '#ff3333' : (statusColor || "currentColor")}
                        style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', marginLeft: '2px' }}>
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                )}
                <span className="explorer-item-name" style={{ color: hasError ? '#f14c4c' : statusColor, opacity: statusColor ? 1 : 0.85 }}>
                    {entry.name}
                </span>
                
                <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    {gStatus && (
                        <span style={{ 
                            fontSize: '10px', 
                            color: statusColor, 
                            fontWeight: 'bold',
                            padding: '0 4px',
                            opacity: 0.8
                        }}>
                            {gStatus === '??' ? 'U' : gStatus}
                        </span>
                    )}
                    {totalErrorCount > 0 && (
                        <span style={{ 
                            padding: '1px 5px', 
                            backgroundColor: '#f14c4c', 
                            color: 'white', 
                            borderRadius: '10px', 
                            fontSize: '9px',
                            fontWeight: 'bold',
                            minWidth: '14px',
                            textAlign: 'center'
                        }}>
                            {totalErrorCount}
                        </span>
                    )}
                </div>
            </div>
            {expanded && entry.isDirectory && (
                <div className="explorer-children">
                    {children
                        .filter(child => !fileFilter || child.name.toLowerCase().includes(fileFilter.toLowerCase()))
                        .map((child) => (
                        <FileTreeItem 
                            key={child.path} 
                            entry={child} 
                            level={level + 1} 
                            onFileOpen={onFileOpen} 
                            refreshKey={refreshKey} 
                            onContextMenu={onContextMenu} 
                            collapseAll={collapseAll} 
                            fileFilter={fileFilter} 
                            fileErrors={fileErrors} 
                            gitStatus={gitStatus} 
                        />
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
    fileErrors = {},
    gitStatus = null
}: { 
    path: string, 
    onFileOpen: (path: string, name: string) => void,
    onFileDeleted?: (deletedPath: string) => void,
    onFileRenamed?: (oldPath: string, newPath: string) => void,
    refreshKey?: number,
    collapseAll?: boolean,
    fileFilter?: string,
    fileErrors?: Record<string, number>,
    gitStatus?: { branch: string, changes: { file: string, status: string }[] } | null
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

    useEffect(() => {
        const ipc = (window as any).ipcRenderer;
        if (!ipc) return;

        const handler = () => {
            fetchFiles();
            setRefreshKey(prev => prev + 1);
        };

        ipc.on('fs:directoryChanged', handler);
        return () => {
            ipc.off('fs:directoryChanged', handler);
        };
    }, [fetchFiles]);

    const handleContextMenu = (e: React.MouseEvent, entry: FileEntry) => {
        setContextMenu({
            x: e.clientX,
            y: e.clientY,
            entry
        })
    }

    const handleRootContextMenu = (e: React.MouseEvent) => {
        e.preventDefault()
        setContextMenu({
            x: e.clientX,
            y: e.clientY,
            entry: { name: '', path, isDirectory: true }
        })
    }

    const handleMenuAction = async (action: string) => {
        if (!contextMenu) return
        const ipc = (window as any).ipcRenderer
        if (!ipc) return

        const entry = contextMenu.entry
        setContextMenu(null)

        switch (action) {
            case 'newFile':
                setNewItemDialog({ type: 'file', parentPath: entry.isDirectory ? entry.path : path })
                break
            case 'newFolder':
                setNewItemDialog({ type: 'folder', parentPath: entry.isDirectory ? entry.path : path })
                break
            case 'rename':
                const newName = prompt('Enter new name:', entry.name)
                if (newName && newName !== entry.name) {
                    const success = await ipc.invoke('fs:rename', { oldPath: entry.path, newName })
                    if (success) {
                        const newPath = entry.path.substring(0, entry.path.lastIndexOf('\\') + 1) + newName
                        onFileRenamed?.(entry.path, newPath)
                        fetchFiles()
                    }
                }
                break
            case 'delete':
                if (confirm(`Are you sure you want to delete ${entry.name}?`)) {
                    const success = await ipc.invoke('fs:delete', entry.path)
                    if (success) {
                        onFileDeleted?.(entry.path)
                        fetchFiles()
                    }
                }
                break
            case 'copyPath':
                navigator.clipboard.writeText(entry.path)
                break
            case 'copyRelativePath':
                const relPath = entry.path.replace(path, '').replace(/^[\\\/]/, '')
                navigator.clipboard.writeText(relPath)
                break
            case 'revealInExplorer':
                ipc.invoke('shell:reveal', entry.path)
                break
            case 'openInTerminal':
                ipc.invoke('terminal:openAt', entry.isDirectory ? entry.path : path)
                break
        }
    }

    const handleCreateItem = async () => {
        if (!newItemDialog || !newItemName) return
        const ipc = (window as any).ipcRenderer
        if (!ipc) return

        const success = await ipc.invoke(
            newItemDialog.type === 'file' ? 'fs:createFile' : 'fs:createDirectory',
            { parentPath: newItemDialog.parentPath, name: newItemName }
        )

        if (success) {
            setNewItemDialog(null)
            setNewItemName('')
            fetchFiles()
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
                    gitStatus={gitStatus}
                />
            ))}
            
            <div 
                className="explorer-empty-space"
                style={{ flex: 1, minHeight: '100px' }}
                onClick={() => setContextMenu(null)}
            />

            {contextMenu && createPortal(
                <div 
                    className="context-menu"
                    style={{ 
                        position: 'fixed', 
                        left: Math.min(contextMenu.x, window.innerWidth - 200),
                        top: Math.min(contextMenu.y, window.innerHeight - 250),
                        zIndex: 1000,
                        boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
                        border: '1px solid var(--glass-border)',
                        minWidth: '200px'
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
                </div>,
                document.body
            )}

            {newItemDialog && (
                <div className="new-item-dialog">
                    <input
                        type="text"
                        value={newItemName}
                        onChange={(e) => setNewItemName(e.target.value)}
                        placeholder={`New ${newItemDialog.type} name...`}
                        autoFocus
                        onKeyDown={(e) => {
                            if (e.key === 'Enter') handleCreateItem()
                            if (e.key === 'Escape') setNewItemDialog(null)
                        }}
                    />
                </div>
            )}
        </div>
    )
}
