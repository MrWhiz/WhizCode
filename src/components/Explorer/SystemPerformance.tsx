import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface SystemStats {
  cpuUsage: number;
  cpuTemp: number | null;
  ramUsage: {
    used: number;
    total: number;
    percent: number;
  };
  gpu: {
    usage: number | null;
    temp: number | null;
    name: string | null;
    memoryUsed: number | null;
    memoryTotal: number | null;
  };
}

const SystemPerformance: React.FC = () => {
  const [stats, setStats] = useState<SystemStats | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setupListener = async () => {
      try {
        unlisten = await listen('system:status', (event: any) => {
          setStats(event.payload as SystemStats);
        });
      } catch (err) {
        console.error('Failed to setup system:status listener:', err);
      }
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  if (!stats) return null;

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  };

  return (
    <div className="system-performance">
      <div className="system-performance-header">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
        </svg>
        <span>SYSTEM PERFORMANCE</span>
      </div>
      
      <div className="system-stats-grid">
        <div className="system-stat-item">
          <div className="stat-main">
            <span className="stat-label">CPU Usage</span>
            <span className="stat-value">{stats.cpuUsage}%</span>
          </div>
          <div className="stat-bar-container">
            <div 
              className="stat-bar" 
              style={{ 
                width: `${stats.cpuUsage}%`, 
                backgroundColor: stats.cpuUsage > 80 ? '#ef4444' : 'var(--accent-primary)' 
              }} 
            />
          </div>
          {stats.cpuTemp !== null && (
            <div className="stat-extra">
              <span className="stat-temp">Temp: {stats.cpuTemp}°C</span>
            </div>
          )}
        </div>

        <div className="system-stat-item">
          <div className="stat-main">
            <span className="stat-label">RAM Usage</span>
            <span className="stat-value">{stats.ramUsage.percent}%</span>
          </div>
          <div className="stat-bar-container">
            <div 
              className="stat-bar" 
              style={{ 
                width: `${stats.ramUsage.percent}%`, 
                backgroundColor: stats.ramUsage.percent > 85 ? '#ef4444' : 'var(--accent-primary)' 
              }} 
            />
          </div>
          <div className="stat-extra">
            <span className="stat-sub">{formatBytes(stats.ramUsage.used)} / {formatBytes(stats.ramUsage.total)}</span>
          </div>
        </div>

        {stats.gpu.name && (
          <div className="system-stat-item">
            <div className="stat-main">
              <span className="stat-label">GPU: {stats.gpu.name.length > 20 ? stats.gpu.name.substring(0, 17) + '...' : stats.gpu.name}</span>
              <span className="stat-value">{stats.gpu.usage !== null ? `${stats.gpu.usage}%` : 'N/A'}</span>
            </div>
            <div className="stat-bar-container">
              <div 
                className="stat-bar" 
                style={{ 
                  width: `${stats.gpu.usage || 0}%`, 
                  backgroundColor: (stats.gpu.usage || 0) > 80 ? '#ef4444' : '#10b981' 
                }} 
              />
            </div>
            {(stats.gpu.temp !== null || stats.gpu.memoryUsed !== null) && (
              <div className="stat-extra">
                {stats.gpu.temp !== null && <span className="stat-temp">Temp: {stats.gpu.temp}°C </span>}
                {stats.gpu.memoryUsed !== null && (
                  <span className="stat-sub">VRAM: {(stats.gpu.memoryUsed / 1024).toFixed(1)} GB</span>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default SystemPerformance;
