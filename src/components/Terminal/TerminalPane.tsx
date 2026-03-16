import { useRef, useEffect } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

interface TerminalPaneProps {
    terminalId?: string
}

export const TerminalPane = ({ terminalId = 'default' }: TerminalPaneProps) => {
    const terminalRef = useRef<HTMLDivElement>(null)

    useEffect(() => {
        if (!terminalRef.current) return
        let term: Terminal | null = null
        let observer: ResizeObserver | null = null
        const ipc = (window as any).ipcRenderer
        let unmounted = false
        let onIncomingData: any
        let handleResize: any

        try {
            term = new Terminal({
                theme: {
                    background: '#1e1e1e',
                    foreground: '#cccccc',
                    cursor: '#cccccc',
                    cursorAccent: '#1e1e1e',
                    selectionBackground: '#264f78',
                    black: '#000000',
                    red: '#cd3131',
                    green: '#0dbc79',
                    yellow: '#e5e510',
                    blue: '#2472c8',
                    magenta: '#bc3fbc',
                    cyan: '#11a8cd',
                    white: '#e5e5e5',
                    brightBlack: '#666666',
                    brightRed: '#f14c4c',
                    brightGreen: '#23d18b',
                    brightYellow: '#f5f543',
                    brightBlue: '#3b8eea',
                    brightMagenta: '#d670d6',
                    brightCyan: '#29b8db',
                    brightWhite: '#e5e5e5'
                },
                fontFamily: "'Cascadia Code', 'Consolas', 'Courier New', monospace",
                fontSize: 13,
                lineHeight: 1.2,
                scrollback: 10000,
                cursorBlink: true,
                cursorStyle: 'block',
                allowTransparency: false,
                convertEol: true
            })

            const fitAddon = new FitAddon()
            term.loadAddon(fitAddon)
            term.open(terminalRef.current)

            // Initial fit
            setTimeout(() => {
                if (!unmounted) {
                    fitAddon.fit()
                }
            }, 50)

            if (ipc) {
                ipc.send('terminal:spawn', terminalId)

                onIncomingData = (_event: any, data: string, id: string) => {
                    if (id === terminalId && term) {
                        term.write(data)
                    }
                }

                ipc.on('terminal:incomingData', onIncomingData)

                term.onData((data: string) => {
                    ipc.send('terminal:keystroke', data, terminalId)
                })

                handleResize = () => {
                    if (unmounted || !term) return
                    try {
                        fitAddon.fit()
                        ipc.send('terminal:resize', term.cols, term.rows, terminalId)
                    } catch (err) {
                        console.error('Terminal resize error:', err)
                    }
                }

                // Delayed resize to ensure container is ready
                setTimeout(handleResize, 100)
                window.addEventListener('resize', handleResize)

                // Observe container size changes
                observer = new ResizeObserver(() => {
                    if (!unmounted) {
                        handleResize()
                    }
                })
                if (terminalRef.current) {
                    observer.observe(terminalRef.current)
                }
            }
        } catch (err: any) {
            console.error('Terminal initialization error:', err)
        }

        return () => {
            unmounted = true
            try {
                if (ipc && onIncomingData) {
                    ipc.off('terminal:incomingData', onIncomingData)
                }
                if (handleResize) {
                    window.removeEventListener('resize', handleResize)
                }
                if (observer) {
                    observer.disconnect()
                }
                if (term) {
                    term.dispose()
                }
            } catch (e) {
                console.error('Terminal cleanup error:', e)
            }
        }
    }, [terminalId])

    return (
        <div 
            style={{ 
                width: '100%', 
                height: '100%', 
                overflow: 'hidden',
                padding: '4px 8px'
            }} 
            ref={terminalRef}
        />
    )
}
