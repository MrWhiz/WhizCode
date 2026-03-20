import { WhizLogo } from '../Branding/WhizLogo';

interface MenuItem {
    label?: string;
    action?: string;
    shortcut?: string;
    separator?: boolean;
}

interface Menu {
    name: string;
    items: MenuItem[];
}


interface TitleBarProps {
    menus: Menu[];
    activeMenu: string | null;
    toggleMenu: (name: string) => void;
    handleMenuHover: (name: string) => void;
    handleMenuAction: (action: string) => void;
}

export const TitleBar = ({
    menus,
    activeMenu,
    toggleMenu,
    handleMenuHover,
    handleMenuAction
}: TitleBarProps) => {
    return (
        <div className="title-bar">
            <div className="title-bar-left">
                <WhizLogo size={32} />
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
                                <div className="dropdown-menu">
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
                WhizCode - Local
            </div>
        </div>
    )
}
