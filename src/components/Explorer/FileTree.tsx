import React, { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import type { FileEntry } from '../../types'
import { fs, events } from '../../lib/tauri-api'
import Popup from '../UI/Popup'
import type { PopupType } from '../UI/Popup'
import { 
    FiFile, 
    FiFolder, 
    FiChevronRight, 
    FiChevronDown, 
    FiMoreVertical, 
    FiPlus, 
    FiFolderPlus, 
    FiTrash2, 
    FiEdit2, 
    FiCopy, 
    FiCornerUpRight, 
    FiTerminal, 
    FiExternalLink 
} from 'react-icons/fi'

interface ContextMenu {
    x: number
    y: number
    entry: FileEntry
}

interface FileTreeItemProps {
    entry: FileEntry
    level?: number
    onFileOpen: (path: string, name: string) => void
    refreshKey: number
    onContextMenu: (e: React.MouseEvent, entry: FileEntry) => void
    collapseAll?: boolean
    fileFilter?: string
    fileErrors?: Record<string, number>
    gitStatus?: { branch: string, changes: { file: string, status: string }[] } | null
    workspacePath?: string
    expandedPaths: Set<string>
    onToggleExpand: (path: string, expanded: boolean) => void
}

const FileTreeItem: React.FC<FileTreeItemProps> = ({
    entry,
    level = 0,
    onFileOpen,
    refreshKey,
    onContextMenu,
    collapseAll = false,
    fileFilter = '',
    fileErrors = {},
    gitStatus = null,
    workspacePath = '',
    expandedPaths,
    onToggleExpand,
}) => {
    const [children, setChildren] = useState<FileEntry[]>([])
    const expanded = expandedPaths.has(entry.path)

    const fetchChildren = useCallback(async () => {
        if (entry.isDirectory) {
            try {
                const res = await fs.readDirectory(entry.path)
                const sorted = res.sort((a, b) => {
                    if (a.isDirectory === b.isDirectory) {
                        return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
                    }
                    return a.isDirectory ? -1 : 1
                })
                setChildren(sorted)
            } catch (error) {
                console.error('Failed to read directory:', error)
                setChildren([])
            }
        }
    }, [entry.path, entry.isDirectory])

    useEffect(() => {
        if (collapseAll) {
            onToggleExpand(entry.path, false)
        }
    }, [collapseAll])

    // Re-fetch whenever refreshKey changes and folder is expanded
    useEffect(() => {
        if (expanded && entry.isDirectory) {
            fetchChildren()
        }
    }, [refreshKey, expanded, entry.isDirectory, fetchChildren])

    const handleClick = async () => {
        if (entry.isDirectory) {
            if (!expanded) {
                await fetchChildren()
            }
            onToggleExpand(entry.path, !expanded)
        } else {
            onFileOpen(entry.path, entry.name)
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

    const getGitStatusSymbol = () => {
        if (!gitStatus || !gitStatus.changes) return null;
        
        const normalizePath = (p: string) => p.replace(/\\/g, '/').replace(/^\/\?\//, '').toLowerCase();
        let relativePath = entry.path;
        if (workspacePath) {
            const wsPath = normalizePath(workspacePath);
            const entryPath = normalizePath(entry.path);
            if (entryPath.startsWith(wsPath)) {
                relativePath = entryPath.substring(wsPath.length).replace(/^\//, '');
            }
        }
        
        const match = gitStatus.changes.find(c => normalizePath(c.file) === relativePath);
        if (match) return match.status;

        if (entry.isDirectory) {
            const hasChangesInside = gitStatus.changes.some(c => normalizePath(c.file).startsWith(relativePath + '/'));
            if (hasChangesInside) return 'M';
        }
        return null;
    }

    const gStatus = getGitStatusSymbol();
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
                        <span style={{ fontSize: '10px', color: statusColor, fontWeight: 'bold', padding: '0 4px', opacity: 0.8 }}>
                            {gStatus === '??' ? 'U' : gStatus}
                        </span>
                    )}
                    {totalErrorCount > 0 && (
                        <span style={{ padding: '1px 5px', backgroundColor: '#f14c4c', color: 'white', borderRadius: '10px', fontSize: '9px', fontWeight: 'bold', minWidth: '14px', textAlign: 'center' }}>
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
                            workspacePath={workspacePath}
                            expandedPaths={expandedPaths}
                            onToggleExpand={onToggleExpand}
                        />
                    ))}
                </div>
            )}
        </div>
    )
}

interface FileTreeProps {
    path: string
    onFileOpen: (path: string, name: string) => void
    onFileDeleted?: (deletedPath: string) => void
    onFileRenamed?: (oldPath: string, newPath: string) => void
    refreshKey?: number
    collapseAll?: boolean
    fileFilter?: string
    fileErrors?: Record<string, number>
    gitStatus?: { branch: string, changes: { file: string, status: string }[] } | null
}

