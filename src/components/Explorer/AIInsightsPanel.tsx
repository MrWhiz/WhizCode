import React, { useState, useEffect } from 'react';
import { ai, vector, cache, errorRecovery, mcp, system } from '../../lib/tauri-api';

interface LearningInsight {
  type: 'pattern' | 'preference' | 'strategy' | 'error';
  confidence: number;
  description: string;
  recommendation: string;
  evidence: string[];
}

interface LearningMetrics {
  totalInteractions: number;
  successRate: number;
  averageResponseTime: number;
  userSatisfactionScore: number;
  improvementTrends: {
    period: string;
    metric: string;
    change: number;
  }[];
}

interface CodeMetrics {
  complexity: number;
  maintainability: number;
  testability: number;
  coupling: number;
  cohesion: number;
  linesOfCode: number;
  technicalDebt: number;
}

interface ContextMemoryStats {
  codePatterns: number;
  userPreferences: number;
  errorPatterns: number;
  successfulStrategies: number;
  sessionHistory: number;
}

interface VectorIndexStats {
  totalChunks: number;
  totalFiles: number;
  totalSymbols: number;
  lastIndexTime: Date | null;
  isIndexing: boolean;
}

interface CacheStats {
  totalEntries: number;
  totalSize: number;
  hitRate: number;
  missRate: number;
  totalHits: number;
  totalMisses: number;
  oldestEntry?: Date;
  newestEntry?: Date;
  mostAccessedTool?: string;
}

interface ErrorRecoveryStats {
  totalErrors: number;
  errorsByType: Record<string, number>;
  recoverySuccessRate: number;
  mostCommonErrors: Array<{ type: string; count: number }>;
}

interface MCPMarketplace {
  powers: Array<{
    id: string;
    name: string;
    description: string;
    category: string;
    installed: boolean;
    enabled: boolean;
  }>;
  categories: string[];
  featured: string[];
}

