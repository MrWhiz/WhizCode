import { useState } from 'react'
import { search } from '../../lib/tauri-api'

interface SearchPanelProps {
    workspacePath: string | null
    onFileOpen: (path: string, name: string) => void
}

export const SearchPanel = ({ workspacePath, onFileOpen }: SearchPanelProps) => {
    const [searchQuery, setSearchQuery] = useState('')
    const [searchResults, setSearchResults] = useState<Array<{ file: string, line: number, content: string }>>([])
    const [isSearching, setIsSearching] = useState(false)
    const [includePattern, setIncludePattern] = useState('')
    const [excludePattern, setExcludePattern] = useState('')

    const handleSearch = async () => {
        if (!searchQuery.trim() || !workspacePath) return
        
        setIsSearching(true)
        try {
            const results = await search.searchFiles(searchQuery, includePattern || undefined)
            setSearchResults(results || [])
        } catch (err) {
            console.error('Search error:', err)
            setSearchResults([])
        }
        setIsSearching(false)
    }

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
            <div style={{ padding: '12px', borderBottom: '1px solid var(--border-color)' }}>
                <input
                    type="text"
                    placeholder="Search"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                    style={{
                        width: '100%',
                        padding: '6px 10px',
                        fontSize: '13px',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '4px',
                        color: 'var(--text-primary)',
                        outline: 'none',
                        marginBottom: '8px'
                    }}
                />
                <input
                    type="text"
                    placeholder="files to include"
                    value={includePattern}
                    onChange={(e) => setIncludePattern(e.target.value)}
                    style={{
                        width: '100%',
                        padding: '4px 8px',
                        fontSize: '11px',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '3px',
                        color: 'var(--text-primary)',
                        outline: 'none',
                        marginBottom: '6px'
                    }}
                />
                <input
                    type="text"
                    placeholder="files to exclude"
                    value={excludePattern}
                    onChange={(e) => setExcludePattern(e.target.value)}
                    style={{
                        width: '100%',
                        padding: '4px 8px',
                        fontSize: '11px',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '3px',
                        color: 'var(--text-primary)',
                        outline: 'none'
                    }}
                />
            </div>

            <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
                {isSearching ? (
                    <div className="empty-state">Searching...</div>
                ) : searchResults.length > 0 ? (
                    <div style={{ fontSize: '12px' }}>
                        <div style={{ padding: '8px', color: 'var(--text-secondary)', fontSize: '11px' }}>
                            {searchResults.length} result{searchResults.length !== 1 ? 's' : ''} in {new Set(searchResults.map(r => r.file)).size} file{new Set(searchResults.map(r => r.file)).size !== 1 ? 's' : ''}
                        </div>
                        {searchResults.map((result, idx) => (
                            <div
                                key={idx}
                                style={{
                                    padding: '6px 8px',
                                    cursor: 'pointer',
                                    borderRadius: '3px',
                                    marginBottom: '2px'
                                }}
                                className="explorer-item"
                                onClick={() => onFileOpen(result.file, result.file.split(/[/\\]/).pop() || '')}
                            >
                                <div style={{ color: 'var(--text-primary)', marginBottom: '2px' }}>
                                    {result.file.replace(workspacePath + '/', '')}
                                </div>
                                <div style={{ color: 'var(--text-secondary)', fontSize: '11px', display: 'flex', gap: '8px' }}>
                                    <span>Line {result.line}</span>
                                    <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                                        {result.content}
                                    </span>
                                </div>
                            </div>
                        ))}
                    </div>
                ) : searchQuery ? (
                    <div className="empty-state">No results found</div>
                ) : (
                    <div className="empty-state">Enter search query</div>
                )}
            </div>
        </div>
    )
}
