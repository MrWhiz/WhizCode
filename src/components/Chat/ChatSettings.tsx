import type { AIProvider } from '../../types'

interface ChatSettingsProps {
    isSettingsOpen: boolean;
    setIsSettingsOpen: (open: boolean) => void;
    plannerProvider: AIProvider;
    setPlannerProvider: (provider: AIProvider) => void;
    plannerModel: string;
    setPlannerModel: (model: string) => void;
    executorProvider: AIProvider;
    setExecutorProvider: (provider: AIProvider) => void;
    executorModel: string;
    setExecutorModel: (model: string) => void;
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
    plannerProvider,
    setPlannerProvider,
    plannerModel,
    setPlannerModel,
    executorProvider,
    setExecutorProvider,
    executorModel,
    setExecutorModel,
    ollamaModels,
    ollamaChecking,
    ollamaError,
    refreshOllamaModels,
    openaiKey,
    setOpenaiKey,
    geminiKey,
    setGeminiKey
}: ChatSettingsProps) => {

    const providers = [
        { id: 'ollama' as const, name: 'Ollama', icon: '🦙' },
        { id: 'openai' as const, name: 'OpenAI', icon: '🤖' },
        { id: 'gemini' as const, name: 'Gemini', icon: '✨' }
    ];

    const renderModelSelector = (
        type: 'Planner' | 'Executor',
        provider: AIProvider,
        setProvider: (p: AIProvider) => void,
        model: string,
        setModel: (m: string) => void
    ) => (
        <div className="settings-model-group">
            <div className="settings-group-title">{type} Model</div>
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
                    {renderModelSelector('Planner', plannerProvider, setPlannerProvider, plannerModel, setPlannerModel)}

                    <div className="settings-separator" />

                    {renderModelSelector('Executor', executorProvider, setExecutorProvider, executorModel, setExecutorModel)}

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