export const AIInsightsPanel: React.FC = () => {
  const [insights, setInsights] = useState<LearningInsight[]>([]);
  const [metrics, setMetrics] = useState<LearningMetrics | null>(null);
  const [codeMetrics, setCodeMetrics] = useState<CodeMetrics | null>(null);
  const [memoryStats, setMemoryStats] = useState<ContextMemoryStats | null>(null);
  const [vectorStats, setVectorStats] = useState<VectorIndexStats | null>(null);
  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null);
  const [errorStats, setErrorStats] = useState<ErrorRecoveryStats | null>(null);
  const [mcpMarketplace, setMcpMarketplace] = useState<MCPMarketplace | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'insights' | 'metrics' | 'code' | 'memory' | 'vector' | 'cache' | 'errors' | 'mcp'>('insights');

  const loadData = async () => {
    setLoading(true);
    try {
      const [
        insightsData, 
        metricsData, 
        memoryData,
        vectorData,
        cacheData,
        errorData,
        mcpData
      ] = await Promise.all([
        ai.getLearningInsights(),
        ai.getLearningMetrics(),
        ai.getContextMemoryStats(),
        vector.getIndexStats(),
        cache.getStats(),
        errorRecovery.getStatistics(),
        mcp.getMarketplace()
      ]);

      setInsights(insightsData || []);
      setMetrics(metricsData);
      setMemoryStats(memoryData);
      setVectorStats(vectorData);
      setCacheStats(cacheStats as any);
      setErrorStats(errorData);
      setMcpMarketplace(mcpData);

      // Get code metrics if workspace is available
      const workspacePath = await system.getWorkspacePath();
      if (workspacePath) {
        const codeMetricsData = await ai.getCodeMetrics(workspacePath);
        setCodeMetrics(codeMetricsData);
      }
    } catch (error) {
      console.error('Failed to load AI insights:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return '#4ade80'; // green
    if (confidence >= 0.6) return '#fbbf24'; // yellow
    return '#f87171'; // red
  };

  const getMetricColor = (value: number, isHighGood: boolean = true) => {
    const threshold = isHighGood ? 70 : 30;
    if (isHighGood) {
      if (value >= threshold) return '#4ade80';
      if (value >= threshold * 0.7) return '#fbbf24';
      return '#f87171';
    } else {
      if (value <= threshold) return '#4ade80';
      if (value <= threshold * 1.5) return '#fbbf24';
      return '#f87171';
    }
  };

  return (
    <div className="ai-insights-panel">
      <div className="panel-header">
        <h3>AI Insights</h3>
        <button onClick={loadData} disabled={loading} className="refresh-btn">
          {loading ? '⟳' : '↻'}
        </button>
      </div>

      <div className="tab-bar">
        <button 
          className={`tab ${activeTab === 'insights' ? 'active' : ''}`}
          onClick={() => setActiveTab('insights')}
        >
          Insights ({insights.length})
        </button>
        <button 
          className={`tab ${activeTab === 'metrics' ? 'active' : ''}`}
          onClick={() => setActiveTab('metrics')}
        >
          Learning
        </button>
        <button 
          className={`tab ${activeTab === 'code' ? 'active' : ''}`}
          onClick={() => setActiveTab('code')}
        >
          Code Quality
        </button>
        <button 
          className={`tab ${activeTab === 'memory' ? 'active' : ''}`}
          onClick={() => setActiveTab('memory')}
        >
          Memory
        </button>
        <button 
          className={`tab ${activeTab === 'vector' ? 'active' : ''}`}
          onClick={() => setActiveTab('vector')}
        >
          Vector Search
        </button>
        <button 
          className={`tab ${activeTab === 'cache' ? 'active' : ''}`}
          onClick={() => setActiveTab('cache')}
        >
          Cache
        </button>
        <button 
          className={`tab ${activeTab === 'errors' ? 'active' : ''}`}
          onClick={() => setActiveTab('errors')}
        >
          Error Recovery
        </button>
        <button 
          className={`tab ${activeTab === 'mcp' ? 'active' : ''}`}
          onClick={() => setActiveTab('mcp')}
        >
          MCP Powers
        </button>
      </div>

      <div className="tab-content">
        {activeTab === 'insights' && (
          <div className="insights-tab">
            {insights.length === 0 ? (
              <div className="empty-state">
                <p>No insights available yet.</p>
                <p>Interact with the AI to generate insights.</p>
              </div>
            ) : (
              <div className="insights-list">
                {insights.map((insight, index) => (
                  <div key={index} className="insight-card">
                    <div className="insight-header">
                      <span className={`insight-type ${insight.type}`}>
                        {insight.type.toUpperCase()}
                      </span>
                      <div 
                        className="confidence-bar"
                        style={{ 
                          backgroundColor: getConfidenceColor(insight.confidence),
                          width: `${insight.confidence * 100}%`
                        }}
                      />
                    </div>
                    <p className="insight-description">{insight.description}</p>
                    <p className="insight-recommendation">💡 {insight.recommendation}</p>
                    {insight.evidence.length > 0 && (
                      <details className="insight-evidence">
                        <summary>Evidence ({insight.evidence.length})</summary>
                        <ul>
                          {insight.evidence.map((evidence, i) => (
                            <li key={i}>{evidence}</li>
                          ))}
                        </ul>
                      </details>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === 'metrics' && (
          <div className="metrics-tab">
            {metrics ? (
              <div className="metrics-grid">
                <div className="metric-card">
                  <h4>Total Interactions</h4>
                  <div className="metric-value">{metrics.totalInteractions}</div>
                </div>
                <div className="metric-card">
                  <h4>Success Rate</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(metrics.successRate * 100) }}
                  >
                    {(metrics.successRate * 100).toFixed(1)}%
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Avg Response Time</h4>
                  <div className="metric-value">
                    {metrics.averageResponseTime.toFixed(1)}s
                  </div>
                </div>
                <div className="metric-card">
                  <h4>User Satisfaction</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(metrics.userSatisfactionScore * 100) }}
                  >
                    {(metrics.userSatisfactionScore * 100).toFixed(1)}%
                  </div>
                </div>
                {metrics.improvementTrends.length > 0 && (
                  <div className="trends-section">
                    <h4>Improvement Trends</h4>
                    {metrics.improvementTrends.map((trend, index) => (
                      <div key={index} className="trend-item">
                        <span className="trend-metric">{trend.metric.replace('_', ' ')}</span>
                        <span 
                          className={`trend-change ${trend.change >= 0 ? 'positive' : 'negative'}`}
                        >
                          {trend.change >= 0 ? '+' : ''}{(trend.change * 100).toFixed(1)}%
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="empty-state">
                <p>No learning metrics available yet.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'code' && (
          <div className="code-tab">
            {codeMetrics ? (
              <div className="metrics-grid">
                <div className="metric-card">
                  <h4>Complexity</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.complexity, false) }}
                  >
                    {codeMetrics.complexity.toFixed(1)}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Maintainability</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.maintainability) }}
                  >
                    {codeMetrics.maintainability.toFixed(1)}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Testability</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.testability) }}
                  >
                    {codeMetrics.testability.toFixed(1)}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Coupling</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.coupling, false) }}
                  >
                    {codeMetrics.coupling.toFixed(1)}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Cohesion</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.cohesion) }}
                  >
                    {codeMetrics.cohesion.toFixed(1)}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Lines of Code</h4>
                  <div className="metric-value">
                    {codeMetrics.linesOfCode.toLocaleString()}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Technical Debt</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(codeMetrics.technicalDebt, false) }}
                  >
                    {codeMetrics.technicalDebt}
                  </div>
                </div>
              </div>
            ) : (
              <div className="empty-state">
                <p>No code metrics available.</p>
                <p>Open a workspace to analyze code quality.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'memory' && (
          <div className="memory-tab">
            {memoryStats ? (
              <div className="memory-stats">
                <div className="stat-item">
                  <span className="stat-label">Code Patterns</span>
                  <span className="stat-value">{memoryStats.codePatterns}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">User Preferences</span>
                  <span className="stat-value">{memoryStats.userPreferences}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Error Patterns</span>
                  <span className="stat-value">{memoryStats.errorPatterns}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Successful Strategies</span>
                  <span className="stat-value">{memoryStats.successfulStrategies}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Session History</span>
                  <span className="stat-value">{memoryStats.sessionHistory}</span>
                </div>
              </div>
            ) : (
              <div className="empty-state">
                <p>No memory statistics available.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'vector' && (
          <div className="vector-tab">
            {vectorStats ? (
              <div className="metrics-grid">
                <div className="metric-card">
                  <h4>Total Chunks</h4>
                  <div className="metric-value">{vectorStats.totalChunks.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Indexed Files</h4>
                  <div className="metric-value">{vectorStats.totalFiles.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Symbols</h4>
                  <div className="metric-value">{vectorStats.totalSymbols.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Last Indexed</h4>
                  <div className="metric-value" style={{ fontSize: '12px' }}>
                    {vectorStats.lastIndexTime ? new Date(vectorStats.lastIndexTime).toLocaleString() : 'Never'}
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Status</h4>
                  <div className="metric-value" style={{ 
                    color: vectorStats.isIndexing ? '#fbbf24' : '#4ade80',
                    fontSize: '14px'
                  }}>
                    {vectorStats.isIndexing ? 'Indexing...' : 'Ready'}
                  </div>
                </div>
              </div>
            ) : (
              <div className="empty-state">
                <p>No vector search statistics available.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'cache' && (
          <div className="cache-tab">
            {cacheStats ? (
              <div className="metrics-grid">
                <div className="metric-card">
                  <h4>Cache Entries</h4>
                  <div className="metric-value">{cacheStats.totalEntries.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Cache Size</h4>
                  <div className="metric-value" style={{ fontSize: '12px' }}>
                    {(cacheStats.totalSize / 1024 / 1024).toFixed(1)} MB
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Hit Rate</h4>
                  <div 
                    className="metric-value"
                    style={{ color: getMetricColor(cacheStats.hitRate * 100) }}
                  >
                    {(cacheStats.hitRate * 100).toFixed(1)}%
                  </div>
                </div>
                <div className="metric-card">
                  <h4>Total Hits</h4>
                  <div className="metric-value">{cacheStats.totalHits.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Total Misses</h4>
                  <div className="metric-value">{cacheStats.totalMisses.toLocaleString()}</div>
                </div>
                <div className="metric-card">
                  <h4>Most Used Tool</h4>
                  <div className="metric-value" style={{ fontSize: '11px' }}>
                    {cacheStats.mostAccessedTool || 'None'}
                  </div>
                </div>
              </div>
            ) : (
              <div className="empty-state">
                <p>No cache statistics available.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'errors' && (
          <div className="errors-tab">
            {errorStats ? (
              <div className="error-stats">
                <div className="metrics-grid">
                  <div className="metric-card">
                    <h4>Total Errors</h4>
                    <div className="metric-value">{errorStats.totalErrors.toLocaleString()}</div>
                  </div>
                  <div className="metric-card">
                    <h4>Recovery Success Rate</h4>
                    <div 
                      className="metric-value"
                      style={{ color: getMetricColor(errorStats.recoverySuccessRate * 100) }}
                    >
                      {(errorStats.recoverySuccessRate * 100).toFixed(1)}%
                    </div>
                  </div>
                </div>
                
                {errorStats.mostCommonErrors.length > 0 && (
                  <div className="common-errors">
                    <h4>Most Common Errors</h4>
                    {errorStats.mostCommonErrors.slice(0, 5).map((error, index) => (
                      <div key={index} className="error-item">
                        <span className="error-type">{error.type.replace('_', ' ')}</span>
                        <span className="error-count">{error.count}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="empty-state">
                <p>No error recovery statistics available.</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'mcp' && (
          <div className="mcp-tab">
            {mcpMarketplace ? (
              <div className="mcp-marketplace">
                <div className="marketplace-stats">
                  <div className="stat-item">
                    <span className="stat-label">Available Powers</span>
                    <span className="stat-value">{mcpMarketplace.powers.length}</span>
                  </div>
                  <div className="stat-item">
                    <span className="stat-label">Installed</span>
                    <span className="stat-value">
                      {mcpMarketplace.powers.filter(p => p.installed).length}
                    </span>
                  </div>
                  <div className="stat-item">
                    <span className="stat-label">Enabled</span>
                    <span className="stat-value">
                      {mcpMarketplace.powers.filter(p => p.enabled).length}
                    </span>
                  </div>
                </div>
                
                <div className="power-categories">
                  <h4>Categories</h4>
                  <div className="category-list">
                    {mcpMarketplace.categories.map(category => (
                      <span key={category} className="category-tag">
                        {category}
                      </span>
                    ))}
                  </div>
                </div>
                
                <div className="featured-powers">
                  <h4>Featured Powers</h4>
                  {mcpMarketplace.powers
                    .filter(power => mcpMarketplace.featured.includes(power.id))
                    .map(power => (
                      <div key={power.id} className="power-item">
                        <div className="power-info">
                          <span className="power-name">{power.name}</span>
                          <span className="power-description">{power.description}</span>
                        </div>
                        <div className="power-status">
                          {power.enabled ? (
                            <span className="status-enabled">Enabled</span>
                          ) : power.installed ? (
                            <span className="status-installed">Installed</span>
                          ) : (
                            <span className="status-available">Available</span>
                          )}
                        </div>
                      </div>
                    ))}
                </div>
              </div>
            ) : (
              <div className="empty-state">
                <p>No MCP marketplace data available.</p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};