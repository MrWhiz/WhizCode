import React, { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

interface TerminalBlockProps {
    logs: string[];
    isLive: boolean;
    isRunning?: boolean;
    requestId?: string;
}

export const TerminalBlock: React.FC<TerminalBlockProps> = ({ logs, isLive, isRunning, requestId }) => {
    const terminalRef = useRef<HTMLDivElement>(null);
    const xtermRef = useRef<Terminal | null>(null);
    const fitAddonRef = useRef<FitAddon | null>(null);
    const lastRenderedIndex = useRef<number>(0);

    useEffect(() => {
        if (!terminalRef.current) return;

        // Initialize xterm.js
        const term = new Terminal({
            cursorBlink: true,
            fontSize: 10,
            fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
            theme: {
                background: '#1e1e2e',
                foreground: '#cdd6f4',
                cursor: '#f5e0dc',
                selectionBackground: '#585b70',
                black: '#45475a',
                red: '#f38ba8',
                green: '#a6e3a1',
                yellow: '#f9e2af',
                blue: '#89b4fa',
                magenta: '#f5c2e7',
                cyan: '#94e2d5',
                white: '#bac272',
            },
            convertEol: true,
            rows: 12,
        });

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        
        term.open(terminalRef.current);
        fitAddon.fit();

        xtermRef.current = term;
        fitAddonRef.current = fitAddon;

        // If we have initial logs, write them
        logs.forEach(log => term.write(log));
        lastRenderedIndex.current = logs.length;

        // Handle terminal input
        term.onData(data => {
            if (requestId && isLive) {
                import('@tauri-apps/api/core').then(({ invoke }) => {
                    invoke('agent_send_terminal_input', { requestId, input: data }).catch(err => {
                        console.error('Failed to send terminal input:', err);
                    });
                });
            }
        });

        // Handle resize
        const resizeHandler = () => fitAddon.fit();
        window.addEventListener('resize', resizeHandler);

        return () => {
            window.removeEventListener('resize', resizeHandler);
            term.dispose();
        };
    }, [isLive, requestId]); // Removed logs from dependency to prevent re-init on every log update

    // Incremental log updates
    useEffect(() => {
        const term = xtermRef.current;
        if (!term) return;

        if (logs.length > lastRenderedIndex.current) {
            const newLogs = logs.slice(lastRenderedIndex.current);
            newLogs.forEach(log => term.write(log));
            lastRenderedIndex.current = logs.length;
        } else if (logs.length < lastRenderedIndex.current) {
            // Logs were reset (e.g. new command)
            term.clear();
            logs.forEach(log => term.write(log));
            lastRenderedIndex.current = logs.length;
        }
    }, [logs]);

    const handleStop = async () => {
        if (requestId) {
            const { invoke } = await import('@tauri-apps/api/core');
            try {
                await invoke('agent_stop_terminal_command', { requestId });
            } catch (err) {
                console.error('Failed to stop terminal command:', err);
            }
        }
    };

    return (
        <div className="terminal-block-container" style={{
            margin: '8px 0',
            borderRadius: '6px',
            overflow: 'hidden',
            border: '1px solid rgba(255,255,255,0.1)',
            background: '#1e1e2e',
            padding: '6px',
            position: 'relative'
        }}>
            <div ref={terminalRef} style={{ width: '100%', height: '100%' }} />
            {isLive && isRunning && requestId && (
                <button
                    onClick={handleStop}
                    style={{
                        position: 'absolute',
                        top: '12px',
                        right: '12px',
                        backgroundColor: '#f38ba8',
                        color: '#11111b',
                        border: 'none',
                        borderRadius: '4px',
                        padding: '4px 8px',
                        fontSize: '10px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        zIndex: 10,
                        display: 'flex',
                        alignItems: 'center',
                        gap: '4px',
                        boxShadow: '0 2px 4px rgba(0,0,0,0.3)',
                        transition: 'all 0.2s ease',
                    }}
                    onMouseEnter={(e) => {
                        e.currentTarget.style.backgroundColor = '#eba0ac';
                        e.currentTarget.style.transform = 'scale(1.05)';
                    }}
                    onMouseLeave={(e) => {
                        e.currentTarget.style.backgroundColor = '#f38ba8';
                        e.currentTarget.style.transform = 'scale(1)';
                    }}
                >
                    <span style={{ fontSize: '12px' }}>■</span> STOP
                </button>
            )}
        </div>
    );
};
