import { useRef, useEffect } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

export const TerminalPane = () => {
    const terminalRef = useRef<HTMLDivElement>(null)

    useEffect(() => {
        if (!terminalRef.current) return
        let term: any;
        let observer: any;
        const ipc = (window as any).ipcRenderer
        let unmounted = false;
        let onIncomingData: any;
        let handleResize: any;

        try {
            term = new Terminal({
                theme: { background: '#1e1e1e', foreground: '#cccccc' },
                fontFamily: "'Consolas', 'Courier New', monospace",
                fontSize: 14,
            })
            const fitAddon = new FitAddon()
            term.loadAddon(fitAddon)
            term.open(terminalRef.current)

            setTimeout(() => fitAddon.fit(), 10)

            if (ipc) {
                ipc.send('terminal:spawn')

                onIncomingData = (_event: any, data: string) => {
                    term.write(data)
                }

                ipc.on('terminal:incomingData', onIncomingData)

                term.onData((data: string) => {
                    ipc.send('terminal:keystroke', data)
                })

                handleResize = () => {
                    if (unmounted) return;
                    fitAddon.fit()
                    ipc.send('terminal:resize', term.cols, term.rows)
                }

                setTimeout(handleResize, 100)
                window.addEventListener('resize', handleResize)

                observer = new ResizeObserver(() => handleResize())
                if (terminalRef.current) {
                    observer.observe(terminalRef.current)
                }
            }
        } catch (err: any) {
            console.error(err)
        }

        return () => {
            unmounted = true;
            try {
                if (ipc && onIncomingData) {
                    if (ipc.off) ipc.off('terminal:incomingData', onIncomingData)
                }
                if (handleResize) window.removeEventListener('resize', handleResize)
                if (observer) observer.disconnect()
                if (term) term.dispose()
            } catch (e) { }
        }
    }, [])

    return (
        <div style={{ width: '100%', height: '100%', overflow: 'hidden' }} ref={terminalRef}></div>
    )
}
