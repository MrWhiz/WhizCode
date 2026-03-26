import { useRef, useEffect } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { terminal as terminalApi, events } from '../../lib/tauri-api'

interface TerminalPaneProps {
  terminalId: string
  clearToken?: number
  isVisible?: boolean
}

export const TerminalPane = ({ terminalId, clearToken = 0, isVisible = true }: TerminalPaneProps) => {
  const terminalRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<Terminal | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const clearTokenRef = useRef(clearToken)

  useEffect(() => {
    if (!terminalRef.current) return
    let unmounted = false
    let observer: ResizeObserver | null = null
    let unlistenData: (() => void) | null = null
    let unlistenExit: (() => void) | null = null
    let fitTimer: number | null = null

    try {
      const term = new Terminal({
        theme: {
          background: '#1e1e1e',
          foreground: '#d4d4d4',
          cursor: '#d4d4d4',
          cursorAccent: '#1e1e1e',
          selectionBackground: '#264f78',
          black: '#000000',
          red: '#f44747',
          green: '#89d185',
          yellow: '#d7ba7d',
          blue: '#569cd6',
          magenta: '#c586c0',
          cyan: '#4ec9b0',
          white: '#d4d4d4',
          brightBlack: '#666666',
          brightRed: '#ff6b68',
          brightGreen: '#b5cea8',
          brightYellow: '#dcdcaa',
          brightBlue: '#9cdcfe',
          brightMagenta: '#d7ba7d',
          brightCyan: '#9cdcfe',
          brightWhite: '#f5f5f5',
        },
        fontFamily: "'Cascadia Code', 'Consolas', 'Courier New', monospace",
        fontSize: 13,
        lineHeight: 1.35,
        scrollback: 20000,
        cursorBlink: true,
        cursorStyle: 'block',
        allowTransparency: false,
        convertEol: true,
        screenReaderMode: false,
        disableStdin: false,
      })

      const fitAddon = new FitAddon()
      fitAddonRef.current = fitAddon
      term.loadAddon(fitAddon)
      termRef.current = term

      const setupListeners = async () => {
        try {
          unlistenData = await events.onTerminalData(terminalId, (data: string) => {
            term.write(data)
          })

          unlistenExit = await events.onTerminalExit(terminalId, (code: number) => {
            term.writeln('')
            term.writeln(`\u001b[38;5;244m[process exited with code ${code}]\u001b[0m`)
          })
        } catch (err) {
          console.error('Failed to setup terminal listeners:', err)
        }
      }
      setupListeners()

      term.open(terminalRef.current)

      const fitTerminal = () => {
        if (unmounted) return
        try {
          fitAddon.fit()
          terminalApi.resizeTerminal(terminalId, term.cols, term.rows).catch((err: unknown) => {
            console.error('Terminal resize error:', err)
          })
        } catch (err: unknown) {
          console.error('Terminal resize error:', err)
        }
      }

      const scheduleFit = (delay = 0) => {
        if (fitTimer) {
          window.clearTimeout(fitTimer)
        }
        fitTimer = window.setTimeout(() => {
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              fitTerminal()
            })
          })
        }, delay)
      }

      const focusTerminal = () => term.focus()
      const handleClick = () => focusTerminal()
      terminalRef.current.addEventListener('click', handleClick)

      scheduleFit(0)
      if (isVisible) {
        setTimeout(() => {
          if (!unmounted) {
            term.focus()
          }
        }, 50)
      }

      term.onData((data: string) => {
        terminalApi.writeToTerminal(terminalId, data).catch((err: unknown) => {
          console.error('Terminal write error:', err)
        })
      })

      const handleResize = () => {
        if (unmounted) return
        scheduleFit(0)
      }

      setTimeout(handleResize, 100)
      window.addEventListener('resize', handleResize)
      observer = new ResizeObserver(() => {
        if (!unmounted) {
          handleResize()
        }
      })
      observer.observe(terminalRef.current)

      return () => {
        terminalRef.current?.removeEventListener('click', handleClick)
        if (unlistenData) unlistenData()
        if (unlistenExit) unlistenExit()
        window.removeEventListener('resize', handleResize)
        observer?.disconnect()
        if (fitTimer) {
          window.clearTimeout(fitTimer)
        }
        term.dispose()
        termRef.current = null
        fitAddonRef.current = null
      }
    } catch (err) {
      console.error('Terminal initialization error:', err)
    }

    return () => {
      unmounted = true
      if (unlistenData) unlistenData()
      if (unlistenExit) unlistenExit()
      observer?.disconnect()
      if (fitTimer) {
        window.clearTimeout(fitTimer)
      }
      termRef.current?.dispose()
      termRef.current = null
      fitAddonRef.current = null
    }
  }, [terminalId])

  useEffect(() => {
    if (!isVisible) return
    const timer = window.setTimeout(() => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          fitAddonRef.current?.fit()
          termRef.current?.focus()
        })
      })
    }, 50)
    return () => window.clearTimeout(timer)
  }, [isVisible])

  useEffect(() => {
    if (clearTokenRef.current === clearToken) return
    clearTokenRef.current = clearToken
    const term = termRef.current
    if (!term) return
    term.clear()
  }, [clearToken])

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        overflow: 'hidden',
        padding: '4px 8px',
        boxSizing: 'border-box',
      }}
      ref={terminalRef}
    />
  )
}
