import Editor, { useMonaco } from '@monaco-editor/react'
import { useRef, useEffect, useState } from 'react'
import type { OpenFileProps } from '../../types'
import { WhizLogo } from '../Branding/WhizLogo'
import { MarkdownRenderer } from './MarkdownRenderer'
import { diagnostics as diagnosticsApi } from '../../lib/tauri-api'

// Reusable debounce utility
function useDebounce<T>(value: T, delayMs: number): T {
    const [debounced, setDebounced] = useState<T>(value);
    useEffect(() => {
        const t = setTimeout(() => setDebounced(value), delayMs);
        return () => clearTimeout(t);
    }, [value, delayMs]);
    return debounced;
}

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
    onFixError?: (filePath: string, line: number, message: string) => void;
    onValidation?: (filePath: string, count: number) => void;
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
    onFixError,
    onValidation
}: EditorAreaProps) => {
    const monaco = useMonaco();
    const activeFile = openFiles.find(f => f.path === activeFileId);
    const editorRef = useRef<any>(null);
    const lastContentRef = useRef<string>('');
    const [diagnostics, setDiagnostics] = useState<any[]>([]);
    const onFixErrorRef = useRef(onFixError);
    const [_editorInstance, setEditorInstance] = useState<any>(null);
    const [isPreviewEnabled, setIsPreviewEnabled] = useState(false);

    const isMarkdown = activeFile?.name.toLowerCase().endsWith('.md');

    // Reset preview when active file changes and it's not markdown
    useEffect(() => {
        if (!isMarkdown) {
            setIsPreviewEnabled(false);
        }
    }, [activeFileId, isMarkdown]);


    // Keep ref in sync
    useEffect(() => {
        onFixErrorRef.current = onFixError;
    }, [onFixError]);

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

    // Debounce content so diagnostics don't fire on every single keystroke.
    // 500ms is a comfortable pause that feels instant but won't flood the IPC bridge.
    const debouncedContent = useDebounce(activeFile?.content, 500);

    // Fetch diagnostics for the active file
    useEffect(() => {
        const fetchDiagnostics = async () => {
            if (!activeFileId || !workspacePath) {
                setDiagnostics([]);
                return;
            }
            
            try {
                const diagnosticsPromise = diagnosticsApi.check(activeFileId, debouncedContent || '', getLanguage(activeFile?.name || ''))
                    .catch(error => {
                        if (error?.type === 'cancelation' || 
                            error?.msg?.includes('canceled') || 
                            error?.message?.includes('canceled')) {
                            return [];
                        }
                        throw error;
                    });

                const timeoutPromise = new Promise<any[]>((_, reject) => 
                    setTimeout(() => reject(new Error('Diagnostics check timeout')), 3000)
                );
                
                const diags = await Promise.race([diagnosticsPromise, timeoutPromise]);
                setDiagnostics(Array.isArray(diags) ? diags : []);
            } catch (error: any) {
                if (error?.type === 'cancelation' || 
                    error?.msg?.includes('canceled') || 
                    error?.message?.includes('canceled') ||
                    error?.message === 'Diagnostics check timeout') {
                    return;
                }
                console.error('Error fetching diagnostics:', error);
                setDiagnostics([]);
            }
        };

        fetchDiagnostics();
        // Use debouncedContent instead of activeFile?.content to avoid firing on every keystroke
    }, [activeFileId, workspacePath, debouncedContent]);

    // Update error markers when diagnostics change
    useEffect(() => {
        if (!monaco || !editorRef.current) return;

        try {
            const editor = editorRef.current;
            const model = editor.getModel();
            if (model) {
                if (diagnostics.length > 0) {
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
                } else {
                    monaco.editor.setModelMarkers(model, 'owner', []);
                }
            }
        } catch (error) {
            console.error('Error updating markers:', error);
        }
    }, [diagnostics, monaco]);

    // Register provider and commands when monaco is available
    useEffect(() => {
        if (!monaco) return;

        const processedInitialPaths = new Set<string>();
        monaco.editor.getModels().forEach((model: any) => {
            const markers = monaco.editor.getModelMarkers({ resource: model.uri });
            const errorCount = markers.filter((m: any) => m.severity === monaco.MarkerSeverity.Error).length;
            
            let rawPath = model.uri.fsPath || model.uri.path || '';
            if (rawPath.startsWith('/') && rawPath.includes(':')) {
                rawPath = rawPath.substring(1);
            }
            
            const normPath = rawPath.replace(/\\/g, '/').toLowerCase();
            if (!processedInitialPaths.has(normPath)) {
                processedInitialPaths.add(normPath);
                if (onValidation) {
                    onValidation(rawPath, errorCount);
                }
            }
        });

        // Register the "Fix with WhiZcode" code action provider
        const provider = monaco.languages.registerCodeActionProvider('*', {
            provideCodeActions: (model: any, _range: any, context: any) => {
                const relevantMarkers = (context.markers || [])
                    .filter((m: any) => m.severity === monaco.MarkerSeverity.Error || m.severity === monaco.MarkerSeverity.Warning);
                
                if (relevantMarkers.length === 0) return { actions: [], dispose: () => {} };

                // Only show one "Fix with WhiZcode" even if there are multiple errors
                return {
                    actions: [{
                        title: '✨ Fix with WhiZcode',
                        diagnostics: relevantMarkers,
                        kind: 'quickfix',
                        command: {
                            id: 'whizcode.fixError',
                            title: 'Fix with WhiZcode',
                            arguments: [
                                model.uri.fsPath || model.uri.path || '', 
                                relevantMarkers[0].startLineNumber, 
                                relevantMarkers[0].message
                            ]
                        },
                        isPreferred: true
                    }],
                    dispose: () => {}
                };
            }
        });

        // Debounce validation callbacks to avoid flooding message queue when many
        // markers change rapidly (e.g. Monaco's own internal type-checking passes).
        let markerDebounceTimer: ReturnType<typeof setTimeout> | null = null;
        let pendingUris: any[] = [];

        const flushMarkerUpdates = () => {
            const urisBatch = pendingUris.splice(0);
            const processedPaths = new Set<string>();
            urisBatch.forEach((uri: any) => {
                const markers = monaco.editor.getModelMarkers({ resource: uri });
                const errorCount = markers.filter((m: any) => m.severity === monaco.MarkerSeverity.Error).length;
                let rawPath = uri.fsPath || uri.path || '';
                if (rawPath.startsWith('/') && rawPath.includes(':')) rawPath = rawPath.substring(1);
                const normPath = rawPath.replace(/\\/g, '/').toLowerCase();
                if (!processedPaths.has(normPath)) {
                    processedPaths.add(normPath);
                    if (onValidation) onValidation(rawPath, errorCount);
                }
            });
        };

        const markerListener = monaco.editor.onDidChangeMarkers((uris: readonly any[]) => {
            pendingUris.push(...uris);
            if (markerDebounceTimer) clearTimeout(markerDebounceTimer);
            markerDebounceTimer = setTimeout(flushMarkerUpdates, 300);
        });

        // Register the command (globally)
        const command = (monaco.editor as any).registerCommand('whizcode.fixError', (_accessor: any, ...args: any[]) => {
            const [filePath, line, message] = args;
            if (onFixErrorRef.current) {
                onFixErrorRef.current(filePath, line, message);
            }
        });

        return () => {
            if (markerDebounceTimer) clearTimeout(markerDebounceTimer);
            provider.dispose();
            markerListener.dispose();
            if (command && typeof command.dispose === 'function') {
                command.dispose();
            }
        };
    }, [monaco, onValidation]);

    const handleEditorChange = (newContent: string | undefined) => {
        if (newContent !== undefined) {
            lastContentRef.current = newContent;
        }
        handleContentChange(newContent);
    };

    const handleEditorMount = (editor: any) => {
        editorRef.current = editor;
        setEditorInstance(editor);
        lastContentRef.current = activeFile?.content || '';
    };

    return (
        <main
            className="main-area"
            style={{
                display: 'flex',
                flexDirection: 'column',
                position: 'relative',
                flex: '1 1 auto',
                minHeight: 0,
                height: 'auto',
                width: '100%',
            }}
        >
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
                        <div style={{ display: 'flex', alignItems: 'center' }}>
                            <span style={{ opacity: 0.6 }}>WhizCode</span> 
                            <span style={{ opacity: 0.3, margin: '0 8px' }}>/</span> 
                            <span style={{ color: 'var(--accent-primary)', fontWeight: 500 }}>{activeFile?.name}</span>
                            <span style={{ opacity: 0.5, marginLeft: '8px', fontSize: '11px' }}>
                                {activeFileId?.replace(workspacePath || '', '').replace(activeFile?.name || '', '')}
                            </span>
                        </div>
                        
                        {isMarkdown && (
                            <div style={{ display: 'flex', gap: '8px', marginRight: '16px' }}>
                                <button 
                                    onClick={() => setIsPreviewEnabled(!isPreviewEnabled)}
                                    style={{
                                        backgroundColor: isPreviewEnabled ? 'var(--accent-primary)' : 'transparent',
                                        color: isPreviewEnabled ? 'white' : 'var(--text-secondary)',
                                        border: '1px solid ' + (isPreviewEnabled ? 'transparent' : 'var(--border-color)'),
                                        padding: '4px 10px',
                                        borderRadius: '4px',
                                        fontSize: '11px',
                                        cursor: 'pointer',
                                        display: 'flex',
                                        alignItems: 'center',
                                        gap: '4px',
                                        transition: 'all 0.2s ease'
                                    }}
                                >
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>
                                    {isPreviewEnabled ? 'Show Code' : 'Preview Markdown'}
                                </button>
                            </div>
                        )}
                    </div>

                    <div style={{ flex: 1, overflow: 'hidden', backgroundColor: '#1e1e1e', margin: '0' }}>
                        {isPreviewEnabled && activeFile ? (
                            <MarkdownRenderer content={activeFile.content} />
                        ) : (
                            <Editor
                                height="100%"
                                language={getLanguage(activeFile?.name || '')}
                                theme="vs-dark"
                                path={activeFileId || undefined}
                                value={activeFile?.content || ''}
                                onChange={handleEditorChange}
                                onMount={handleEditorMount}
                                options={{
                                    minimap: { enabled: false },
                                    fontSize: 14,
                                    wordWrap: 'on',
                                    fontFamily: "'Consolas', 'Courier New', monospace",
                                    hover: { enabled: true },
                                    quickSuggestions: true,
                                }}
                            />
                        )}
                    </div>
                </>
            ) : (
                <div className="welcome-screen">
                    <div className="welcome-content">
                <div className="welcome-icon">
                    <WhizLogo size={32} showText={true} centered={true} style={{ marginBottom: '10px' }} />
                </div>
                <h1 className="welcome-title" style={{ display: 'none' }}>WhizCode</h1>
                <p className="welcome-subtitle">A powerful local-first AI coding IDE</p>
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
