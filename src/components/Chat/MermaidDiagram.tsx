import React, { useEffect, useRef } from 'react';
import mermaid from 'mermaid';

mermaid.initialize({
  startOnLoad: true,
  theme: 'dark',
  securityLevel: 'loose',
  fontFamily: 'Fira Code, monospace',
});

export const MermaidDiagram: React.FC<{ chart: string }> = ({ chart }) => {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current && chart) {
      mermaid.contentLoaded();
      mermaid.render(`mermaid-${Math.floor(Math.random() * 1000)}`, chart).then((res) => {
        if (ref.current) {
          ref.current.innerHTML = res.svg;
        }
      }).catch(err => {
        console.error('Mermaid parsing error:', err);
      });
    }
  }, [chart]);

  return (
    <div 
      ref={ref} 
      className="mermaid-wrapper" 
      style={{ 
        background: '#1e1e2e',
        padding: '12px',
        borderRadius: '8px',
        border: '1px solid #313244',
        margin: '10px 0',
        display: 'flex',
        justifyContent: 'center',
        overflowX: 'auto'
      }}
    />
  );
};
