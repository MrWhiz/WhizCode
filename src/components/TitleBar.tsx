import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useState, useEffect } from 'react'
import { saveAppState } from '../lib/appState'

interface MenuItem {
    label?: string
    action?: string
    shortcut?: string
    separator?: boolean
}

interface Menu {
    name: string
    items: MenuItem[]
}

interface TitleBarProps {
    menus: Menu[]
    activeMenu: string | null
    toggleMenu: (menu: string) => void
    handleMenuHover: (menu: string) => void
    handleMenuAction: (action: string) => void
}

export const TitleBar = ({ menus, activeMenu, toggleMenu, handleMenuHover, handleMenuAction }: TitleBarProps) => {
    const [isMaximized, setIsMaximized] = useState(false)

    useEffect(() => {
        const checkMaximized = async () => {
            try {
                const appWindow = WebviewWindow.getCurrent()
                const maximized = await appWindow.isMaximized()
                setIsMaximized(maximized)
            } catch (error) {
                console.error('Failed to check maximized state:', error)
            }
        }

        checkMaximized()
    }, [])

    const handleMinimize = async () => {
        try {
            const appWindow = WebviewWindow.getCurrent()
            await appWindow.minimize()
        } catch (error) {
            console.error('Failed to minimize:', error)
        }
    }

    const handleMaximize = async () => {
        try {
            const appWindow = WebviewWindow.getCurrent()
            if (isMaximized) {
                await appWindow.unmaximize()
                setIsMaximized(false)
                saveAppState({ isMaximized: false })
            } else {
                await appWindow.maximize()
                setIsMaximized(true)
                saveAppState({ isMaximized: true })
            }
        } catch (error) {
            console.error('Failed to maximize:', error)
        }
    }

    const handleClose = async () => {
        try {
            const appWindow = WebviewWindow.getCurrent()
            await appWindow.close()
        } catch (error) {
            console.error('Failed to close:', error)
        }
    }

    return (
        <div className="title-bar glass">
            <div className="title-bar-left">
                <div className="logo">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#007acc" strokeWidth="2">
                        <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                </div>
                <div className="menu-bar">
                    {menus.map(menu => (
                        <div
                            key={menu.name}
                            className={`menu-item ${activeMenu === menu.name ? 'active' : ''}`}
                            onClick={() => toggleMenu(menu.name)}
                            onMouseEnter={() => handleMenuHover(menu.name)}
                        >
                            {menu.name}
                            {activeMenu === menu.name && (
                                <div className="dropdown-menu" onMouseLeave={() => toggleMenu('')}>
                                    {menu.items.map((item, i) => (
                                        item.separator ? (
                                            <div key={i} className="dropdown-item separator" />
                                        ) : (
                                            <div key={i} className="dropdown-item" onClick={(e) => { e.stopPropagation(); handleMenuAction(item.action!); }}>
                                                <span>{item.label}</span>
                                                {item.shortcut && <span className="shortcut">{item.shortcut}</span>}
                                            </div>
                                        )
                                    ))}
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            </div>
            <div className="title-bar-center">
                <span>WhizCode</span>
            </div>
            <div className="title-bar-right" style={{ display: 'flex', gap: '8px', alignItems: 'center', paddingRight: '8px' }}>
                <button
                    onClick={handleMinimize}
                    style={{
                        background: 'none',
                        border: 'none',
                        color: 'rgba(255,255,255,0.7)',
                        cursor: 'pointer',
                        padding: '4px 8px',
                        display: 'flex',
                        alignItems: 'center',
                        fontSize: '16px'
                    }}
                    title="Minimize"
                >
                    −
                </button>
                <button
                    onClick={handleMaximize}
                    style={{
                        background: 'none',
                        border: 'none',
                        color: 'rgba(255,255,255,0.7)',
                        cursor: 'pointer',
                        padding: '4px 8px',
                        display: 'flex',
                        alignItems: 'center',
                        fontSize: '16px'
                    }}
                    title={isMaximized ? 'Restore' : 'Maximize'}
                >
                    {isMaximized ? '❐' : '□'}
                </button>
                <button
                    onClick={handleClose}
                    style={{
                        background: 'none',
                        border: 'none',
                        color: 'rgba(255,255,255,0.7)',
                        cursor: 'pointer',
                        padding: '4px 8px',
                        display: 'flex',
                        alignItems: 'center',
                        fontSize: '16px'
                    }}
                    title="Close"
                >
                    ✕
                </button>
            </div>
        </div>
    )
}
