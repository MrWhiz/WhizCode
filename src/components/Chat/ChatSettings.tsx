import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { ollama } from '../../lib/tauri-api'
import type { AIProvider } from '../../types'

const RECOMMENDED_TASK_MODEL = 'qwen3:latest'
const RECOMMENDED_EMBED_MODEL = 'nomic-embed-text'

interface ChatSettingsProps {
    isSettingsOpen: boolean;
    setIsSettingsOpen: (open: boolean) => void;
    modelProvider: AIProvider;
    setModelProvider: (provider: AIProvider) => void;
    model: string;
    setModel: (model: string) => void;
    ollamaModels: string[];
    ollamaChecking: boolean;
    ollamaError: string | null;
    refreshOllamaModels: () => void;
    openaiKey: string;
    setOpenaiKey: (key: string) => void;
    geminiKey: string;
    setGeminiKey: (key: string) => void;
    bedrockRegion: string;
    setBedrockRegion: (region: string) => void;
    bedrockAccessKey: string;
    setBedrockAccessKey: (key: string) => void;
    bedrockSecretKey: string;
    setBedrockSecretKey: (key: string) => void;
    isAutopilotMode: boolean;
    setIsAutopilotMode: (mode: boolean) => void;
    azureLoginUrl: string;
    setAzureLoginUrl: (url: string) => void;
    azureEmbeddingUrl: string;
    setAzureEmbeddingUrl: (url: string) => void;
    azureCompletionUrl: string;
    setAzureCompletionUrl: (url: string) => void;
    azureUsername: string;
    setAzureUsername: (name: string) => void;
    azurePassword: string;
    setAzurePassword: (pass: string) => void;
    azureTokenStatus: { hasToken: boolean; timeLeft?: number; expires?: number };
    onGenerateAzureToken: () => void;
    contextLength: number;
    setContextLength: (length: number) => void;
}

