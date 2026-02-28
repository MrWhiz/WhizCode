import Editor from '@monaco-editor/react'
import type { OpenFileProps } from '../../types'

interface EditorAreaProps {
    openFiles: OpenFileProps[];
    activeFileId: string | null;
    setActiveFileId: (id: string | null) => void;
    workspacePath: string | null;
    handleFileClose: (path: string, e: React.MouseEvent) => void;
    getLanguage: (fileName: string) => string;
    handleContentChange: (newContent: string | undefined) => void;
    handleMenuAction: (action: string) => void;
}

export const EditorArea = ({
    openFiles,
    activeFileId,
    setActiveFileId,
    workspacePath,
    handleFileClose,
    getLanguage,
    handleContentChange,
    handleMenuAction
}: EditorAreaProps) => {
    const activeFile = openFiles.find(f => f.path === activeFileId);

    return (
        <main className="main-area" style={{ display: 'flex', flexDirection: 'column' }}>
            {openFiles.length > 0 ? (
                <>
                    <div className="tabs" style={{ display: 'flex', overflowX: 'auto' }}>
                        {openFiles.map(file => (
                            <div key={file.path} className={`tab ${activeFileId === file.path ? 'active' : ''}`} onClick={() => setActiveFileId(file.path)}>
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#007acc" strokeWidth="2" style={{ marginRight: '2px' }}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path></svg>
                                {file.name}
                                <div style={{ marginLeft: 16, fontSize: 12, padding: '2px 4px', cursor: 'pointer', borderRadius: '4px' }} className="close-btn" onClick={(e) => handleFileClose(file.path, e)}>×</div>
                            </div>
                        ))}
                    </div>

                    <div className="breadcrumbs">
                        WhizCode <span style={{ opacity: 0.5 }}>&gt;</span> {activeFileId?.replace(workspacePath || '', '')}
                    </div>

                    <div style={{ flex: 1, overflow: 'hidden', backgroundColor: '#1e1e1e', margin: '0' }}>
                        <Editor
                            height="100%"
                            language={getLanguage(activeFile?.name || '')}
                            theme="vs-dark"
                            value={activeFile?.content || ''}
                            onChange={handleContentChange}
                            options={{
                                minimap: { enabled: false },
                                fontSize: 14,
                                wordWrap: 'on',
                                fontFamily: "'Consolas', 'Courier New', monospace"
                            }}
                        />
                    </div>
                </>
            ) : (
                <div className="welcome-screen">
                    <div className="welcome-content">
                        <div className="welcome-icon">⚡</div>
                        <h1 className="welcome-title">WhizCode</h1>
                        <p className="welcome-subtitle">Ollama-powered code editor</p>
                        <div className="welcome-actions">
                            <button className="welcome-btn" onClick={() => handleMenuAction('open-folder')}>
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
                                Open Folder
                            </button>
                            <button className="welcome-btn secondary" onClick={() => handleMenuAction('new-terminal')}>
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>
                                New Terminal
                            </button>
                        </div>
                        <p className="welcome-hint">Open a folder and use the chat panel to ask the AI about your code</p>
                    </div>
                </div>
            )}
        </main>
    )
}
