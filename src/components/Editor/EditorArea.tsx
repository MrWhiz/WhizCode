import Editor from '@monaco-editor/react'
import { useRef, useEffect, useState } from 'react'
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
    fileErrors?: Record<string, number>;
    onFixError?: (error: { file: string; line: number; message: string; code?: string }) => void;
}

export const EditorArea = ({
    openFiles,
    activeFileId,
    setActiveFileId,
    workspacePath,
    handleFileClose,
    getLanguage,
    handleContentChange,
    handleMenuAction,
    fileErrors = {},
    onFixError
}: EditorAreaProps) => {
    const activeFile = openFiles.find(f => f.path === activeFileId);
    const editorRef = useRef<any>(null);
    const lastContentRef = useRef<string>('');
    const [markers, setMarkers] = useState<any[]>([]);
    const [diagnostics, setDiagnostics] = useState<any[]>([]);

    // Update editor content when file changes from backend
    useEffect(() => {
        if (activeFile && editorRef.current && activeFile.content !== lastContentRef.current) {
            const editor = editorRef.current;
            const model = editor.getModel();
            if (model) {
                // Set the content directly on the model to avoid triggering onChange
                const currentContent = model.getValue();
                if (currentContent !== activeFile.content) {
                    model.setValue(activeFile.content);
                    lastContentRef.current = activeFile.content;
                }
            }
        }
    }, [activeFile?.content, activeFileId]);

    // Fetch diagnostics for the active file
    useEffect(() => {
        const fetchDiagnostics = async () => {
            if (!activeFileId || !workspacePath) {
                setDiagnostics([]);
                return;
            }
            
            const ipc = (window as any).ipcRenderer;
            if (!ipc) {
                setDiagnostics([]);
                return;
            }

            try {
                // Add timeout to prevent hanging
                const timeoutPromise = new Promise((_, reject) => 
                    setTimeout(() => reject(new Error('Diagnostics check timeout')), 3000)
                );
                
                const diags = await Promise.race([
                    ipc.invoke('diagnostics:check', activeFileId, workspacePath),
                    timeoutPromise
                ]);
                
                const diagnosticsArray = Array.isArray(diags) ? diags : [];
                console.log(`[EDITOR] Fetched ${diagnosticsArray.length} diagnostics for ${activeFileId}`);
                setDiagnostics(diagnosticsArray);
            } catch (error) {
                console.error('Error fetching diagnostics:', error);
                setDiagnostics([]);
            }
        };

        fetchDiagnostics();
    }, [activeFileId, workspacePath]);

    // Update error markers when diagnostics change
    useEffect(() => {
        try {
            if (editorRef.current && diagnostics.length > 0) {
                const editor = editorRef.current;
                const monaco = editor._domElement?.ownerDocument?.defaultView?.monaco;
                if (monaco) {
                    const model = editor.getModel();
                    if (model) {
                        const errorMarkers = diagnostics.map((diag: any) => ({
                            startLineNumber: Math.max(1, diag.line || 1),
                            startColumn: Math.max(1, diag.column || 1),
                            endLineNumber: Math.max(1, diag.line || 1),
                            endColumn: Math.min(Math.max(1, (diag.column || 1) + 50), 200),
                            message: diag.message || 'Error',
                            severity: diag.severity === 'error' ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
                            code: diag.code,
                        }));
                        monaco.editor.setModelMarkers(model, 'owner', errorMarkers);
                    }
                }
            } else if (editorRef.current && diagnostics.length === 0) {
                const editor = editorRef.current;
                const monaco = editor._domElement?.ownerDocument?.defaultView?.monaco;
                if (monaco) {
                    const model = editor.getModel();
                    if (model) {
                        monaco.editor.setModelMarkers(model, 'owner', []);
                    }
                }
            }
        } catch (error) {
            console.error('Error updating markers:', error);
        }
    }, [diagnostics]);

    // Set up editor context menu for fixing errors
    useEffect(() => {
        if (!editorRef.current) return;

        try {
            const editor = editorRef.current;
            const monaco = editor._domElement?.ownerDocument?.defaultView?.monaco;
            if (!monaco) return;

            // Register context menu action
            const disposable = editor.addAction({
                id: 'whizcode.fixError',
                label: '🔧 Fix with WhizCode',
                keybindings: [],
                precondition: null,
                keybindingContext: null,
                contextMenuGroupId: '1_modification',
                contextMenuOrder: 1.5,
                run: (ed: any) => {
                    const position = ed.getPosition();
                    if (!position) return;

                    // Get the line content
                    const model = ed.getModel();
                    const lineContent = model.getLineContent(position.lineNumber);
                    
                    // Find error at this line
                    const markers = monaco.editor.getModelMarkers({ resource: model.uri });
                    const errorAtLine = markers.find((m: any) => m.startLineNumber === position.lineNumber);
                    
                    if (errorAtLine && onFixError) {
                        onFixError({
                            file: activeFileId || '',
                            line: position.lineNumber,
                            message: errorAtLine.message,
                            code: errorAtLine.code
                        });
                    }
                }
            });

            return () => {
                try {
                    disposable.dispose();
                } catch (e) {
                    // Ignore disposal errors
                }
            };
        } catch (error) {
            console.error('Error setting up editor context menu:', error);
        }
    }, [activeFileId, onFixError]);

    const handleEditorChange = (newContent: string | undefined) => {
        if (newContent !== undefined) {
            lastContentRef.current = newContent;
        }
        handleContentChange(newContent);
    };

    const handleEditorMount = (editor: any) => {
        editorRef.current = editor;
        lastContentRef.current = activeFile?.content || '';
    };

    return (
        <main className="main-area" style={{ display: 'flex', flexDirection: 'column' }}>
            {openFiles.length > 0 ? (
                <>
                    <div className="tabs" style={{ display: 'flex', overflowX: 'auto' }}>
                        {openFiles.map(file => (
                            <div key={file.path} className={`tab ${activeFileId === file.path ? 'active' : ''}`} onClick={() => setActiveFileId(file.path)}>
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#007acc" strokeWidth="2" style={{ marginRight: '2px' }}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path></svg>
                                {file.name}
                                {fileErrors[file.path] && fileErrors[file.path] > 0 && (
                                    <span style={{ 
                                        marginLeft: '6px', 
                                        padding: '2px 6px', 
                                        backgroundColor: '#f48771', 
                                        color: '#fff', 
                                        borderRadius: '3px', 
                                        fontSize: '11px',
                                        fontWeight: 'bold'
                                    }}>
                                        {fileErrors[file.path]}
                                    </span>
                                )}
                                <div style={{ marginLeft: 16, fontSize: 12, padding: '2px 4px', cursor: 'pointer', borderRadius: '4px' }} className="close-btn" onClick={(e) => handleFileClose(file.path, e)}>×</div>
                            </div>
                        ))}
                    </div>

                    <div className="breadcrumbs" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div>
                            WhizCode <span style={{ opacity: 0.5 }}>&gt;</span> {activeFileId?.replace(workspacePath || '', '')}
                        </div>
                        {diagnostics.length > 0 && (
                            <button
                                onClick={() => {
                                    if (diagnostics.length > 0 && onFixError) {
                                        const error = diagnostics[0];
                                        onFixError({
                                            file: activeFileId || '',
                                            line: error.line,
                                            message: error.message,
                                            code: error.code
                                        });
                                    }
                                }}
                                style={{
                                    padding: '4px 12px',
                                    backgroundColor: '#f48771',
                                    color: '#fff',
                                    border: 'none',
                                    borderRadius: '4px',
                                    cursor: 'pointer',
                                    fontSize: '12px',
                                    fontWeight: 'bold',
                                    marginRight: '8px'
                                }}
                            >
                                🔧 Fix with WhizCode ({diagnostics.length})
                            </button>
                        )}
                    </div>

                    <div style={{ flex: 1, overflow: 'hidden', backgroundColor: '#1e1e1e', margin: '0' }}>
                        <Editor
                            height="100%"
                            language={getLanguage(activeFile?.name || '')}
                            theme="vs-dark"
                            value={activeFile?.content || ''}
                            onChange={handleEditorChange}
                            onMount={handleEditorMount}
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