export const ChatSettings = ({
    isSettingsOpen,
    setIsSettingsOpen,
    modelProvider,
    setModelProvider,
    model,
    setModel,
    ollamaModels,
    ollamaChecking,
    ollamaError,
    refreshOllamaModels,
    openaiKey,
    setOpenaiKey,
    geminiKey,
    setGeminiKey,
    bedrockRegion,
    setBedrockRegion,
    bedrockAccessKey,
    setBedrockAccessKey,
    bedrockSecretKey,
    setBedrockSecretKey,
    isAutopilotMode,
    setIsAutopilotMode,
    azureLoginUrl,
    setAzureLoginUrl,
    azureEmbeddingUrl,
    setAzureEmbeddingUrl,
    azureCompletionUrl,
    setAzureCompletionUrl,
    azureUsername,
    setAzureUsername,
    azurePassword,
    setAzurePassword,
    azureTokenStatus,
    onGenerateAzureToken,
    contextLength,
    setContextLength
}: ChatSettingsProps) => {

    const [pullingModels, setPullingModels] = useState<Record<string, { status: string; progress?: number }>>({})

    useEffect(() => {
        let unlisten: (() => void) | null = null
        listen<{ model: string; status: string; completed?: number; total?: number }>('ollama:pull_progress', (event) => {
            const { model: m, status, completed, total } = event.payload
            setPullingModels(prev => {
                if (status === 'done' || status.startsWith('error')) {
                    const next = { ...prev }
                    delete next[m]
                    return next
                }
                const progress = (completed && total && total > 0) ? Math.round((completed / total) * 100) : undefined
                return { ...prev, [m]: { status, progress } }
            })
        }).then(fn => { unlisten = fn })
        return () => { if (unlisten) unlisten() }
    }, [])

    const handlePullModel = async (modelName: string) => {
        setPullingModels(prev => ({ ...prev, [modelName]: { status: 'starting...' } }))
        try {
            await ollama.pullModel(modelName)
        } catch (e) {
            setPullingModels(prev => {
                const next = { ...prev }
                delete next[modelName]
                return next
            })
        }
    }

    const providers = [
        { id: 'ollama' as const, name: 'Ollama', icon: '🦙', description: 'Local models' },
        { id: 'openai' as const, name: 'OpenAI', icon: '🤖', description: 'GPT models' },
        { id: 'gemini' as const, name: 'Gemini', icon: '✨', description: 'Google AI' },
        { id: 'bedrock' as const, name: 'AWS Bedrock', icon: '☁️', description: 'AWS managed models' },
        { id: 'azure-gateway' as const, name: 'Azure Gateway', icon: '🌐', description: 'Enterprise LLM Gateway' }
    ];

    const openaiModels = [
        'gpt-4o',
        'gpt-4o-mini',
        'gpt-4-turbo',
        'gpt-4-turbo-preview',
        'gpt-4',
        'gpt-3.5-turbo',
        'gpt-3.5-turbo-16k'
    ];

    const geminiModels = [
        'gemini-1.5-pro',
        'gemini-1.5-pro-002',
        'gemini-1.5-flash',
        'gemini-1.5-flash-002',
        'gemini-1.0-pro',
        'gemini-1.0-pro-001'
    ];

    const bedrockModels = [
        'anthropic.claude-3-5-sonnet-20241022-v2:0',
        'anthropic.claude-3-5-haiku-20241022-v1:0',
        'anthropic.claude-3-opus-20240229-v1:0',
        'anthropic.claude-3-sonnet-20240229-v1:0',
        'anthropic.claude-3-haiku-20240307-v1:0',
        'meta.llama3-2-90b-instruct-v1:0',
        'meta.llama3-2-11b-instruct-v1:0',
        'meta.llama3-2-3b-instruct-v1:0',
        'meta.llama3-2-1b-instruct-v1:0'
    ];

    const bedrockRegions = [
        'us-east-1',
        'us-west-2',
        'eu-west-1',
        'eu-central-1',
        'ap-southeast-1',
        'ap-northeast-1'
    ];

    const getAvailableModels = () => {
        switch (modelProvider) {
            case 'ollama':
                return ollamaModels;
            case 'openai':
                return openaiModels;
            case 'gemini':
                return geminiModels;
            case 'bedrock':
                return bedrockModels;
            case 'azure-gateway':
                return ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo'];
            default:
                return [];
        }
    };

    const handleProviderChange = (newProvider: AIProvider) => {
        setModelProvider(newProvider);
        // Set default model for the provider if current model is not compatible
        const availableModels = newProvider === 'ollama' ? ollamaModels : 
                               newProvider === 'openai' ? openaiModels :
                               newProvider === 'gemini' ? geminiModels :
                               bedrockModels;
        
        // Only change model if current model is not in the new provider's list
        // This allows users to keep custom models when switching providers
        if (newProvider === 'ollama') {
            // For Ollama, always use a model from the available list
            if (availableModels.length > 0) {
                setModel(availableModels[0]);
            }
        } else {
            // For other providers, only change if current model is not in common list
            if (availableModels.length > 0 && !model) {
                setModel(availableModels[0]);
            }
        }
    };

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
                    <span className="settings-badge">Single Model</span>
                )}
            </div>

            {isSettingsOpen && (
                <div className="chat-settings-body">
                    {/* Provider Selection */}
                    <div className="settings-model-group">
                        <div className="settings-group-title">AI Provider</div>
                        <div className="settings-group-description">Choose your AI model provider</div>
                        
                        <div className="settings-field">
                            <select 
                                className="settings-select"
                                value={modelProvider}
                                onChange={(e) => handleProviderChange(e.target.value as AIProvider)}
                            >
                                {providers.map(p => (
                                    <option key={p.id} value={p.id}>
                                        {p.icon} {p.name} - {p.description}
                                    </option>
                                ))}
                            </select>
                        </div>
                    </div>

                    <div className="settings-separator" />

                    {/* Context Length - Only for Ollama */}
                    {modelProvider === 'ollama' && (
                        <>
                            <div className="settings-model-group">
                                <div className="settings-group-title" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ opacity: 0.8 }}>
                                        <circle cx="12" cy="12" r="3"></circle>
                                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                                    </svg>
                                    Context length
                                </div>
                                <div className="settings-group-description">
                                    Context length determines how much of your conversation local LLMs can remember and use to generate responses.
                                </div>
                                
                                <div className="settings-field" style={{ padding: '0 8px' }}>
                                    <div className="context-slider-container">
                                        <input 
                                            type="range"
                                            min="0"
                                            max="6"
                                            step="1"
                                            className="settings-range"
                                            value={[4096, 8192, 16384, 32768, 65536, 131072, 262144].indexOf(contextLength)}
                                            onChange={(e) => setContextLength([4096, 8192, 16384, 32768, 65536, 131072, 262144][Number(e.target.value)])}
                                            style={{ position: 'relative', zIndex: 10 }}
                                        />
                                        <div style={{ position: 'absolute', top: 32, width: '100%', left: 0 }}>
                                            {[4096, 8192, 16384, 32768, 65536, 131072, 262144].map((val, idx, arr) => {
                                                const labels = ['4k', '8k', '16k', '32k', '64k', '128k', '256k'];
                                                const isActive = contextLength === val;
                                                return (
                                                    <div 
                                                        key={val} 
                                                        className={`context-tick ${isActive ? 'active' : ''}`}
                                                        onClick={() => setContextLength(val)}
                                                        style={{ 
                                                            position: 'absolute', 
                                                            left: `${(idx / (arr.length - 1)) * 100}%`,
                                                            transform: 'translateX(-50%)',
                                                            display: 'flex',
                                                            flexDirection: 'column',
                                                            alignItems: 'center',
                                                            cursor: 'pointer'
                                                        }}
                                                    >
                                                        <div className="tick-mark" style={{ 
                                                            width: 1, 
                                                            height: 4, 
                                                            background: 'var(--border-color)', 
                                                            marginBottom: 8 
                                                        }}></div>
                                                        <span style={{ 
                                                            fontSize: '10px', 
                                                            opacity: isActive ? 1 : 0.5,
                                                            fontWeight: isActive ? 600 : 400,
                                                            color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)'
                                                        }}>{labels[idx]}</span>
                                                    </div>
                                                )
                                            })}
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <div className="settings-separator" />
                        </>
                    )}

                    {/* Model Selection */}
                    <div className="settings-model-group">
                        <div className="settings-group-title">Model</div>
                        <div className="settings-group-description">
                            {modelProvider === 'ollama' && 'Select from locally available models'}
                            {modelProvider === 'openai' && 'Choose from common OpenAI models or enter a custom model name'}
                            {modelProvider === 'gemini' && 'Choose from common Gemini models or enter a custom model name'}
                            {modelProvider === 'bedrock' && 'Choose from common Bedrock models or enter a custom model ARN/ID'}
                        </div>
                        
                        {modelProvider === 'ollama' && (
                            <div className="settings-field">
                                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                                    <div className="ollama-status-mini">
                                        {ollamaChecking ? (
                                            <div className="spinner" style={{ width: 12, height: 12 }}></div>
                                        ) : ollamaError ? (
                                            <span style={{ color: '#f14c4c' }}>📴 Offline</span>
                                        ) : (
                                            <span style={{ color: '#89d185' }}>✅ Online ({ollamaModels.length} models)</span>
                                        )}
                                    </div>
                                    <button 
                                        className="refresh-btn-mini" 
                                        onClick={refreshOllamaModels}
                                        disabled={ollamaChecking}
                                        title="Refresh Ollama models"
                                    >
                                        {ollamaChecking ? '⏳' : '↻'}
                                    </button>
                                </div>
                                
                                {ollamaModels.length > 0 ? (
                                    <select 
                                        className="settings-select"
                                        value={model}
                                        onChange={(e) => setModel(e.target.value)}
                                    >
                                        {ollamaModels.map(m => (
                                            <option key={m} value={m}>{m}</option>
                                        ))}
                                    </select>
                                ) : (
                                    <div className="settings-field-description" style={{ color: '#f14c4c', padding: '8px', background: 'var(--bg-secondary)', borderRadius: '4px', border: '1px solid #f14c4c' }}>
                                        {ollamaError ? (
                                            <>
                                                <div style={{ fontWeight: 'bold', marginBottom: '4px' }}>❌ {ollamaError}</div>
                                                <div style={{ fontSize: '10px', opacity: 0.8 }}>
                                                    To fix this:
                                                    <br />1. Install Ollama from <a href="https://ollama.ai" target="_blank" rel="noopener noreferrer">ollama.ai</a>
                                                    <br />2. Run: <code>ollama serve</code>
                                                    <br />3. Install a model: <code>ollama pull qwen2.5-coder:latest</code>
                                                </div>
                                            </>
                                        ) : (
                                            'No models found. Install models with: ollama pull <model-name>'
                                        )}
                                    </div>
                                )}

                                {/* Recommended Models */}
                                <div style={{ marginTop: 12 }}>
                                    <div style={{ fontSize: '11px', opacity: 0.6, marginBottom: 6, textTransform: 'uppercase', letterSpacing: '0.05em' }}>Recommended Models</div>
                                    {[
                                        { name: RECOMMENDED_TASK_MODEL, label: 'Task Model', desc: 'Reasoning & coding' },
                                        { name: RECOMMENDED_EMBED_MODEL, label: 'Embedding Model', desc: 'Vector search & indexing' },
                                    ].map(({ name, label, desc }) => {
                                        const isInstalled = ollamaModels.some(m => m === name || m.startsWith(name.split(':')[0]))
                                        const pulling = pullingModels[name]
                                        return (
                                            <div key={name} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '5px 8px', marginBottom: 4, background: 'var(--bg-secondary)', borderRadius: 4, border: '1px solid var(--border-color)' }}>
                                                <div>
                                                    <span style={{ fontSize: '12px', fontFamily: 'monospace' }}>{name}</span>
                                                    <span style={{ fontSize: '10px', opacity: 0.5, marginLeft: 6 }}>{label} · {desc}</span>
                                                </div>
                                                {isInstalled ? (
                                                    <span style={{ fontSize: '10px', color: '#89d185' }}>✓ installed</span>
                                                ) : pulling ? (
                                                    <span style={{ fontSize: '10px', opacity: 0.7 }}>
                                                        {pulling.progress !== undefined ? `${pulling.progress}%` : pulling.status}
                                                    </span>
                                                ) : (
                                                    <button
                                                        onClick={() => handlePullModel(name)}
                                                        style={{ fontSize: '10px', padding: '2px 8px', background: 'var(--accent-primary)', color: '#fff', border: 'none', borderRadius: 3, cursor: 'pointer' }}
                                                    >
                                                        Pull
                                                    </button>
                                                )}
                                            </div>
                                        )
                                    })}
                                </div>
                            </div>
                        )}

                        {modelProvider === 'openai' && (
                            <div className="settings-field">
                                <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8 }}>
                                    <select 
                                        className="settings-select"
                                        value={openaiModels.includes(model) ? model : 'custom'}
                                        onChange={(e) => {
                                            if (e.target.value !== 'custom') {
                                                setModel(e.target.value);
                                            } else {
                                                setModel(''); // Clear the model to show custom input
                                            }
                                        }}
                                        style={{ flex: 1 }}
                                    >
                                        {openaiModels.map(m => (
                                            <option key={m} value={m}>{m}</option>
                                        ))}
                                        <option value="custom">Custom model...</option>
                                    </select>
                                </div>
                                {!openaiModels.includes(model) && (
                                    <>
                                        <input
                                            type="text"
                                            className="settings-input"
                                            value={model}
                                            onChange={(e) => setModel(e.target.value)}
                                            placeholder="Enter custom model name (e.g., gpt-4o-mini)"
                                            style={{ marginTop: 8 }}
                                        />
                                        <div className="settings-field-description" style={{ marginTop: 4 }}>
                                            Enter any OpenAI model name. See <a href="https://platform.openai.com/docs/models" target="_blank" rel="noopener noreferrer">OpenAI docs</a> for available models.
                                        </div>
                                    </>
                                )}
                            </div>
                        )}

                        {modelProvider === 'gemini' && (
                            <div className="settings-field">
                                <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8 }}>
                                    <select 
                                        className="settings-select"
                                        value={geminiModels.includes(model) ? model : 'custom'}
                                        onChange={(e) => {
                                            if (e.target.value !== 'custom') {
                                                setModel(e.target.value);
                                            } else {
                                                setModel(''); // Clear the model to show custom input
                                            }
                                        }}
                                        style={{ flex: 1 }}
                                    >
                                        {geminiModels.map(m => (
                                            <option key={m} value={m}>{m}</option>
                                        ))}
                                        <option value="custom">Custom model...</option>
                                    </select>
                                </div>
                                {!geminiModels.includes(model) && (
                                    <>
                                        <input
                                            type="text"
                                            className="settings-input"
                                            value={model}
                                            onChange={(e) => setModel(e.target.value)}
                                            placeholder="Enter custom model name (e.g., gemini-1.5-pro-002)"
                                            style={{ marginTop: 8 }}
                                        />
                                        <div className="settings-field-description" style={{ marginTop: 4 }}>
                                            Enter any Gemini model name. See <a href="https://ai.google.dev/gemini-api/docs/models/gemini" target="_blank" rel="noopener noreferrer">Gemini docs</a> for available models.
                                        </div>
                                    </>
                                )}
                            </div>
                        )}

                        {modelProvider === 'bedrock' && (
                            <div className="settings-field">
                                <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8 }}>
                                    <select 
                                        className="settings-select"
                                        value={bedrockModels.includes(model) ? model : 'custom'}
                                        onChange={(e) => {
                                            if (e.target.value !== 'custom') {
                                                setModel(e.target.value);
                                            } else {
                                                setModel(''); // Clear the model to show custom input
                                            }
                                        }}
                                        style={{ flex: 1 }}
                                    >
                                        {bedrockModels.map(m => (
                                            <option key={m} value={m}>{m}</option>
                                        ))}
                                        <option value="custom">Custom model...</option>
                                    </select>
                                </div>
                                {!bedrockModels.includes(model) && (
                                    <>
                                        <input
                                            type="text"
                                            className="settings-input"
                                            value={model}
                                            onChange={(e) => setModel(e.target.value)}
                                            placeholder="Enter custom model ARN or ID (e.g., anthropic.claude-3-5-sonnet-20241022-v2:0)"
                                            style={{ marginTop: 8 }}
                                        />
                                        <div className="settings-field-description" style={{ marginTop: 4 }}>
                                            Enter any Bedrock model ID or ARN. See <a href="https://docs.aws.amazon.com/bedrock/latest/userguide/model-ids.html" target="_blank" rel="noopener noreferrer">AWS docs</a> for available models.
                                        </div>
                                    </>
                                )}
                            </div>
                        )}
                    </div>

                    <div className="settings-separator" />

                    {/* Provider-specific Configuration */}
                    {modelProvider === 'openai' && (
                        <>
                            <div className="settings-section-title">OpenAI Configuration</div>
                            <div className="settings-field">
                                <label className="settings-field-label">API Key</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={openaiKey}
                                    onChange={e => setOpenaiKey(e.target.value)}
                                    placeholder="sk-..."
                                />
                                <div className="settings-field-description">
                                    Get your API key from <a href="https://platform.openai.com/api-keys" target="_blank" rel="noopener noreferrer">OpenAI Platform</a>
                                </div>
                            </div>
                            <div className="settings-separator" />
                        </>
                    )}

                    {modelProvider === 'gemini' && (
                        <>
                            <div className="settings-section-title">Gemini Configuration</div>
                            <div className="settings-field">
                                <label className="settings-field-label">API Key</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={geminiKey}
                                    onChange={e => setGeminiKey(e.target.value)}
                                    placeholder="AIzaSy..."
                                />
                                <div className="settings-field-description">
                                    Get your API key from <a href="https://makersuite.google.com/app/apikey" target="_blank" rel="noopener noreferrer">Google AI Studio</a>
                                </div>
                            </div>
                            <div className="settings-separator" />
                        </>
                    )}

                    {modelProvider === 'bedrock' && (
                        <>
                            <div className="settings-section-title">AWS Bedrock Configuration</div>
                            <div className="settings-field">
                                <label className="settings-field-label">Region</label>
                                <select 
                                    className="settings-select"
                                    value={bedrockRegion}
                                    onChange={(e) => setBedrockRegion(e.target.value)}
                                >
                                    {bedrockRegions.map(region => (
                                        <option key={region} value={region}>{region}</option>
                                    ))}
                                </select>
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Access Key ID</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={bedrockAccessKey}
                                    onChange={e => setBedrockAccessKey(e.target.value)}
                                    placeholder="AKIA..."
                                />
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Secret Access Key</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={bedrockSecretKey}
                                    onChange={e => setBedrockSecretKey(e.target.value)}
                                    placeholder="..."
                                />
                            </div>
                            <div className="settings-field-description">
                                Configure AWS credentials with Bedrock access. See <a href="https://docs.aws.amazon.com/bedrock/latest/userguide/security_iam_service-with-iam.html" target="_blank" rel="noopener noreferrer">AWS documentation</a>
                            </div>
                            <div className="settings-separator" />
                        </>
                    )}
                    
                    {modelProvider === 'azure-gateway' && (
                        <>
                            <div className="settings-section-title">Azure Gateway Configuration</div>
                            <div className="settings-field">
                                <label className="settings-field-label">Login URL</label>
                                <input
                                    type="text"
                                    className="settings-input"
                                    value={azureLoginUrl}
                                    onChange={e => setAzureLoginUrl(e.target.value)}
                                    placeholder="https://..."
                                />
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Embedding URL</label>
                                <input
                                    type="text"
                                    className="settings-input"
                                    value={azureEmbeddingUrl}
                                    onChange={e => setAzureEmbeddingUrl(e.target.value)}
                                    placeholder="https://..."
                                />
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Chat Completion URL</label>
                                <input
                                    type="text"
                                    className="settings-input"
                                    value={azureCompletionUrl}
                                    onChange={e => setAzureCompletionUrl(e.target.value)}
                                    placeholder="https://..."
                                />
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Username</label>
                                <input
                                    type="text"
                                    className="settings-input"
                                    value={azureUsername}
                                    onChange={e => setAzureUsername(e.target.value)}
                                    placeholder="Domain\Username"
                                />
                            </div>
                            <div className="settings-field">
                                <label className="settings-field-label">Password</label>
                                <input
                                    type="password"
                                    className="settings-input"
                                    value={azurePassword}
                                    onChange={e => setAzurePassword(e.target.value)}
                                    placeholder="..."
                                />
                            </div>
                            <div className="settings-field">
                                <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 8 }}>
                                    <button 
                                        className={`settings-btn ${!azureLoginUrl || !azureUsername || !azurePassword ? 'disabled' : ''}`}
                                        onClick={onGenerateAzureToken}
                                        disabled={!azureLoginUrl || !azureUsername || !azurePassword}
                                        style={{
                                            padding: '8px 16px',
                                            borderRadius: '6px',
                                            background: 'var(--accent-primary)',
                                            color: 'white',
                                            border: 'none',
                                            cursor: 'pointer',
                                            fontWeight: 600,
                                            fontSize: '11px',
                                            letterSpacing: '0.5px'
                                        }}
                                    >
                                        GENERATE SESSION TOKEN
                                    </button>

                                    <div className="azure-token-status">
                                        {azureTokenStatus.hasToken ? (
                                            <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: '11px' }}>
                                                <span style={{ color: '#89d185' }}>● Valid</span>
                                                <span style={{ opacity: 0.6 }}>(Exp: ~{azureTokenStatus.timeLeft}h)</span>
                                            </div>
                                        ) : (
                                            <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: '11px', opacity: 0.7 }}>
                                                <span style={{ color: '#cca700' }}>● No Active Token</span>
                                            </div>
                                        )}
                                    </div>
                                </div>
                            </div>
                            <div className="settings-field-description">
                                The login URL will be used to generate a session token valid for ~24 hours. The token is stored securely for all future requests.
                            </div>
                            <div className="settings-separator" />
                        </>
                    )}

                    {/* Agent Mode */}

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
                </div>
            )}
        </div>
    )
}
