

interface ActivityBarProps {
    activeView: 'explorer' | 'search' | 'source-control' | 'tasks' | null
    setActiveView: (view: 'explorer' | 'search' | 'source-control' | 'tasks' | null) => void
    isChatOpen: boolean
    setIsChatOpen: (open: boolean) => void
}

export const ActivityBar = ({ activeView, setActiveView, isChatOpen, setIsChatOpen }: ActivityBarProps) => {
    return (
        <div className="activity-bar">
            <div
                className={`activity-item ${activeView === 'explorer' ? 'active' : ''}`}
                onClick={() => setActiveView(activeView === 'explorer' ? null : 'explorer')}
                title="Explorer (Ctrl+Shift+E)"
            >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M3 3h7l2 2h9v14H3V3z" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
            </div>
            <div
                className={`activity-item ${activeView === 'search' ? 'active' : ''}`}
                onClick={() => setActiveView(activeView === 'search' ? null : 'search')}
                title="Search (Ctrl+Shift+F)"
            >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <circle cx="11" cy="11" r="8" strokeLinecap="round" strokeLinejoin="round" />
                    <path d="m21 21-4.35-4.35" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
            </div>
            <div
                className={`activity-item ${activeView === 'source-control' ? 'active' : ''}`}
                onClick={() => setActiveView(activeView === 'source-control' ? null : 'source-control')}
                title="Source Control (Ctrl+Shift+G)"
            >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M12 2v8M6.29 12.29l1.42 1.42M17.71 12.29l-1.42 1.42M2 12h8M14 12h8M6.29 11.71l1.42-1.42M17.71 11.71l-1.42-1.42" strokeLinecap="round" strokeLinejoin="round"/>
                    <path d="M18 18c0 1.1-.9 2-2 2H8c-1.1 0-2-.9-2-2v-3l6-6 6 6v3z"/>
                </svg>
            </div>
            <div
                className={`activity-item ${activeView === 'tasks' ? 'active' : ''}`}
                onClick={() => setActiveView(activeView === 'tasks' ? null : 'tasks')}
                title="To-Do List"
            >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="m9 11 3 3L22 4M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
            </div>
            <div
                className={`activity-item ${isChatOpen ? 'active' : ''}`}
                onClick={() => setIsChatOpen(!isChatOpen)}
                title="AI Agent"
                style={{ marginTop: 'auto' }}
            >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
            </div>
        </div>
    )
}
