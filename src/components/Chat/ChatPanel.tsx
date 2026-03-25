import React from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'
import type { Message, AgentStep } from '../../types'
import { ChatSettings } from './ChatSettings'
import { MermaidDiagram } from './MermaidDiagram'
import { TerminalBlock } from './TerminalBlock'

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
    liveStreamingContent?: string;
    selectedImages: string[];
    setSelectedImages: React.Dispatch<React.SetStateAction<string[]>>;
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

// Smart summary formatter — turns raw "Executed X with args: {...}" into readable text
const formatStepSummary = (tool: string, summary: string): string => {
    // Try to extract args JSON from the summary
    const match = summary.match(/args:\s*(\{.*\})/s);
    if (!match) return summary;

    try {
        const args = JSON.parse(match[1]);
        switch (tool) {
            case 'write_file':
            case 'edit_file': {
                const path = args.path || args.file || '';
                const fileName = path.split(/[/\\]/).pop() || path;
                return tool === 'write_file'
                    ? `Write  ${fileName}`
                    : `Edit  ${fileName}${args.start_line ? `  (lines ${args.start_line}–${args.end_line || '?'})` : ''}`;
            }
            case 'read_file': {
                const path = args.path || '';
                const fileName = path.split(/[/\\]/).pop() || path;
                return `Read  ${fileName}`;
            }
            case 'list_directory': {
                const path = args.path || '';
                return `List  ${path}`;
            }
            case 'search_files': {
                return `Search  "${args.pattern || args.query}"${args.path ? `  in ${args.path}` : ''}`;
            }
            case 'run_command': {
                return `$ ${args.command || ''}`;
            }
            case 'git': {
                return `git ${args.operation || ''}${args.message ? ` "${args.message}"` : ''}`;
            }
            case 'npm': {
                return `npm ${args.operation || ''}${args.package ? ` ${args.package}` : ''}`;
            }
            default:
                return summary.replace(/Executed \w+ with args: \{.*\}/s, `${tool}`);
        }
    } catch {
        return summary;
    }
};

