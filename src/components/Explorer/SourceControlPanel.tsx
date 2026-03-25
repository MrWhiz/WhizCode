import { useState, useEffect } from 'react'
import { git } from '../../lib/tauri-api'

interface GitStatus {
    branch: string
    changes: Array<{ file: string, status: string, staged?: boolean }>
}

interface ReviewFinding {
    file: string
    severity: string
    line: number
    message: string
    suggestion?: string
}

interface SourceControlPanelProps {
    workspacePath: string | null
}

export const SourceControlPanel = ({ workspacePath }: SourceControlPanelProps) => {
    const [gitStatus, setGitStatus] = useState<GitStatus | null>(null)
    const [commitMessage, setCommitMessage] = useState('')
    const [isLoading, setIsLoading] = useState(false)
    const [reviewFindings, setReviewFindings] = useState<ReviewFinding[]>([])
    const [isReviewing, setIsReviewing] = useState(false)

    useEffect(() => {
        if (workspacePath) {
            refreshStatus()
        }
    }, [workspacePath])

    const refreshStatus = async () => {
        if (!workspacePath) return
        setIsLoading(true)
        try {
            const status = await git.getStatus(workspacePath)
            setGitStatus(status)
        } catch (err) {
            console.error('Git status error:', err)
            setGitStatus(null)
        }
        setIsLoading(false)
    }

    const handleCommit = async () => {
        if (!commitMessage.trim() || !workspacePath) return
        setIsLoading(true)
        try {
            await git.commit(workspacePath, commitMessage.trim())
            setCommitMessage('')
            await refreshStatus()
        } catch (err) {
            console.error('Git commit error:', err)
        } finally {
            setIsLoading(false)
        }
    }

    const handleStage = async (file: string) => {
        if (!workspacePath) return
        setIsLoading(true)
        try {
            const status = await git.stageFile(workspacePath, file)
            setGitStatus(status)
        } catch (err) {
            console.error('Git stage error:', err)
        } finally {
            setIsLoading(false)
        }
    }

    const handleReview = async () => {
        if (!workspacePath) return
        setIsReviewing(true)
        try {
            const report = await git.reviewWorkingTree(workspacePath)
            setReviewFindings(report.findings)
        } catch (err) {
            console.error('Git review error:', err)
            setReviewFindings([])
        } finally {
            setIsReviewing(false)
        }
    }

    const getStatusIcon = (status: string) => {
        switch (status) {
            case 'M': return '●'
            case 'A': return '+'
            case 'D': return '−'
            case '?': return '?'
            default: return '●'
        }
    }

    const getStatusColor = (status: string) => {
        switch (status) {
            case 'M': return '#007acc'
            case 'A': return '#89d185'
            case 'D': return '#f14c4c'
            case '?': return '#858585'
            default: return '#cccccc'
        }
    }

    if (!workspacePath) {
        return <div className="empty-state">No folder opened</div>
    }

    if (isLoading) {
        return <div className="empty-state">Loading...</div>
    }

    if (!gitStatus) {
        return <div className="empty-state">Not a git repository</div>
    }

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
            <div style={{ padding: '12px', borderBottom: '1px solid var(--border-color)' }}>
                <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '8px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <polyline points="16 3 21 3 21 8" />
                        <line x1="4" y1="20" x2="21" y2="3" />
                    </svg>
                    <span>{gitStatus.branch}</span>
                    <button
                        onClick={refreshStatus}
                        style={{
                            marginLeft: 'auto',
                            background: 'none',
                            border: 'none',
                            cursor: 'pointer',
                            color: 'var(--text-secondary)',
                            padding: '2px 6px'
                        }}
                        title="Refresh"
                    >
                        ↻
                    </button>
                </div>
                <textarea
                    placeholder="Message (Ctrl+Enter to commit)"
                    value={commitMessage}
                    onChange={(e) => setCommitMessage(e.target.value)}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter' && e.ctrlKey) {
                            e.preventDefault()
                            handleCommit()
                        }
                    }}
                    style={{
                        width: '100%',
                        padding: '6px 10px',
                        fontSize: '12px',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '4px',
                        color: 'var(--text-primary)',
                        outline: 'none',
                        resize: 'vertical',
                        minHeight: '60px',
                        fontFamily: 'inherit'
                    }}
                />
                <button
                    onClick={handleCommit}
                    disabled={!commitMessage.trim()}
                    style={{
                        width: '100%',
                        marginTop: '8px',
                        padding: '6px',
                        fontSize: '12px',
                        backgroundColor: 'var(--button-bg)',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: commitMessage.trim() ? 'pointer' : 'not-allowed',
                        opacity: commitMessage.trim() ? 1 : 0.5
                    }}
                >
                    Commit
                </button>
                <button
                    onClick={handleReview}
                    disabled={isReviewing || gitStatus.changes.length === 0}
                    style={{
                        width: '100%',
                        marginTop: '8px',
                        padding: '6px',
                        fontSize: '12px',
                        backgroundColor: '#3b82f6',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: isReviewing || gitStatus.changes.length === 0 ? 'not-allowed' : 'pointer',
                        opacity: isReviewing || gitStatus.changes.length === 0 ? 0.5 : 1
                    }}
                >
                    {isReviewing ? 'Reviewing...' : 'Review Changes'}
                </button>
            </div>

            <div style={{ flex: 1, overflowY: 'auto' }}>
                <div style={{ padding: '8px 12px', fontSize: '11px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                    Changes ({gitStatus.changes.length})
                </div>
                {gitStatus.changes.length > 0 ? (
                    gitStatus.changes.map((change, idx) => (
                        <div
                            key={idx}
                            style={{
                                padding: '6px 12px',
                                fontSize: '13px',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '8px',
                                cursor: 'pointer'
                            }}
                            className="explorer-item"
                            onClick={() => handleStage(change.file)}
                        >
                            <span style={{ color: getStatusColor(change.status), fontWeight: 'bold' }}>
                                {getStatusIcon(change.status)}
                            </span>
                            {change.staged && (
                                <span style={{ fontSize: '10px', color: '#89d185', fontWeight: 700 }}>
                                    STAGED
                                </span>
                            )}
                            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                                {change.file}
                            </span>
                        </div>
                    ))
                ) : (
                    <div className="empty-state">No changes</div>
                )}
                <div style={{ padding: '12px', borderTop: '1px solid var(--border-color)' }}>
                    <div style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase', marginBottom: '8px' }}>
                        Review Findings ({reviewFindings.length})
                    </div>
                    {reviewFindings.length > 0 ? (
                        reviewFindings.map((finding, idx) => (
                            <div
                                key={`${finding.file}:${finding.line}:${idx}`}
                                style={{
                                    padding: '8px',
                                    borderRadius: '4px',
                                    marginBottom: '8px',
                                    background: 'rgba(255,255,255,0.03)',
                                    borderLeft: `3px solid ${finding.severity === 'error' ? '#f14c4c' : '#f59e0b'}`
                                }}
                            >
                                <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                                    {finding.file}:{finding.line}
                                </div>
                                <div style={{ fontSize: '12px', color: 'var(--text-primary)', marginBottom: '4px' }}>
                                    {finding.message}
                                </div>
                                {finding.suggestion && (
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
                                        Suggestion: {finding.suggestion}
                                    </div>
                                )}
                            </div>
                        ))
                    ) : (
                        <div className="empty-state" style={{ padding: '8px 0' }}>
                            {isReviewing ? 'Review in progress...' : 'No review findings yet'}
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
