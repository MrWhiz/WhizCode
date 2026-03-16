import type { AIProvider } from '../../types'

interface ChatSettingsProps {
    isSettingsOpen: boolean;
    setIsSettingsOpen: (open: boolean) => void;
    primaryModelProvider: AIProvider;
    setPrimaryModelProvider: (provider: AIProvider) => void;
    primaryModel: string;
    setPrimaryModel: (model: string) => void;
    toolModelProvider: AIProvider;
    setToolModelProvider: (provider: AIProvider) => void;
    toolModel: string;
    setToolModel: (model: string) => void;
    ollamaModels: string[];
    ollamaChecking: boolean;
    ollamaError: string | null;
    refreshOllamaModels: () => void;
    openaiKey: string;
    setOpenaiKey: (key: string) => void;
    geminiKey: string;
    setGeminiKey: (key: string) => void;
    isAutopilotMode: boolean;
    setIsAutopilotMode: (mode: boolean) => void;
}

export const ChatSettings = ({
    isSettingsOpen,
    setIsSettingsOpen,
    primaryModelProvider,
    setPrimaryModelProvider,
    primaryModel,
    setPrimaryModel,
    toolModelProvider,
    setToolModelProvider,
    toolModel,
    setToolModel,
    ollamaModels,
    ollamaChecking,
    ollamaError,
    refreshOllamaModels,
    openaiKey,
    setOpenaiKey,
    geminiKey,
    setGeminiKey,
    isAutopilotMode,
    setIsAutopilotMode
}: ChatSettingsProps) => {

    const providers = [
        { id: 'ollama' as const, name: 'Ollama', icon: '🦙' },
        { id: 'openai' as const, name: 'OpenAI', icon: '🤖' },
        { id: 'gemini' as const, name: 'Gemini', icon: '✨' }
    ];

    const renderModelSelector = (
        type: 'Primary' | 'Tool',
        provider: AIProvider,
        setProvider: (p: AIProvider) => void,
        model: string,
        setModel: (m: string) => void,
        description: string
    ) => (
        <div className="settings-model-group">
            <div className="settings-group-title">{type} Model</div>
            <div className="settings-group-description">{description}</div>
            <div className="provider-selector-compact">
                {providers.map(p => (
                    <button
                        key={p.id}
                        className={`provider-btn ${provider === p.id ? 'active' : ''}`}
                        onClick={() => setProvider(p.id)}
                    >
                        <span>{p.icon}</span> {p.name}
                    </button>
                ))}
            </div>

            {provider === 'ollama' && ollamaModels.length > 0 && (
                <div className="model-list mini">
                    {ollamaModels.map(m => (
                        <div
                            key={m}
                            className={`model-item ${model === m ? 'active' : ''}`}
                            onClick={() => setModel(m)}
                        >
                            <span className="model-name">{m}</span>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );

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
                    <span>Agent Configuration</span>
                </div>
                {!isSettingsOpen && (
                    <span className="settings-badge">Multi-Model Active</span>
                )}
            </div>

            {isSettingsOpen && (
                <div className="chat-settings-body">
                    {renderModelSelector(
                        'Primary', 
                        primaryModelProvider, 
                        setPrimaryModelProvider, 
                        primaryModel, 
                        setPrimaryModel,
                        'For reasoning, planning, and decision-making'
                    )}

                    <div className="settings-separator" />

                    {renderModelSelector(
                        'Tool', 
                        toolModelProvider, 
                        setToolModelProvider, 
                        toolModel, 
                        setToolModel,
                        'For code generation and tool execution'
                    )}

                    <div className="settings-separator" />

                    <div className="settings-section-title">Agent Mode</div>
                    <div className="settings-field">
                        <label className="settings-field-label" style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
                            <input
                                type="checkbox"
                                checked={isAutopilotMode}
                                onChange={(e) => setIsAutopilotMode(e.target.checked)}
                                style={{ accentColor: 'var(--accent-primary)' }}
                            />
                            <span>Autopilot Mode</span>
                        </label>
                        <div className="settings-group-description" style={{ marginTop: 4, marginLeft: 24 }}>
                            {isAutopilotMode 
                                ? '🚀 Agent can modify files autonomously without approval'
                                : '🛡️ Agent will ask for approval before modifying files'}
                        </div>
                    </div>

                    <div className="settings-separator" />

                    <div className="settings-section-title">Credentials</div>
                    <div className="settings-field">
                        <label className="settings-field-label">OpenAI Key</label>
                        <input
                            type="password"
                            className="settings-input"
                            value={openaiKey}
                            onChange={e => setOpenaiKey(e.target.value)}
                            placeholder="sk-..."
                        />
                    </div>
                    <div className="settings-field">
                        <label className="settings-field-label">Gemini Key</label>
                        <input
                            type="password"
                            className="settings-input"
                            value={geminiKey}
                            onChange={e => setGeminiKey(e.target.value)}
                            placeholder="AIzaSy..."
                        />
                    </div>

                    <div className="ollama-status-mini">
                        {ollamaChecking ? (
                            <div className="spinner" style={{ width: 8, height: 8 }}></div>
                        ) : ollamaError ? (
                            <span style={{ color: '#f14c4c' }}>📴 Ollama Offline</span>
                        ) : (
                            <span style={{ color: '#89d185' }}>✅ Ollama Online</span>
                        )}
                        <button className="refresh-btn-mini" onClick={refreshOllamaModels}>↻</button>
                    </div>
                </div>
            )}
        </div>
    )
}