export const FileTree: React.FC<FileTreeProps> = ({ 
    path, 
    onFileOpen, 
    onFileDeleted, 
    onFileRenamed,
    refreshKey: externalRefreshKey = 0,
    collapseAll = false,
    fileFilter = '',
    fileErrors = {},
    gitStatus = null
}) => {
    const [files, setFiles] = useState<FileEntry[]>([])
    const [refreshKey, setRefreshKey] = useState(0)
    const [contextMenu, setContextMenu] = useState<ContextMenu | null>(null)
    const [newItemDialog, setNewItemDialog] = useState<{ type: 'file' | 'folder', parentPath: string } | null>(null)
    const [newItemName, setNewItemName] = useState('')

    // Persist expanded folders per workspace path
    const storageKey = `filetree-expanded:${path}`
    const [expandedPaths, setExpandedPaths] = useState<Set<string>>(() => {
        try {
            const saved = localStorage.getItem(storageKey)
            return saved ? new Set(JSON.parse(saved)) : new Set()
        } catch {
            return new Set()
        }
    })

    // Reset expanded state when workspace changes
    useEffect(() => {
        try {
            const saved = localStorage.getItem(`filetree-expanded:${path}`)
            setExpandedPaths(saved ? new Set(JSON.parse(saved)) : new Set())
        } catch {
            setExpandedPaths(new Set())
        }
    }, [path])

    const handleToggleExpand = useCallback((entryPath: string, isExpanded: boolean) => {
        setExpandedPaths(prev => {
            const next = new Set(prev)
            if (isExpanded) {
                next.add(entryPath)
            } else {
                next.delete(entryPath)
            }
            try {
                localStorage.setItem(`filetree-expanded:${path}`, JSON.stringify([...next]))
            } catch {}
            return next
        })
    }, [path])

    // Popup state
    const [popupState, setPopupState] = useState<{
        isOpen: boolean;
        title: string;
        message: string;
        type: PopupType;
        onConfirm?: () => void;
    }>({
        isOpen: false,
        title: '',
        message: '',
        type: 'info'
    })

    const showPopup = (title: string, message: string, type: PopupType = 'info', onConfirm?: () => void) => {
        setPopupState({
            isOpen: true,
            title,
            message,
            type,
            onConfirm: onConfirm ? () => {
                onConfirm()
                closePopup()
            } : undefined
        })
    }

    const closePopup = () => {
        setPopupState(prev => ({ ...prev, isOpen: false }))
    }

    const fetchFiles = useCallback(async () => {
        try {
            const res = await fs.readDirectory(path)
            const sorted = res.sort((a, b) => {
                if (a.isDirectory === b.isDirectory) {
                    return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
                }
                return a.isDirectory ? -1 : 1
            })
            setFiles(sorted)
        } catch (error) {
            console.error('Failed to read directory:', path, error)
            setFiles([])
        }
    }, [path])

    useEffect(() => {
        fetchFiles()
    }, [path, externalRefreshKey, fetchFiles])

    // Handle collapseAll
    useEffect(() => {
        if (collapseAll) {
            setExpandedPaths(new Set())
            try { localStorage.setItem(`filetree-expanded:${path}`, '[]') } catch {}
        }
    }, [collapseAll, path])


    const handleContextMenu = (e: React.MouseEvent, entry: FileEntry) => {
        setContextMenu({ x: e.clientX, y: e.clientY, entry })
    }

    const handleRootContextMenu = (e: React.MouseEvent) => {
        e.preventDefault()
        setContextMenu({ x: e.clientX, y: e.clientY, entry: { name: '', path, isDirectory: true } })
    }

    const handleMenuAction = async (action: string) => {
        if (!contextMenu) return
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
                // For now, rename still uses prompt as it's an input. 
                // In a future version, this could be an in-line editor or a full Modal.
                const newName = prompt('Enter new name:', entry.name)
                if (newName && newName !== entry.name) {
                    try {
                        const lastSlash = Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\'));
                        const parentPath = entry.path.substring(0, lastSlash + 1);
                        const newPath = parentPath + newName
                        await fs.renameFile(entry.path, newPath)
                        onFileRenamed?.(entry.path, newPath)
                        fetchFiles()
                        setRefreshKey(prev => prev + 1)
                    } catch (error) {
                        showPopup('Rename Failed', error instanceof Error ? error.message : String(error), 'error')
                    }
                }
                break
            case 'delete':
                showPopup(
                    'Delete Item',
                    `Are you sure you want to delete ${entry.name}?`,
                    'confirm',
                    async () => {
                        try {
                            if (entry.isDirectory) {
                                await fs.deleteDirectory(entry.path)
                            } else {
                                await fs.deleteFile(entry.path)
                            }
                            onFileDeleted?.(entry.path)
                            fetchFiles()
                            setRefreshKey(prev => prev + 1)
                        } catch (error) {
                            let message = error instanceof Error ? error.message : String(error)
                            if (message.includes('os error 32') || message.includes('used by another process')) {
                                message = 'The directory is being used by another process. Please close any active processes in this folder and try again.'
                            }
                            showPopup('Delete Failed', message, 'error')
                        }
                    }
                )
                break
            case 'copyPath':
                navigator.clipboard.writeText(entry.path)
                break
            case 'copyRelativePath':
                const relPath = entry.path.replace(path, '').replace(/^[\\\/]/, '')
                navigator.clipboard.writeText(relPath)
                break
            case 'revealInExplorer':
                try { await fs.revealInExplorer(entry.path) } catch (error) { 
                    showPopup('Reveal Failed', String(error), 'error')
                }
                break
            case 'openInTerminal':
                try {
                    let dirPath = entry.path;
                    if (!entry.isDirectory) {
                        const lastSlash = Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\'));
                        dirPath = lastSlash !== -1 ? entry.path.substring(0, lastSlash) : entry.path;
                    }
                    await fs.openTerminal(dirPath)
                } catch (error) { 
                    showPopup('Terminal Failed', String(error), 'error')
                }
                break
        }
    }

    const handleCreateItem = async () => {
        if (!newItemDialog || !newItemName) return
        try {
            const separator = newItemDialog.parentPath.includes('\\') ? '\\' : '/'
            const parent = newItemDialog.parentPath.replace(/[/\\]$/, '')
            const itemPath = `${parent}${separator}${newItemName}`
            if (newItemDialog.type === 'file') await fs.createFile(itemPath)
            else await fs.createDirectory(itemPath)
            setNewItemDialog(null)
            setNewItemName('')
            fetchFiles()
            setRefreshKey(prev => prev + 1)
        } catch (error) {
            showPopup('Creation Failed', error instanceof Error ? error.message : String(error), 'error')
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
                    workspacePath={path}
                    expandedPaths={expandedPaths}
                    onToggleExpand={handleToggleExpand}
                />
            ))}
            
            <div className="explorer-empty-space" style={{ flex: 1, minHeight: '100px' }} onClick={() => setContextMenu(null)} />

            {contextMenu && createPortal(
                <div className="context-menu glass-dark animate-fade-in"
                    style={{ 
                        position: 'fixed', 
                        left: Math.min(contextMenu.x, window.innerWidth - 200),
                        top: Math.min(contextMenu.y, window.innerHeight - 350),
                        zIndex: 1000,
                        minWidth: '200px'
                    }}
                    onClick={(e) => e.stopPropagation()}>
                    <div className="context-menu-item" onClick={() => handleMenuAction('newFile')}>
                        <FiFile className="context-menu-icon" /> New File
                    </div>
                    <div className="context-menu-item" onClick={() => handleMenuAction('newFolder')}>
                        <FiFolderPlus className="context-menu-icon" /> New Folder
                    </div>
                    {contextMenu.entry.name && (
                        <>
                            <div className="context-menu-separator" />
                            <div className="context-menu-item" onClick={() => handleMenuAction('rename')}>
                                <FiEdit2 className="context-menu-icon" /> Rename
                            </div>
                            <div className="context-menu-item danger" onClick={() => handleMenuAction('delete')}>
                                <FiTrash2 className="context-menu-icon" /> Delete
                            </div>
                            <div className="context-menu-separator" />
                            <div className="context-menu-item" onClick={() => handleMenuAction('copyPath')}>
                                <FiCopy className="context-menu-icon" /> Copy Path
                            </div>
                            <div className="context-menu-item" onClick={() => handleMenuAction('revealInExplorer')}>
                                <FiExternalLink className="context-menu-icon" /> Reveal in Explorer
                            </div>
                            <div className="context-menu-item" onClick={() => handleMenuAction('openInTerminal')}>
                                <FiTerminal className="context-menu-icon" /> Open in Terminal
                            </div>
                        </>
                    )}
                </div>,
                document.body
            )}

            {newItemDialog && (
                <div className="new-item-dialog glass animate-fade-in" style={{ padding: '8px' }}>
                    <input
                        className="new-item-input"
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

            <Popup
                isOpen={popupState.isOpen}
                onClose={closePopup}
                onConfirm={popupState.onConfirm}
                title={popupState.title}
                message={popupState.message}
                type={popupState.type}
            />
        </div>
    )
}
