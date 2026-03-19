import React from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'
import type { Message, AgentStep } from '../../types'
import { ChatSettings } from './ChatSettings'

interface ChatPanelProps {
    chatWidth: number;
    handleChatResize: (e: React.MouseEvent) => void;
    isChatOpen: boolean;
    setIsChatOpen: (open: boolean) => void;
    workspacePath: string | null;
    messages: Message[];
    isLoading: boolean;
    agentSteps: AgentStep[];
    input: string;
    setInput: (val: string) => void;
    handleSend: () => void;
    handleReset: () => void;
    handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
    getToolIcon: (tool: string) => string;
    messagesEndRef: React.RefObject<HTMLDivElement | null>;
    handlePermissionResponse: (approved: boolean, stepIdx?: number) => void;
    handleStop: () => void;
    // Settings props
    settingsProps: any;
}

const LogContainer = ({ logs }: { logs: string[] }) => {
    const logsEndRef = React.useRef<HTMLDivElement>(null);
    React.useEffect(() => {
        logsEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }, [logs]);

    return (
        <div className="agent-step-logs">
            {logs.map((log, li) => (
                <span key={li} className="log-line">{log}</span>
            ))}
            <div ref={logsEndRef} />
        </div>
    );
};

const StepBlock = ({ step, getToolIcon, isLive = false }: { step: AgentStep, getToolIcon: (t: string) => string, isLive?: boolean }) => {
    const [logsOpen, setLogsOpen] = React.useState(false);
    const hasLogs = step.logs && step.logs.length > 0;
    const canOpenLogs = step.tool === 'run_command' || hasLogs;

    // Auto-open logs if the step is a running command
    React.useEffect(() => {
        if (isLive && step.status === 'running' && step.tool === 'run_command') {
            setLogsOpen(true);
        }
    }, [isLive, step.status, step.tool]);

    const handleClick = () => {
        if (canOpenLogs) {
            setLogsOpen(o => !o);
        }
    };

    // Determine phase badge
    const getPhaseLabel = (phase?: string) => {
        switch (phase) {
            case 'planning': return '📋 PLANNING';
            case 'execution': return '⚙️ EXECUTION';
            case 'summary': return '📊 SUMMARY';
            default: return '';
        }
    };

    return (
        <div className={`agent-step ${step.status}`}>
            <div
                className="agent-step-header"
                onClick={handleClick}
                style={{ 
                    cursor: canOpenLogs ? 'pointer' : 'default',
                    userSelect: 'none'
                }}
            >
                {isLive && step.status === 'running' ? (
                    <div className="spinner" style={{ width: 10, height: 10 }}></div>
                ) : (
                    <span className="agent-step-icon">{getToolIcon(step.tool)}</span>
                )}
                <span className="agent-step-summary">{step.summary}</span>
                {step.planPhase && (
                    <span style={{ 
                        fontSize: '9px', 
                        opacity: 0.6, 
                        marginLeft: '8px',
                        padding: '2px 6px',
                        background: 'rgba(0,0,0,0.2)',
                        borderRadius: '3px'
                    }}>
                        {getPhaseLabel(step.planPhase)}
                    </span>
                )}
                {step.status === 'done' && <span className="agent-step-check">✓</span>}
                {canOpenLogs && (
                    <span style={{ marginLeft: 'auto', fontSize: '10px', opacity: 0.7, paddingLeft: 6 }}>
                        {logsOpen ? '▲' : '▼'}
                    </span>
                )}
            </div>
            {step.data && <EditDetails data={step.data} />}
            {canOpenLogs && logsOpen && <LogContainer logs={step.logs && step.logs.length > 0 ? step.logs : ['(No logs yet)']} />}
        </div>
    );
};

