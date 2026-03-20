import React, { useState, useEffect } from 'react';

interface BrainMetric {
  label: string;
  value: string | number;
  status: 'healthy' | 'warning' | 'critical';
  trend?: 'up' | 'down' | 'stable';
  details?: string;
}

interface BrainHealthStats {
  overallScore: number;
  components: {
    planning: BrainMetric;
    memory: BrainMetric;
    retrieval: BrainMetric;
    recovery: BrainMetric;
    knowledge: BrainMetric;
  };
}

export const BrainDashboard: React.FC = () => {
  const [health, setHealth] = useState<BrainHealthStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'memory' | 'recovery' | 'learning'>('overview');

  const loadData = async () => {
    setLoading(true);
    try {
      const ipc = (window as any).ipcRenderer;
      if (!ipc) return;

      const [
        memoryStats,
        vectorStats,
        errorStats,
        learningMetrics
      ] = await Promise.all([
        ipc.invoke('ai:get-context-memory-stats'),
        ipc.invoke('vector:get-index-stats'),
        ipc.invoke('error-recovery:get-statistics'),
        ipc.invoke('ai:get-learning-metrics')
      ]);

      // Synthesize health data
      const recoveryRate = errorStats?.recoverySuccessRate || 0;
      const successRate = learningMetrics?.successRate || 0;
      
      const synthesized: BrainHealthStats = {
        overallScore: Math.round((recoveryRate * 0.3 + successRate * 0.7) * 100),
        components: {
          planning: {
            label: 'Task Planning',
            value: 'LLM-Driven',
            status: 'healthy',
            trend: 'up',
            details: 'Strategic Planner is active with LLM classification.'
          },
          memory: {
            label: 'Context Memory',
            value: memoryStats?.codePatterns || 0,
            status: (memoryStats?.codePatterns || 0) > 0 ? 'healthy' : 'warning',
            details: `${memoryStats?.codePatterns || 0} patterns indexed in memory.`
          },
          retrieval: {
            label: 'Vector Retrieval',
            value: vectorStats?.totalChunks || 0,
            status: vectorStats?.totalChunks > 0 ? 'healthy' : 'warning',
            details: `${vectorStats?.totalChunks || 0} code chunks in vector index.`
          },
          recovery: {
            label: 'Hero Recovery',
            value: `${Math.round(recoveryRate * 100)}%`,
            status: recoveryRate > 0.7 ? 'healthy' : recoveryRate > 0.4 ? 'warning' : 'critical',
            details: `Autonomous healing success rate is ${Math.round(recoveryRate * 100)}%.`
          },
          knowledge: {
            label: 'KI Distillation',
            value: learningMetrics?.totalInteractions || 0,
            status: 'healthy',
            details: `${learningMetrics?.totalInteractions || 0} total interactions distilled.`
          }
        }
      };

      setHealth(synthesized);
    } catch (error) {
      console.error('Failed to load brain health data:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 30000); // Pool every 30s
    return () => clearInterval(interval);
  }, []);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'healthy': return 'var(--success-color, #4ade80)';
      case 'warning': return 'var(--warning-color, #fbbf24)';
      case 'critical': return 'var(--error-color, #f87171)';
      default: return 'var(--text-secondary)';
    }
  };

  return (
    <div className="brain-dashboard">
      <div className="brain-header">
        <div className="brain-title">
          <div className="brain-icon-pulse">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z"/>
              <path d="M12 6v6l4 2"/>
            </svg>
          </div>
          <h3>Brain Health</h3>
        </div>
        <div className="overall-score-badge">
          <span className="score-label">COGNITIVE STATUS</span>
          <span className="score-value" style={{ color: health && health.overallScore > 70 ? '#4ade80' : '#fbbf24' }}>
            {health ? `${health.overallScore}%` : '--'}
          </span>
        </div>
      </div>

      <div className="health-grid">
        {health && Object.entries(health.components).map(([key, metric]) => (
          <div key={key} className={`health-card ${metric.status}`}>
            <div className="card-header">
              <span className="metric-label">{metric.label}</span>
              <div className={`status-dot ${metric.status}`} />
            </div>
            <div className="metric-main">
              <span className="metric-value">{metric.value}</span>
              {metric.trend && <span className={`trend-icon ${metric.trend}`}>{metric.trend === 'up' ? '↗' : '↘'}</span>}
            </div>
            <p className="metric-details">{metric.details}</p>
          </div>
        ))}
      </div>

      <div className="brain-tabs-container">
        <div className="brain-tabs">
          <button className={activeTab === 'overview' ? 'active' : ''} onClick={() => setActiveTab('overview')}>Overview</button>
          <button className={activeTab === 'memory' ? 'active' : ''} onClick={() => setActiveTab('memory')}>Memory</button>
          <button className={activeTab === 'recovery' ? 'active' : ''} onClick={() => setActiveTab('recovery')}>Recovery</button>
        </div>
        <div className="brain-tab-content">
          {activeTab === 'overview' && (
            <div className="proactive-tasks">
              <h4>Active Cognitive Processes</h4>
              <div className="process-list">
                <div className="process-item">
                  <div className="process-indicator active" />
                  <span>Vector Indexing: Idle</span>
                </div>
                <div className="process-item">
                  <div className="process-indicator active" />
                  <span>Strategic Planning: Ready</span>
                </div>
                <div className="process-item">
                  <div className="process-indicator pulsate" />
                  <span>Learning System: Monitoring</span>
                </div>
              </div>
            </div>
          )}
          {activeTab === 'memory' && <p className="tab-placeholder">Context memory heatmap coming soon...</p>}
          {activeTab === 'recovery' && <p className="tab-placeholder">Recent autonomous fixes logging...</p>}
        </div>
      </div>
      
      <button className="brain-refresh-btn" onClick={loadData} disabled={loading}>
        {loading ? 'Optimizing Neurons...' : 'Force Cognitive Sync'}
      </button>
    </div>
  );
};
