interface ActivityBarProps {
    isChatOpen: boolean;
    setIsChatOpen: (open: boolean) => void;
}

export const ActivityBar = ({ isChatOpen, setIsChatOpen }: ActivityBarProps) => {
    return (
        <div className="activity-bar">
            <svg className="activity-icon active" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="9" y1="3" x2="9" y2="21"></line>
            </svg>
            <svg className="activity-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <circle cx="11" cy="11" r="8"></circle>
                <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            </svg>
            <svg
                className={`activity-icon ${isChatOpen ? 'active' : ''}`}
                viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
                onClick={() => setIsChatOpen(!isChatOpen)}
                title="Toggle Chat Panel"
            >
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
            </svg>
        </div>
    )
}