const EditDetails = ({ data }: { data: any }) => {
    const [isOpen, setIsOpen] = React.useState(false);

    if (!data) return null;

    // Handle plan data
    if (data.plan) {
        const plan = data.plan;
        return (
            <div className="agent-step-details" style={{ marginTop: '8px' }}>
                <div
                    className="details-toggle"
                    onClick={() => setIsOpen(!isOpen)}
                    style={{
                        fontSize: '11px',
                        color: 'var(--accent-primary)',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '4px',
                        userSelect: 'none',
                        fontWeight: 600
                    }}
                >
                    {isOpen ? '⊖ Hide Plan' : '⊕ View Plan'}
                </div>
                {isOpen && (
                    <div className="details-content" style={{
                        marginTop: '8px',
                        background: 'rgba(0,0,0,0.2)',
                        borderRadius: '4px',
                        overflow: 'hidden',
                        border: '1px solid var(--border-color)',
                        padding: '8px'
                    }}>
                        <div style={{ marginBottom: '8px' }}>
                            <div style={{ fontSize: '10px', color: 'var(--text-tertiary)', fontWeight: 600, textTransform: 'uppercase', marginBottom: '4px' }}>
                                Objective
                            </div>
                            <div style={{ fontSize: '11px', color: 'var(--text-primary)' }}>{plan.objective}</div>
                        </div>
                        
                        <div style={{ marginBottom: '8px' }}>
                            <div style={{ fontSize: '10px', color: 'var(--text-tertiary)', fontWeight: 600, textTransform: 'uppercase', marginBottom: '4px' }}>
                                Tasks ({plan.tasks?.length || 0})
                            </div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                                {plan.tasks?.map((task: any, i: number) => (
                                    <div key={i} style={{
                                        fontSize: '10px',
                                        padding: '4px 6px',
                                        background: 'rgba(0,0,0,0.3)',
                                        borderRadius: '3px',
                                        borderLeft: '2px solid var(--accent-primary)'
                                    }}>
                                        <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                                            {i + 1}. {task.description}
                                        </div>
                                        <div style={{ fontSize: '9px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                                            Type: {task.type} • Duration: ~{task.estimatedDuration}s
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>

                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', fontSize: '10px' }}>
                            <div>
                                <div style={{ color: 'var(--text-tertiary)', fontWeight: 600, textTransform: 'uppercase', marginBottom: '2px' }}>
                                    Duration
                                </div>
                                <div style={{ color: 'var(--text-primary)' }}>~{plan.estimatedDuration}s</div>
                            </div>
                            <div>
                                <div style={{ color: 'var(--text-tertiary)', fontWeight: 600, textTransform: 'uppercase', marginBottom: '2px' }}>
                                    Risk Level
                                </div>
                                <div style={{ 
                                    color: plan.riskLevel === 'high' ? '#ff6b6b' : plan.riskLevel === 'medium' ? '#ffa500' : '#51cf66'
                                }}>
                                    {plan.riskLevel?.toUpperCase()}
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        );
    }

    return (
        <div className="agent-step-details" style={{ marginTop: '8px' }}>
            <div
                className="details-toggle"
                onClick={() => setIsOpen(!isOpen)}
                style={{
                    fontSize: '11px',
                    color: 'var(--accent-primary)',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    userSelect: 'none',
                    fontWeight: 600
                }}
            >
                {isOpen ? '⊖ Hide Changes' : '⊕ View Changes'}
            </div>
            {isOpen && (
                <div className="details-content" style={{
                    marginTop: '8px',
                    background: 'rgba(0,0,0,0.2)',
                    borderRadius: '4px',
                    overflow: 'hidden',
                    border: '1px solid var(--border-color)'
                }}>
                    {data.edits ? (
                        data.edits.map((edit: any, i: number) => (
                            <div key={i} className="edit-block-preview" style={{ padding: '8px', borderBottom: i < data.edits.length - 1 ? '1px solid var(--border-color)' : 'none' }}>
                                <div style={{ fontSize: '10px', color: '#f14c4c', fontWeight: 600, marginBottom: '4px' }}>REMOVE</div>
                                <pre style={{ margin: '0 0 8px 0', fontSize: '11px', whiteSpace: 'pre-wrap', color: '#ff8888', background: 'rgba(241,76,76,0.05)', padding: '4px', borderRadius: '2px' }}>{edit.search}</pre>
                                <div style={{ fontSize: '10px', color: '#89d185', fontWeight: 600, marginBottom: '4px' }}>ADD</div>
                                <pre style={{ margin: 0, fontSize: '11px', whiteSpace: 'pre-wrap', color: '#89d185', background: 'rgba(137,209,133,0.05)', padding: '4px', borderRadius: '2px' }}>{edit.replace}</pre>
                            </div>
                        ))
                    ) : data.changes ? (
                        data.changes.map((change: any, i: number) => (
                            <div key={i} className="edit-block-preview" style={{ padding: '8px', borderBottom: i < data.changes.length - 1 ? '1px solid var(--border-color)' : 'none' }}>
                                <div style={{ fontSize: '11px', fontWeight: 600, marginBottom: '6px', color: 'var(--text-secondary)' }}>{change.path}</div>
                                <pre style={{ margin: 0, fontSize: '11px', whiteSpace: 'pre-wrap', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{change.diff}</pre>
                            </div>
                        ))
                    ) : null}
                </div>
            )}
        </div>
    );
};

const MessageContent = ({ content, role }: { content: string, role: string }) => {
    if (role !== 'assistant') return <>{content}</>;

    // 1. Extract and clean thought blocks
    const thoughtRegex = /<THOUGHT>([\s\S]*?)<\/THOUGHT>/gi;
    const thoughts: string[] = [];
    let match;
    let cleanContent = content;

    while ((match = thoughtRegex.exec(content)) !== null) {
        thoughts.push(match[1].trim());
    }

    // Normalize and strip all possible internal control tags
    cleanContent = content
        .replace(thoughtRegex, '')
        .replace(/<IDENTITY>[\s\S]*?<\/IDENTITY>/gi, '')
        .replace(/<PRIME_DIRECTIVE>[\s\S]*?<\/PRIME_DIRECTIVE>/gi, '')
        .replace(/<PLAN>[\s\S]*?<\/PLAN>/gi, '')
        .replace(/<PROJECT_STATUS>[\s\S]*?<\/PROJECT_STATUS>/gi, '')
        .replace(/<OUTPUT_FORMAT>[\s\S]*?<\/OUTPUT_FORMAT>/gi, '')
        .trim();

    // 2. Hide tool call JSONs - detect if content contains any tool call JSON patterns
    const toolCallPatterns = [
        '"tool":',
        '"file_path":',
        '"content":',
        '"function_calls"',
        '"invoke"',
        '"antml:function_calls"',
        '"write_file"',
        '"read_file"',
        '"edit_file"',
        '"run_command"',
        '"list_directory"'
    ];
    
    const hasToolCallJson = toolCallPatterns.some(pattern => cleanContent.includes(pattern)) ||
                           (cleanContent.includes('{') && cleanContent.includes('"tool"'));
    
    if (hasToolCallJson) {
        return null; // Hide tool call JSONs completely
    }

    // 3. Detect if the remaining content is just a JSON-like completion summary
    let isJsonSummary = false;
    if (cleanContent.startsWith('{') && cleanContent.endsWith('}') && cleanContent.includes('"status"')) {
        try {
            isJsonSummary = true;
        } catch (e) { }
    }

    return (
        <div className="assistant-content-wrapper">
            {thoughts.length > 0 && (
                <div className="thought-process" style={{
                    marginBottom: '12px',
                    background: 'rgba(0, 0, 0, 0.2)',
                    borderRadius: '6px',
                    borderLeft: '3px solid var(--accent-primary)',
                    padding: '8px 10px'
                }}>
                    <div className="thought-header" style={{
                        fontSize: '10px',
                        fontWeight: 700,
                        color: 'var(--text-tertiary)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.5px',
                        marginBottom: '4px',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '6px'
                    }}>
                        <span style={{ opacity: 0.6 }}>🧠</span> REASONING
                    </div>
                    {thoughts.map((t, i) => (
                        <div key={i} className="thought-body" style={{
                            fontSize: '11.5px',
                            color: 'var(--text-secondary)',
                            fontStyle: 'italic',
                            lineHeight: '1.4'
                        }}>{t}</div>
                    ))}
                </div>
            )}
            <div className="message-main-body">
                {isJsonSummary ? (
                    <SyntaxHighlighter
                        style={vscDarkPlus as any}
                        language="json"
                        PreTag="div"
                        customStyle={{
                            margin: '8px 0',
                            borderRadius: '6px',
                            fontSize: '12px',
                            border: '1px solid var(--border-color)',
                            background: 'rgba(0,0,0,0.3)'
                        }}
                    >
                        {cleanContent}
                    </SyntaxHighlighter>
                ) : (
                    <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                            code({ className, children, ...props }) {
                                const match = /language-(\w+)/.exec(className || '');
                                const codeString = String(children).replace(/\n$/, '');
                                return match ? (
                                    <SyntaxHighlighter
                                        style={vscDarkPlus as any}
                                        language={match[1]}
                                        PreTag="div"
                                        customStyle={{
                                            margin: '8px 0',
                                            borderRadius: '6px',
                                            fontSize: '12px',
                                            border: '1px solid var(--border-color)',
                                        }}
                                    >
                                        {codeString}
                                    </SyntaxHighlighter>
                                ) : (
                                    <code className="inline-code" {...props}>
                                        {children}
                                    </code>
                                );
                            },
                            a({ href, children }) {
                                return <a href={href} target="_blank" rel="noreferrer" className="md-link">{children}</a>;
                            },
                        }}
                    >
                        {cleanContent}
                    </ReactMarkdown>
                )}
            </div>
        </div>
    );
};

export const ChatPanel = ({
    chatWidth,
    handleChatResize,
    isChatOpen,
    setIsChatOpen,
    workspacePath,
    messages,
    isLoading,
    agentSteps,
    input,
    setInput,
    handleSend,
    handleReset,
    handleKeyDown,
    getToolIcon,
    messagesEndRef,
    handlePermissionResponse,
    handleStop,
    settingsProps
}: ChatPanelProps) => {
    const [respondedSteps, setRespondedSteps] = React.useState<Record<number, boolean>>({});
    const [alwaysRun, setAlwaysRun] = React.useState(false);
    const [countdown, setCountdown] = React.useState<number | null>(null);

    React.useEffect(() => {
        if (!isLoading) {
            setRespondedSteps({});
            setCountdown(null);
            // Reset alwaysRun on task completion? Or keep it? Usually better to reset for safety.
            // setAlwaysRun(false); 
        }
    }, [isLoading]);

    const onPermissionClick = (approved: boolean, idx: number) => {
        setRespondedSteps(prev => ({ ...prev, [idx]: true }));
        setCountdown(null);
        handlePermissionResponse(approved, idx);
    };

    const pendingPermissionStepIdx = agentSteps.findIndex((s, i) => 
        s.status === 'awaiting_permission' && !respondedSteps[i]
    );

    // Handle auto-run logic
    React.useEffect(() => {
        if (alwaysRun && pendingPermissionStepIdx >= 0 && !respondedSteps[pendingPermissionStepIdx] && countdown === null) {
            setCountdown(3); // 3 second countdown
        } else if (!alwaysRun || pendingPermissionStepIdx < 0) {
            setCountdown(null);
        }
    }, [alwaysRun, pendingPermissionStepIdx, respondedSteps]);

    React.useEffect(() => {
        if (countdown !== null && countdown > 0) {
            const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
            return () => clearTimeout(timer);
        } else if (countdown === 0 && pendingPermissionStepIdx >= 0) {
            onPermissionClick(true, pendingPermissionStepIdx);
        }
    }, [countdown, pendingPermissionStepIdx]);

    if (!isChatOpen) return null;

    return (
        <>
            <div className="chat-resize-handle" onMouseDown={handleChatResize} />
            <div className="chat-panel" style={{ width: `${chatWidth}px` }}>
                <div className="chat-panel-header">
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="2">
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
                        </svg>
                        <span style={{ fontWeight: 600, fontSize: 12 }}>WHIZCODE AGENT</span>
                    </div>
                    <div style={{ display: 'flex', gap: 4 }}>
                        <div className="chat-header-btn" onClick={handleReset} title="Reset conversation">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                <path d="M23 4v6h-6"></path>
                                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                            </svg>
                        </div>
                        <div className="chat-header-btn" onClick={() => setIsChatOpen(false)} title="Close panel">×</div>
                    </div>
                </div>

                {workspacePath && (
                    <div className="chat-context-bar">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                        </svg>
                        <span>{workspacePath.split(/[/\\]/).pop()}</span>
                        <span className="context-connected">● Context loaded</span>
                    </div>
                )}

                <ChatSettings {...settingsProps} />

                <div className="chat-messages">
                    {messages.map((msg, idx) => (
                        <div key={idx} className={`chat-msg ${msg.role}`}>
                            <div className="chat-msg-sender">
                                {msg.role === 'user' ? 'YOU' : 'WhizCode'}
                            </div>
                            {msg.steps && msg.steps.length > 0 && (
                                <div className="agent-steps">
                                    {msg.steps.map((step, si) => (
                                        <StepBlock key={si} step={step} getToolIcon={getToolIcon} />
                                    ))}
                                </div>
                            )}
                            <div className="chat-msg-content">
                                <MessageContent content={msg.content} role={msg.role} />
                            </div>
                        </div>
                    ))}

                    {isLoading && (
                        <div className="chat-msg assistant">
                            <div className="chat-msg-sender">WHIZCODE</div>
                            <div className="chat-msg-content">
                                {agentSteps.length > 0 ? (
                                    <div className="agent-steps live">
                                        {agentSteps.map((step, si) => (
                                            <StepBlock key={si} step={step} getToolIcon={getToolIcon} isLive={true} />
                                        ))}
                                    </div>
                                ) : (
                                    <div className="thinking-indicator">
                                        <div className="thinking-dot"></div>
                                        <div className="thinking-dot"></div>
                                        <div className="thinking-dot"></div>
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                    <div ref={messagesEndRef} />
                </div>

                <div className="chat-input-area">
                    {isLoading && (
                        <div className="agent-status-indicator" style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '8px',
                            padding: '8px 12px',
                            background: 'rgba(0, 122, 204, 0.1)',
                            border: '1px solid rgba(0, 122, 204, 0.3)',
                            borderRadius: '4px',
                            marginBottom: '10px',
                            fontSize: '11px',
                            color: 'var(--accent-primary)',
                            fontWeight: 500
                        }}>
                            <div className="spinner" style={{ width: 10, height: 10 }}></div>
                            <span>
                                {agentSteps.length > 0 && agentSteps[agentSteps.length - 1].status === 'running'
                                    ? agentSteps[agentSteps.length - 1].summary
                                    : agentSteps.length > 0 && agentSteps[agentSteps.length - 1].status === 'awaiting_permission'
                                    ? 'Waiting for permission...'
                                    : 'Thinking...'}
                            </span>
                        </div>
                    )}
                    {pendingPermissionStepIdx >= 0 && (
                        <div className="permission-controls-enhanced" style={{
                            display: 'flex', flexDirection: 'column', gap: '10px',
                            padding: '12px', background: 'var(--vscode-bg)',
                            border: '1px solid var(--accent-primary)', borderRadius: '6px',
                            marginBottom: '10px',
                            boxShadow: '0 -4px 12px rgba(0,0,0,0.3)',
                            borderLeft: '4px solid var(--accent-primary)'
                        }}>
                            <div style={{ display: 'flex', alignItems: 'flex-start', gap: '10px' }}>
                                <div style={{ fontSize: '18px', marginTop: '2px' }}>🛡️</div>
                                <div style={{ flex: 1, minWidth: 0 }}>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)', fontWeight: 600, textTransform: 'uppercase', marginBottom: '4px' }}>
                                        Permission Required
                                    </div>
                                    <div style={{
                                        fontSize: '13px',
                                        fontWeight: 400,
                                        lineHeight: '1.4',
                                        wordBreak: 'break-word',
                                        overflowWrap: 'anywhere',
                                        color: 'var(--text-primary)',
                                        fontFamily: 'var(--font-mono)',
                                        background: 'rgba(0,0,0,0.2)',
                                        padding: '6px',
                                        borderRadius: '4px'
                                    }}>
                                        {agentSteps[pendingPermissionStepIdx].summary}
                                    </div>
                                </div>
                            </div>

                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderTop: '1px solid var(--border-color)', paddingTop: '8px' }}>
                                <label style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer', userSelect: 'none' }}>
                                    <input
                                        type="checkbox"
                                        checked={alwaysRun}
                                        onChange={(e) => setAlwaysRun(e.target.checked)}
                                        style={{ accentColor: 'var(--accent-primary)' }}
                                    />
                                    <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Always run in this interaction</span>
                                </label>

                                <div style={{ display: 'flex', gap: '8px' }}>
                                    <button
                                        className="perm-btn deny"
                                        onClick={() => onPermissionClick(false, pendingPermissionStepIdx)}
                                        disabled={respondedSteps[pendingPermissionStepIdx] || !isLoading}
                                        style={{ padding: '4px 12px' }}
                                    >Deny</button>
                                    <button
                                        className="perm-btn approve"
                                        onClick={() => onPermissionClick(true, pendingPermissionStepIdx)}
                                        disabled={respondedSteps[pendingPermissionStepIdx] || !isLoading}
                                        style={{ padding: '4px 20px', minWidth: '80px', position: 'relative' }}
                                    >
                                        {countdown !== null ? `Run (${countdown}s)` : 'Run'}
                                    </button>
                                </div>
                            </div>
                        </div>
                    )}
                    <div className="chat-input-box">
                        <textarea
                            value={input}
                            onChange={(e) => setInput(e.target.value)}
                            onKeyDown={handleKeyDown}
                            placeholder={workspacePath ? "Ask about your code..." : "Open a folder first..."}
                            rows={1}
                            disabled={isLoading}
                        />
                        {!isLoading ? (
                            <button className="send-btn" onClick={handleSend} disabled={!input.trim() || isLoading}>
                                <svg className="send-icon" viewBox="0 0 24 24">
                                    <path d="M22 2L11 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                    <path d="M22 2L15 22L11 13L2 9L22 2Z" fill="currentColor" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                </svg>
                            </button>
                        ) : (
                            <button className="stop-btn" onClick={handleStop} title="Stop Agent">
                                <svg className="stop-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" fill="currentColor"></rect>
                                </svg>
                            </button>
                        )}
                    </div>
                </div>
            </div>
        </>
    );
}
