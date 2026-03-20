import React, { useEffect, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import mermaid from 'mermaid';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';

interface MermaidProps {
    chart: string;
}

const Mermaid = ({ chart }: MermaidProps) => {
    const ref = useRef<HTMLDivElement>(null);
    const [svg, setSvg] = useState<string>('');

    useEffect(() => {
        mermaid.initialize({
            startOnLoad: true,
            theme: 'dark',
            securityLevel: 'loose',
            fontFamily: 'Inter, system-ui',
            themeVariables: {
                primaryColor: '#1e1e1e',
                primaryTextColor: '#fff',
                primaryBorderColor: '#007ACC',
                lineColor: '#569cd6',
                secondaryColor: '#2d2d2d',
                tertiaryColor: '#252526'
            }
        });

        const renderChart = async () => {
            if (ref.current && chart) {
                try {
                    const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
                    const { svg } = await mermaid.render(id, chart);
                    setSvg(svg);
                } catch (err) {
                    console.error('Mermaid render error:', err);
                    setSvg('<div style="color: #f44336; padding: 10px; border: 1px solid #f44336; border-radius: 4px;">Error rendering diagram</div>');
                }
            }
        };

        renderChart();
    }, [chart]);

    return (
        <div ref={ref} style={{
            display: 'flex',
            justifyContent: 'center',
            margin: '24px 0',
            backgroundColor: '#000',
            padding: '24px',
            borderRadius: '12px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            overflowX: 'auto'
        }} dangerouslySetInnerHTML={{ __html: svg }} />
    );
};

interface MarkdownRendererProps {
    content: string;
    className?: string;
}

export const MarkdownRenderer = ({ content, className }: MarkdownRendererProps) => {
    return (
        <div 
            className={`markdown-preview ${className || ''}`} 
            style={{
                height: '100%',
                overflowY: 'auto',
                backgroundColor: '#1e1e1e',
                padding: '40px',
                color: '#d4d4d4',
                lineHeight: '1.6',
                fontSize: '15px'
            }}
        >
            <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                    code({ node, inline, className, children, ...props }: any) {
                        const match = /language-(\w+)/.exec(className || '');
                        const lang = match ? match[1] : '';

                        if (lang === 'mermaid') {
                            return <Mermaid chart={String(children).replace(/\n$/, '')} />;
                        }

                        return !inline && match ? (
                            <SyntaxHighlighter
                                style={vscDarkPlus as any}
                                language={lang}
                                PreTag="div"
                                {...props}
                            >
                                {String(children).replace(/\n$/, '')}
                            </SyntaxHighlighter>
                        ) : (
                            <code className={className} {...props} style={{
                                backgroundColor: 'rgba(255,255,255,0.1)',
                                padding: '2px 4px',
                                borderRadius: '4px',
                                fontSize: '0.9em'
                            }}>
                                {children}
                            </code>
                        );
                    },
                        h1: ({ children }) => <h1 style={{ borderBottom: '1px solid #333', paddingBottom: '0.3em', marginTop: '24px', marginBottom: '16px' }}>{children}</h1>,
                        h2: ({ children }) => <h2 style={{ borderBottom: '1px solid #333', paddingBottom: '0.3em', marginTop: '24px', marginBottom: '16px' }}>{children}</h2>,
                        h3: ({ children }) => <h3 style={{ marginTop: '24px', marginBottom: '16px' }}>{children}</h3>,
                        p: ({ children }) => <p style={{ marginBottom: '16px' }}>{children}</p>,
                        ul: ({ children }) => <ul style={{ marginBottom: '16px', paddingLeft: '2em' }}>{children}</ul>,
                        ol: ({ children }) => <ol style={{ marginBottom: '16px', paddingLeft: '2em' }}>{children}</ol>,
                        li: ({ children }) => <li style={{ marginBottom: '4px' }}>{children}</li>,
                        blockquote: ({ children }) => (
                            <blockquote style={{
                                borderLeft: '4px solid #444',
                                paddingLeft: '1em',
                                color: '#888',
                                margin: '0 0 16px 0'
                            }}>
                                {children}
                            </blockquote>
                        ),
                        table: ({ children }) => (
                            <table style={{
                                borderCollapse: 'collapse',
                                width: '100%',
                                marginBottom: '16px'
                            }}>
                                {children}
                            </table>
                        ),
                        th: ({ children }) => (
                            <th style={{
                                border: '1px solid #444',
                                padding: '6px 13px',
                                backgroundColor: '#2d2d2d'
                            }}>
                                {children}
                            </th>
                        ),
                        td: ({ children }) => (
                            <td style={{
                                border: '1px solid #444',
                                padding: '6px 13px'
                            }}>
                                {children}
                            </td>
                        ),
                        a: ({ children, href }) => <a href={href} target="_blank" rel="noopener noreferrer" style={{ color: '#4fc1ff', textDecoration: 'none' }}>{children}</a>,
                    }}
                >
                    {content}
                </ReactMarkdown>
        </div>
    );
};
