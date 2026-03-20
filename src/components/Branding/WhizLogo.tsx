import React from 'react';

interface LogoProps {
    size?: number;
    showText?: boolean;
    className?: string;
    style?: React.CSSProperties;
    textColor?: string;
    centered?: boolean;
}

export const WhizLogo = ({ size = 24, showText = false, className, style, textColor, centered }: LogoProps) => {
    return (
        <div
            className={`whiz-logo-container ${className || ''}`}
            style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: centered ? 'center' : 'flex-start',
                gap: '12px',
                height: size,
                margin: centered ? '0 auto' : '0',
                ...style
            }}
        >
            <svg
                width={size}
                height={size}
                viewBox="0 0 100 100"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
                style={{ overflow: 'visible' }}
            >
                <defs>
                    <linearGradient id="shield-grad-2" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stopColor="#3b82f6" />
                        <stop offset="100%" stopColor="#8b5cf6" />
                    </linearGradient>
                    <linearGradient id="bolt-grad-2" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stopColor="#22d3ee" />
                        <stop offset="100%" stopColor="white" />
                    </linearGradient>
                    <filter id="glow-bolt" x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur stdDeviation="3.5" result="blur" />
                        <feComposite in="SourceGraphic" in2="blur" operator="over" />
                    </filter>
                </defs>

                {/* Outer Brackets (Minimalist Shield) */}
                <path
                    d="M25 20L10 50L25 80"
                    stroke="url(#shield-grad-2)"
                    strokeWidth="8"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                />
                <path
                    d="M75 20L90 50L75 80"
                    stroke="url(#shield-grad-2)"
                    strokeWidth="8"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                />

                {/* High-Voltage Lightning Bolt */}
                <path
                    d="M55 10L35 55H65L45 90"
                    stroke="url(#bolt-grad-2)"
                    strokeWidth="10"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    filter="url(#glow-bolt)"
                />
            </svg>

            {showText && (
                <span style={{
                    fontSize: size * 0.9,
                    fontWeight: 800,
                    color: textColor || 'var(--text-primary)',
                    fontFamily: "'Outfit', sans-serif",
                    letterSpacing: '-2px',
                    filter: 'drop-shadow(0 0 15px rgba(59, 130, 246, 0.4))'
                }}>
                    WHIZ<span style={{ opacity: 0.5, fontWeight: 300 }}>CODE</span>
                </span>
            )}
        </div>
    );
};
