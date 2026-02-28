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
    handlePermissionResponse: (approved: boolean) => void;
    // Settings props
    settingsProps: any;
}

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
    settingsProps
}: ChatPanelProps) => {
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
                                        <div key={si} className={`agent-step ${step.status}`}>
                                            <span className="agent-step-icon">{getToolIcon(step.tool)}</span>
                                            <span className="agent-step-summary">{step.summary}</span>
                                            {step.status === 'done' && <span className="agent-step-check">✓</span>}
                                        </div>
                                    ))}
                                </div>
                            )}
                            <div className="chat-msg-content">
                                {msg.role === 'assistant' ? (
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
                                        {msg.content}
                                    </ReactMarkdown>
                                ) : (
                                    msg.content
                                )}
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
                                            <div key={si} className={`agent-step ${step.status}`}>
                                                {step.status === 'running' ? (
                                                    <div className="spinner" style={{ width: 10, height: 10 }}></div>
                                                ) : (
                                                    <span className="agent-step-icon">{getToolIcon(step.tool)}</span>
                                                )}
                                                <span className="agent-step-summary">{step.summary}</span>
                                                {step.status === 'done' && <span className="agent-step-check">✓</span>}
                                                {step.status === 'awaiting_permission' && (
                                                    <div className="permission-controls">
                                                        <button className="perm-btn approve" onClick={() => handlePermissionResponse(true)}>Run</button>
                                                        <button className="perm-btn deny" onClick={() => handlePermissionResponse(false)}>Deny</button>
                                                    </div>
                                                )}
                                            </div>
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
                    <div className="chat-input-box">
                        <textarea
                            value={input}
                            onChange={(e) => setInput(e.target.value)}
                            onKeyDown={handleKeyDown}
                            placeholder={workspacePath ? "Ask about your code..." : "Open a folder first..."}
                            rows={1}
                            disabled={isLoading}
                        />
                        <button className="send-btn" onClick={handleSend} disabled={!input.trim() || isLoading}>
                            <svg className="send-icon" viewBox="0 0 24 24">
                                <path d="M22 2L11 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                <path d="M22 2L15 22L11 13L2 9L22 2Z" fill="currentColor" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </>
    );
}
