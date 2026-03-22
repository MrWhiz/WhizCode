import React, { useState, useEffect } from 'react';
import { specs } from '../../lib/tauri-api';

interface SpecSummary {
  name: string;
  slug: string;
  totalTasks: number;
  completedTasks: number;
  progress: number;
  updatedAt: string;
}

interface SpecTask {
  id: string;
  description: string;
  completed: boolean;
  subtasks?: SpecTask[];
}

interface SpecDetail {
  name: string;
  slug: string;
  requirements: string;
  design: string;
  tasks: SpecTask[];
  progress: number;
}

export const SpecsPanel: React.FC = () => {
  const [specsList, setSpecsList] = useState<SpecSummary[]>([]);
  const [selectedSpec, setSelectedSpec] = useState<SpecDetail | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    loadSpecs();
  }, []);

  const loadSpecs = async () => {
    setIsLoading(true);
    try {
      const result = await specs.list();
      setSpecsList(result);
    } catch (err) {
      console.error('Failed to load specs:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const loadSpecDetail = async (slug: string) => {
    setIsLoading(true);
    try {
      const result = await specs.get(slug);
      setSelectedSpec(result);
    } catch (err) {
      console.error('Failed to load spec detail:', err);
    } finally {
      setIsLoading(false);
    }
  };

  if (selectedSpec) {
    return (
      <div className="specs-panel detail-view">
        <div className="specs-header">
          <button className="back-btn" onClick={() => setSelectedSpec(null)}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M19 12H5M12 19l-7-7 7-7" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            Back to List
          </button>
          <h3>{selectedSpec.name}</h3>
        </div>

        <div className="spec-progress-bar">
          <div className="progress-fill" style={{ width: `${selectedSpec.progress}%` }}></div>
        </div>

        <div className="spec-sections">
          <div className="spec-section">
            <h4>Requirements</h4>
            <div className="spec-content">
              {selectedSpec.requirements.split('\n').map((line, i) => (
                <p key={i}>{line}</p>
              ))}
            </div>
          </div>

          <div className="spec-section">
            <h4>Implementation Tasks</h4>
            <div className="tasks-list">
              {selectedSpec.tasks.map((task) => (
                <div key={task.id} className={`task-item ${task.completed ? 'completed' : ''}`}>
                  <input type="checkbox" checked={task.completed} readOnly />
                  <span>{task.description}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="specs-panel">
      <div className="specs-header">
        <h3>Feature Specs</h3>
        <button className="refresh-btn" onClick={loadSpecs} disabled={isLoading}>
           <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" strokeLinecap="round" strokeLinejoin="round"/>
           </svg>
        </button>
      </div>

      <div className="specs-list">
        {isLoading && <div className="loading">Loading specs...</div>}
        {!isLoading && specsList.length === 0 && (
          <div className="empty-state">
            <p>No specs found.</p>
            <p className="hint">Ask the AI to "create a spec for..."</p>
          </div>
        )}
        {specsList.map((spec) => (
          <div 
            key={spec.slug} 
            className="spec-card"
            onClick={() => loadSpecDetail(spec.slug)}
          >
            <div className="spec-card-header">
              <span className="spec-name">{spec.name}</span>
              <span className="spec-status">{spec.progress}%</span>
            </div>
            <div className="spec-progress-mini">
              <div className="progress-fill" style={{ width: `${spec.progress}%` }}></div>
            </div>
            <div className="spec-meta">
              <span>{spec.completedTasks}/{spec.totalTasks} tasks</span>
              <span>Updated {new Date(spec.updatedAt).toLocaleDateString()}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
