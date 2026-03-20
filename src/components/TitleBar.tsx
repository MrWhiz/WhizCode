
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
        </div>
    )
}