const StepBlock = ({ step, getToolIcon, isLive = false }: { step: AgentStep, getToolIcon: (t: string) => string, isLive?: boolean }) => {
    const [logsOpen, setLogsOpen] = React.useState(false);
    const hasLogs = step.logs && step.logs.length > 0;
    const canOpenLogs = step.tool === 'run_command' || hasLogs;

    // Auto-open logs if the step is a running command or if it fails
    React.useEffect(() => {
        if ((isLive && (step.status === 'running' || step.status === 'started') && step.tool === 'run_command') || step.status === 'failed') {
            setLogsOpen(true);
        }
    }, [isLive, step.status, step.tool]);

    const personaIcon = step.persona === 'planner' ? '🗺️' : 
                        step.persona === 'researcher' ? '🔍' :
                        step.persona === 'executor' ? '🛠️' :
                        step.persona === 'reviewer' ? '⚖️' : '🤖';
    const personaColor = step.persona === 'planner' ? '#cba6f7' : 
                         step.persona === 'researcher' ? '#89b4fa' :
                         step.persona === 'executor' ? '#a6e3a1' :
                         step.persona === 'reviewer' ? '#f9e2af' : '#9399b2';

    const handleClick = () => {
        if (canOpenLogs) {
            setLogsOpen(o => !o);
        }
    };

    const displaySummary = formatStepSummary(step.tool, step.summary);

    return (
        <div className={`agent-step ${step.status}`}>
            <div
                className="agent-step-header"
                onClick={handleClick}
                style={{ 
                    cursor: canOpenLogs ? 'pointer' : 'default',
                    userSelect: 'none',
                    display: 'flex',
                    alignItems: 'center'
                }}
            >
                {step.persona && (
                    <span style={{ 
                        fontSize: '9px', 
                        background: `${personaColor}15`, 
                        color: personaColor, 
                        padding: '1px 5px', 
                        borderRadius: '3px',
                        border: `1px solid ${personaColor}33`,
                        marginRight: '8px',
                        fontWeight: 'bold',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '3px'
                    }}>
                        {personaIcon} {step.persona.toUpperCase()}
                    </span>
                )}
                {isLive && (step.status === 'running' || step.status === 'started') ? (
                    <div className="spinner" style={{ width: 10, height: 10 }}></div>
                ) : (
                    <span className="agent-step-icon">{getToolIcon(step.tool)}</span>
                )}
                <span className="agent-step-summary">{displaySummary}</span>
                {/* Status badge */}
                <span style={{
                    marginLeft: '8px',
                    fontSize: '9px',
                    padding: '2px 6px',
                    borderRadius: '3px',
                    fontWeight: 'bold',
                    backgroundColor: 
                        step.status === 'identified' ? '#6c7086' :
                        step.status === 'started' ? '#89b4fa' :
                        step.status === 'completed' || step.status === 'done' ? '#a6e3a1' :
                        step.status === 'failed' ? '#f38ba8' :
                        '#9399b2',
                    color: '#1e1e2e'
                }}>
                    {step.status === 'identified' ? 'IDENTIFIED' :
                     step.status === 'started' ? 'RUNNING' :
                     step.status === 'completed' ? 'DONE' :
                     step.status === 'done' ? 'DONE' :
                     step.status === 'failed' ? 'FAILED' :
                     step.status.toUpperCase()}
                </span>
                {(step.status === 'done' || step.status === 'completed') && <span className="agent-step-check">✓</span>}
                {step.status === 'failed' && <span style={{ color: 'var(--error-color)', fontSize: 12 }}>✗</span>}
                {canOpenLogs && (
                    <span style={{ marginLeft: 'auto', fontSize: '10px', opacity: 0.5, paddingLeft: 6 }}>
                        {logsOpen ? '▲' : '▼'}
                    </span>
                )}
            </div>
            {step.data && <EditDetails data={step.data} />}
            {step.result && step.result.includes('file:///') && (
                <div style={{ padding: '8px 12px' }}>
                    <img 
                      src={step.result.split('URL: ')[1] || step.result.match(/file:\/\/\/[^\s]+/)?.[0]} 
                      alt="Generated Asset" 
                      style={{ maxWidth: '100%', borderRadius: '4px', border: '1px solid #313244', cursor: 'pointer' }}
                      onClick={() => window.open(step.result?.split('URL: ')[1])}
                    />
                </div>
            )}
            {canOpenLogs && logsOpen && (
                step.tool === 'run_command' ? (
                    <TerminalBlock 
                      logs={step.logs && step.logs.length > 0 ? step.logs : []} 
                      isLive={isLive} 
                      isRunning={step.status === 'running' || step.status === 'started'}
                      requestId={step.requestId} 
                    />
                ) : (
                    <LogContainer logs={step.logs && step.logs.length > 0 ? step.logs : ['(No logs yet)']} />
                )
            )}
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

    let thoughts: string[] = [];
    let cleanContent = content;

    const extractThoughts = (match: string, p1: string) => {
        const t = p1.trim();
        if (t && !thoughts.includes(t)) {
            thoughts.push(t);
        }
        return ''; // Remove the matched thought from the clean content
    };

    // 1. Extract and perfectly strip CLOSED thought blocks first
    const closedPatterns = [
        /<thought>([\s\S]*?)<\/thought>/gi,
        /<think>([\s\S]*?)<\/think>/gi,
        /\[thought\]([\s\S]*?)\[\/thought\]/gi,
        /\[think\]([\s\S]*?)\[\/think\]/gi,
    ];

    for (const pattern of closedPatterns) {
        cleanContent = cleanContent.replace(pattern, extractThoughts);
    }

    // 2. Handle UNCLOSED thought blocks (crucial for streaming)
    const unclosedPatterns = [
        /<thought>([\s\S]*?)(?=\n```|\n\{|$)/gi,
        /<think>([\s\S]*?)(?=\n```|\n\{|$)/gi,
        /\[thought\]([\s\S]*?)(?=\n```|\n\{|$)/gi,
        /\[think\]([\s\S]*?)(?=\n```|\n\{|$)/gi,
    ];

    for (const pattern of unclosedPatterns) {
        cleanContent = cleanContent.replace(pattern, extractThoughts);
    }

    // Normalize and strip all possible internal control tags
    cleanContent = cleanContent
        .replace(/<IDENTITY>[\s\S]*?<\/IDENTITY>/gi, '')
        .replace(/<PRIME_DIRECTIVE>[\s\S]*?<\/PRIME_DIRECTIVE>/gi, '')
        .replace(/<PLAN>[\s\S]*?<\/PLAN>/gi, '')
        .replace(/<PROJECT_STATUS>[\s\S]*?<\/PROJECT_STATUS>/gi, '')
        .replace(/<OUTPUT_FORMAT>[\s\S]*?<\/OUTPUT_FORMAT>/gi, '')
        .trim();

    // 2. Extract and hide tool call JSONs - only hide the JSON blocks, not the whole message
    const jsonBlockRegex = /```json\s*\{[\s\S]*?\}\s*```/g;
    const rawJsonRegex = /(?:\n|^)\s*\{[\s\S]*?\}\s*(?:\n|$)/g;
    
    // Support hiding partial/streaming JSON blocks
    const streamingJsonBlockRegex = /```json\s*\{[\s\S]*$/g;
    const streamingRawJsonRegex = /(?:\n|^)\s*\{[\s\S]*$/g;
    
    // Check if the content is JUST a tool call (no other text or thoughts)
    const hasToolCallJson = cleanContent.includes('"tool":') || (cleanContent.includes('{') && cleanContent.includes('"tool"'));
    
    // Strip JSON blocks (complete and partial)
    let strippedContent = cleanContent
        .replace(jsonBlockRegex, '')
        .replace(rawJsonRegex, '')
        .replace(streamingJsonBlockRegex, '')
        .replace(streamingRawJsonRegex, '')
        .trim();
    
    // If there's literally nothing left but a tool call and no thoughts, hide the message body
    if (hasToolCallJson && !strippedContent && thoughts.length === 0) {
        return null; 
    }

    // Otherwise, show the stripped content (the explanation/reasoning) but hide the JSON blocks
    const finalDisplayContent = strippedContent;

    // 3. Detect if the remaining content is just a JSON-like completion summary
    let isJsonSummary = false;
    if (finalDisplayContent.startsWith('{') && finalDisplayContent.endsWith('}') && finalDisplayContent.includes('"status"')) {
        try {
            isJsonSummary = true;
        } catch (e) { }
    }

    return (
        <div className="assistant-content-wrapper">
            {thoughts.length > 0 && (
                <div className="thought-process glass" style={{
                    marginBottom: '12px',
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
                        {finalDisplayContent}
                    </SyntaxHighlighter>
                ) : (
                    <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                            code({ className, children, ...props }) {
                                const match = /language-(\w+)/.exec(className || '');
                                const codeString = String(children).replace(/\n$/, '');
                                
                                if (match && match[1] === 'mermaid') {
                                    return <MermaidDiagram chart={codeString} />;
                                }

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
                        {finalDisplayContent}
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
    settingsProps,
    liveStreamingContent = '',
    selectedImages,
    setSelectedImages
}: ChatPanelProps) => {
    const [respondedSteps, setRespondedSteps] = React.useState<Record<number, boolean>>({});
    const [alwaysRun, setAlwaysRun] = React.useState(false);
    const [countdown, setCountdown] = React.useState<number | null>(null);
    const [currentPhase, setCurrentPhase] = React.useState<string>('analyzing');
    const [phaseStartTime, setPhaseStartTime] = React.useState<number>(Date.now());
    const [elapsedSeconds, setElapsedSeconds] = React.useState<number>(0);
    
    // WhizCode metrics state
    const [tokensPerSecond, setTokensPerSecond] = React.useState<number | undefined>();
    const [estimatedTimeRemaining, setEstimatedTimeRemaining] = React.useState<number | undefined>();
    const [totalTokens, setTotalTokens] = React.useState<number | undefined>();
    const [phaseHistory, setPhaseHistory] = React.useState<string[]>([]);

    React.useEffect(() => {
        const unlistenPhase = (window as any).__TAURI_INVOKE__?.('listen', {
            event: 'agent:phase',
            handler: (event: any) => {
                const phase = event.payload?.phase || 'analyzing';
                setCurrentPhase(phase);
                setPhaseStartTime(Date.now());
                // Add to phase history
                setPhaseHistory(prev => {
                    const updated = [...prev, phase];
                    // Keep only last 5 phases to avoid clutter
                    return updated.slice(-5);
                });
            }
        }).catch(() => {});

        return () => {
            unlistenPhase?.then((unlisten: any) => unlisten?.());
        };
    }, []);

    // Listen for metrics events
    React.useEffect(() => {
        const unlistenMetrics = (window as any).__TAURI_INVOKE__?.('listen', {
            event: 'agent:metrics',
            handler: (event: any) => {
                const metrics = event.payload;
                if (metrics.tokens_per_second !== undefined) {
                    setTokensPerSecond(metrics.tokens_per_second);
                }
                if (metrics.estimated_time_remaining !== undefined) {
                    setEstimatedTimeRemaining(metrics.estimated_time_remaining);
                }
                if (metrics.total_tokens !== undefined) {
                    setTotalTokens(metrics.total_tokens);
                }
            }
        }).catch(() => {});

        return () => {
            unlistenMetrics?.then((unlisten: any) => unlisten?.());
        };
    }, []);

    React.useEffect(() => {
        if (!isLoading) {
            setRespondedSteps({});
            setCountdown(null);
            // Reset metrics when loading completes
            setTokensPerSecond(undefined);
            setEstimatedTimeRemaining(undefined);
            setTotalTokens(undefined);
            setPhaseHistory([]);
            // Reset alwaysRun on task completion? Or keep it? Usually better to reset for safety.
            // setAlwaysRun(false); 
        } else {
            // Reset phase start time when loading begins
            setPhaseStartTime(Date.now());
            setPhaseHistory([]);
        }
    }, [isLoading]);

    // Update elapsed time every second — use a real counter instead of a
    // no-op setState trick which forces unnecessary re-renders.
    React.useEffect(() => {
        if (!isLoading) {
            setElapsedSeconds(0);
            return;
        }
        setElapsedSeconds(0);
        const timer = setInterval(() => {
            setElapsedSeconds(s => s + 1);
        }, 1000);
        return () => clearInterval(timer);
    }, [isLoading, phaseStartTime]);

    const onPermissionClick = (approved: boolean, idx: number) => {
        setRespondedSteps(prev => ({ ...prev, [idx]: true }));
        setCountdown(null);
        handlePermissionResponse(approved, idx);
    };

    const imageInputRef = React.useRef<HTMLInputElement>(null);
    const handleImageChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!e.target.files) return;
        Array.from(e.target.files).forEach(file => {
            const reader = new FileReader();
            reader.onloadend = () => {
                const base64 = reader.result as string;
                setSelectedImages(prev => [...prev, base64]);
            };
            reader.readAsDataURL(file);
        });
        e.target.value = ''; // Reset for next selection
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

    const getCurrentThought = (content: string) => {
        if (!content) return null;
        
        // Only show thought blocks - don't show raw LLM output which may contain
        // model thinking tokens with repeated words
        const patterns = [
            /\[THOUGHT\]([\s\S]*?)(?:\[\/THOUGHT\]|```|\{)/i,
            /<think>([\s\S]*?)(?:<\/think>|```|\{)/i,
            /\[REASONING\]([\s\S]*?)(?:\[\/REASONING\]|```|\{)/i,
        ];

        for (const pattern of patterns) {
            const match = content.match(pattern);
            if (match && match[1].trim()) {
                const thought = match[1].trim();
                return thought.length > 150 ? '...' + thought.slice(-150) : thought;
            }
        }
        
        return null;
    };

    const activeThought = getCurrentThought(liveStreamingContent);

    if (!isChatOpen) return null;

    return (
        <>
            <div className="chat-resize-handle" onMouseDown={handleChatResize} />
            <div className="chat-panel glass" style={{ width: `${chatWidth}px` }}>
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
                                {msg.images && msg.images.length > 0 && (
                                    <div className="msg-images" style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginBottom: '8px' }}>
                                        {msg.images.map((img, i) => (
                                            <img key={i} src={img} style={{ maxWidth: '200px', maxHeight: '150px', borderRadius: '4px', border: '1px solid rgba(255,255,255,0.1)' }} />
                                        ))}
                                    </div>
                                )}
                                <MessageContent content={msg.content} role={msg.role} />
                            </div>
                        </div>
                    ))}

                    {isLoading && (
                        <div className="chat-msg assistant">
                            <div className="chat-msg-sender">WHIZCODE</div>
                            <div className="chat-msg-content">
                                {agentSteps.length > 0 && (
                                    <div className="agent-steps live">
                                        {agentSteps.map((step, si) => (
                                            <StepBlock key={step.requestId || `step_${si}`} step={step} getToolIcon={getToolIcon} isLive={true} />
                                        ))}
                                    </div>
                                )}

                                {!liveStreamingContent && agentSteps.length === 0 && (
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
                        <div className="thought-stream-container" style={{
                            marginBottom: '10px',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '4px'
                        }}>
                            <div className="dynamic-thought-bar glass" style={{
                                border: '1px solid rgba(0, 122, 204, 0.2)',
                                borderRadius: '4px',
                                padding: '6px 12px',
                                fontSize: '11px',
                                color: 'var(--text-secondary)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '8px',
                                minHeight: '28px',
                                overflow: 'hidden'
                            }}>
                                <div className="thought-pulse" style={{
                                    width: '8px',
                                    height: '8px',
                                    borderRadius: '50%',
                                    background: 'var(--accent-primary)',
                                    boxShadow: '0 0 8px var(--accent-primary)',
                                    flexShrink: 0
                                }}></div>
                                    <div style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                                        <span style={{ fontWeight: 600, color: 'var(--accent-primary)', marginRight: '6px' }}>
                                            {currentPhase.toUpperCase()}:
                                        </span>
                                        <span className="live-thought-text" style={{ fontStyle: 'italic', opacity: 1, maxWidth: 'none' }}>
                                            {activeThought || (agentSteps.length > 0 ? (
                                                agentSteps.find((s: any) => s.status === 'running')?.summary || 
                                                [...agentSteps].reverse().find((s: any) => s.status === 'done')?.summary || 
                                                'Initiating plan...'
                                            ) : 'Analyzing context...')}
                                        </span>
                                    </div>
                                    <span style={{ fontSize: '10px', color: 'var(--text-secondary)', marginLeft: '8px', whiteSpace: 'nowrap' }}>
                                        {elapsedSeconds}s
                                    </span>
                            </div>
                            
                            {/* WhizCode Metrics Display */}
                            {(tokensPerSecond !== undefined || estimatedTimeRemaining !== undefined || totalTokens !== undefined) && (
                                <div style={{
                                    display: 'flex',
                                    gap: '12px',
                                    fontSize: '10px',
                                    color: 'var(--text-secondary)',
                                    padding: '6px 12px',
                                    backgroundColor: 'rgba(59, 130, 246, 0.05)',
                                    borderRadius: '4px',
                                    border: '1px solid rgba(59, 130, 246, 0.1)',
                                }}>
                                    {totalTokens !== undefined && (
                                        <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
                                            <span style={{ color: 'var(--accent-primary)', fontWeight: 'bold' }}>📊</span>
                                            <span>{totalTokens} tokens</span>
                                        </div>
                                    )}
                                    {tokensPerSecond !== undefined && (
                                        <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
                                            <span style={{ color: 'var(--accent-primary)', fontWeight: 'bold' }}>⚡</span>
                                            <span>{tokensPerSecond.toFixed(1)} tok/s</span>
                                        </div>
                                    )}
                                    {estimatedTimeRemaining !== undefined && (
                                        <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
                                            <span style={{ color: 'var(--accent-primary)', fontWeight: 'bold' }}>⏱️</span>
                                            <span>~{estimatedTimeRemaining < 60 ? `${estimatedTimeRemaining}s` : `${Math.floor(estimatedTimeRemaining / 60)}m`}</span>
                                        </div>
                                    )}
                                </div>
                            )}
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
                    {selectedImages.length > 0 && (
                        <div className="image-previews" style={{ display: 'flex', gap: '8px', marginBottom: '8px', overflowX: 'auto', padding: '4px' }}>
                            {selectedImages.map((img, i) => (
                                <div key={i} style={{ position: 'relative', width: '60px', height: '60px', flexShrink: 0 }}>
                                    <img src={img} style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: '4px', border: '1px solid var(--border-color)' }} />
                                    <button 
                                        onClick={() => setSelectedImages(prev => prev.filter((_, idx) => idx !== i))}
                                        style={{ position: 'absolute', top: '-6px', right: '-6px', background: '#e74c3c', color: 'white', border: 'none', borderRadius: '50%', width: '16px', height: '16px', fontSize: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', cursor: 'pointer' }}
                                    >×</button>
                                </div>
                            ))}
                        </div>
                    )}
                    <div className="chat-input-box">
                        <input 
                            type="file" 
                            ref={imageInputRef} 
                            style={{ display: 'none' }} 
                            accept="image/*" 
                            multiple 
                            onChange={handleImageChange} 
                        />
                        <button 
                          onClick={() => imageInputRef.current?.click()}
                          disabled={isLoading}
                          style={{
                              background: 'transparent',
                              border: 'none',
                              fontSize: '18px',
                              cursor: 'pointer',
                              padding: '0 8px',
                              opacity: isLoading ? 0.3 : 0.7,
                              transition: 'opacity 0.2s'
                          }}
                          onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
                          onMouseLeave={(e) => e.currentTarget.style.opacity = '0.7'}
                        >🖼️</button>
                        <textarea
                            className="chat-input"
                            value={input}
                            onChange={(e) => setInput(e.target.value)}
                            onKeyDown={handleKeyDown}
                            onPaste={(e) => {
                                const items = e.clipboardData.items;
                                for (let i = 0; i < items.length; i++) {
                                    if (items[i].type.indexOf('image') !== -1) {
                                        const file = items[i].getAsFile();
                                        if (file) {
                                            const reader = new FileReader();
                                            reader.onloadend = () => {
                                                const base64 = reader.result as string;
                                                setSelectedImages(prev => [...prev, base64]);
                                            };
                                            reader.readAsDataURL(file);
                                        }
                                    }
                                }
                            }}
                            placeholder={workspacePath ? "Ask about your code..." : "Open a folder first..."}
                            rows={3}
                            disabled={isLoading}
                        />
                        {!isLoading ? (
                            <button className="send-btn" onClick={() => handleSend()} disabled={!input.trim() && selectedImages.length === 0}>
                                <svg className="send-icon" viewBox="0 0 24 24">
                                    <path d="M22 2L11 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                    <path d="M22 2L15 22L11 13L2 9L22 2Z" fill="currentColor" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                </svg>
                            </button>
                        ) : (
                            <button className="stop-btn" onClick={() => handleStop()} title="Stop Agent">
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
