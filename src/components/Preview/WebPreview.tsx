import React, { useState, useEffect, useRef } from 'react';

interface WebPreviewProps {
  url?: string;
  onUrlChange?: (url: string) => void;
  onScreenshot?: (dataUrl: string) => void;
}

export const WebPreview: React.FC<WebPreviewProps> = ({ url: initialUrl = 'http://localhost:5173', onUrlChange, onScreenshot }) => {
  const [currentUrl, setCurrentUrl] = useState(initialUrl);
  const [inputUrl, setInputUrl] = useState(initialUrl);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const handleNavigate = (e: React.FormEvent) => {
    e.preventDefault();
    let targetUrl = inputUrl;
    if (!targetUrl.startsWith('http://') && !targetUrl.startsWith('https://')) {
      targetUrl = 'http://' + targetUrl;
    }
    setCurrentUrl(targetUrl);
    if (onUrlChange) onUrlChange(targetUrl);
  };

  const handleRefresh = () => {
    if (iframeRef.current) {
      iframeRef.current.src = currentUrl;
    }
  };

  return (
    <div className="web-preview-container" style={{
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      backgroundColor: '#1e1e2e',
      borderLeft: '1px solid #313244',
      overflow: 'hidden'
    }}>
      <div className="preview-toolbar" style={{
        padding: '8px 12px',
        background: 'rgba(30, 30, 46, 0.8)',
        backdropFilter: 'blur(10px)',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        borderBottom: '1px solid #313244'
      }}>
        <div style={{ display: 'flex', gap: '4px' }}>
          <button className="toolbar-btn" onClick={handleRefresh} title="Refresh">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
            </svg>
          </button>
        </div>
        
        <form onSubmit={handleNavigate} style={{ flex: 1 }}>
          <input 
            type="text" 
            value={inputUrl} 
            onChange={(e) => setInputUrl(e.target.value)}
            style={{
              width: '100%',
              backgroundColor: '#181825',
              border: '1px solid #313244',
              borderRadius: '4px',
              color: '#cdd6f4',
              padding: '4px 10px',
              fontSize: '12px',
              outline: 'none'
            }}
          />
        </form>
        
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <div style={{
            width: '8px',
            height: '8px',
            borderRadius: '50%',
            backgroundColor: '#a6e3a1',
            boxShadow: '0 0 8px #a6e3a1'
          }}></div>
          <span style={{ fontSize: '11px', color: '#a6adc8' }}>Live</span>
        </div>
      </div>
      
      <div className="preview-viewport" style={{ flex: 1, position: 'relative' }}>
        <iframe 
          ref={iframeRef}
          src={currentUrl} 
          title="App Preview"
          style={{
            width: '100%',
            height: '100%',
            border: 'none',
            backgroundColor: 'white'
          }}
        />
      </div>
    </div>
  );
};
