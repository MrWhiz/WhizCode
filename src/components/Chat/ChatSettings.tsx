import type { AIProvider } from '../../types'

interface ChatSettingsProps {
    isSettingsOpen: boolean;
    setIsSettingsOpen: (open: boolean) => void;
    aiProvider: AIProvider;
    setAiProvider: (provider: AIProvider) => void;
    ollamaModel: string;
    setOllamaModel: (model: string) => void;
    ollamaModels: string[];
    ollamaChecking: boolean;
    ollamaError: string | null;
    refreshOllamaModels: () => void;
    openaiKey: string;
    setOpenaiKey: (key: string) => void;
    geminiKey: string;
    setGeminiKey: (key: string) => void;
}

export const ChatSettings = ({
    isSettingsOpen,
    setIsSettingsOpen,
    aiProvider,
    setAiProvider,
    ollamaModel,
    setOllamaModel,
    ollamaModels,
    ollamaChecking,
    ollamaError,
    refreshOllamaModels,
    openaiKey,
    setOpenaiKey,
    geminiKey,
    setGeminiKey
}: ChatSettingsProps) => {
    return (
        <div className="chat-settings-section">
            <div className="chat-settings-header" onClick={() => setIsSettingsOpen(!isSettingsOpen)}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ transform: isSettingsOpen ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.15s' }}>
                        <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <circle cx="12" cy="12" r="3"></circle>
                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                    </svg>
                    <span>AI Settings</span>
                </div>
                <span className="settings-badge">{aiProvider === 'ollama' ? `Ollama · ${ollamaModel}` : aiProvider === 'openai' ? 'OpenAI' : 'Gemini'}</span>
            </div>

            {isSettingsOpen && (
                <div className="chat-settings-body">
                    <label className="settings-label">Provider</label>
                    <div className="provider-selector-compact">
                        {[
                            { id: 'ollama' as const, name: 'Ollama', icon: '🦙' },
                            { id: 'openai' as const, name: 'OpenAI', icon: '🤖' },
                            { id: 'gemini' as const, name: 'Gemini', icon: '✨' }
                        ].map(p => (
                            <button
                                key={p.id}
                                className={`provider-btn ${aiProvider === p.id ? 'active' : ''}`}
                                onClick={() => setAiProvider(p.id)}
                            >
                                <span>{p.icon}</span> {p.name}
                            </button>
                        ))}
                    </div>

                    {aiProvider === 'ollama' && (
                        <div className="settings-provider-config">
                            <div className="settings-hint-box">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ flexShrink: 0, marginTop: 2 }}>
                                    <circle cx="12" cy="12" r="10"></circle>
                                    <line x1="12" y1="16" x2="12" y2="12"></line>
                                    <line x1="12" y1="8" x2="12.01" y2="8"></line>
                                </svg>
                                <div>
                                    <strong>Setup:</strong> Install from <span className="settings-link">ollama.com</span>, then run <code>ollama pull llama3</code>
                                </div>
                            </div>

                            <div className="ollama-status-bar">
                                <div className="ollama-status-indicator">
                                    {ollamaChecking ? (
                                        <><div className="spinner" style={{ width: 10, height: 10 }}></div><span style={{ color: 'var(--text-secondary)' }}>Checking...</span></>
                                    ) : ollamaError ? (
                                        <><div className="status-dot offline"></div><span style={{ color: '#f14c4c', fontSize: 11 }}>{ollamaError}</span></>
                                    ) : ollamaModels.length > 0 ? (
                                        <><div className="status-dot online"></div><span style={{ color: '#89d185' }}>Running · {ollamaModels.length} models</span></>
                                    ) : (
                                        <><div className="status-dot offline"></div><span style={{ color: 'var(--text-secondary)' }}>Not checked</span></>
                                    )}
                                </div>
                                <button className="settings-btn-secondary" onClick={refreshOllamaModels}>↻</button>
                            </div>

                            {ollamaModels.length > 0 && (
                                <div className="settings-field">
                                    <label className="settings-field-label">Model</label>
                                    <div className="model-list">
                                        {ollamaModels.map(m => (
                                            <div
                                                key={m}
                                                className={`model-item ${ollamaModel === m ? 'active' : ''}`}
                                                onClick={() => setOllamaModel(m)}
                                            >
                                                <span className="model-icon">🧠</span>
                                                <span className="model-name">{m}</span>
                                                {ollamaModel === m && (
                                                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="3" style={{ marginLeft: 'auto' }}>
                                                        <polyline points="20 6 9 17 4 12"></polyline>
                                                    </svg>
                                                )}
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    )}

                    {aiProvider === 'openai' && (
                        <div className="settings-provider-config">
                            <div className="settings-hint-box">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ flexShrink: 0, marginTop: 2 }}>
                                    <circle cx="12" cy="12" r="10"></circle>
                                    <line x1="12" y1="16" x2="12" y2="12"></line>
                                    <line x1="12" y1="8" x2="12.01" y2="8"></line>
                                </svg>
                                <div>
                                    Get API key from <span className="settings-link">platform.openai.com/api-keys</span>.
                                </div>
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">API Key</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={openaiKey}
                                    onChange={e => setOpenaiKey(e.target.value)}
                                    placeholder="sk-..."
                                    spellCheck={false}
                                />
                            </div>
                        </div>
                    )}

                    {aiProvider === 'gemini' && (
                        <div className="settings-provider-config">
                            <div className="settings-hint-box">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ flexShrink: 0, marginTop: 2 }}>
                                    <circle cx="12" cy="12" r="10"></circle>
                                    <line x1="12" y1="16" x2="12" y2="12"></line>
                                    <line x1="12" y1="8" x2="12.01" y2="8"></line>
                                </svg>
                                <div>
                                    Get API key from <span className="settings-link">aistudio.google.com/apikey</span>.
                                </div>
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">API Key</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={geminiKey}
                                    onChange={e => setGeminiKey(e.target.value)}
                                    placeholder="AIzaSy..."
                                    spellCheck={false}
                                />
                            </div>
                        </div>
                    )}
                </div>
            )}
        </div>
    )
}
