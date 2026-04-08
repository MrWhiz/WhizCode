import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./RepositoryConfig.css";

interface RepositoryConfigProps {
  onClose: () => void;
  onSave: () => void;
}

interface SkillsConfig {
  repository_url: string;
  max_skills: number;
  confidence_threshold: number;
  cache_ttl: number;
  enabled_skills: string[];
}

const RepositoryConfig: React.FC<RepositoryConfigProps> = ({
  onClose,
  onSave,
}) => {
  const [config, setConfig] = useState<SkillsConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [urlError, setUrlError] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      setLoading(true);
      const cfg = await invoke<SkillsConfig>("get_skills_config");
      setConfig(cfg);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to load configuration: ${errorMessage}`);
    } finally {
      setLoading(false);
    }
  };

  const validateUrl = (url: string): boolean => {
    try {
      new URL(url);
      setUrlError(null);
      return true;
    } catch {
      setUrlError("Invalid URL format");
      return false;
    }
  };

  const handleUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const url = e.target.value;
    if (config) {
      setConfig({ ...config, repository_url: url });
      if (url) {
        validateUrl(url);
      }
    }
  };

  const handleMaxSkillsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(e.target.value, 10);
    if (config && value > 0) {
      setConfig({ ...config, max_skills: value });
    }
  };

  const handleThresholdChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseFloat(e.target.value);
    if (config && value >= 0 && value <= 1) {
      setConfig({ ...config, confidence_threshold: value });
    }
  };

  const handleSave = async () => {
    if (!config) return;

    if (!validateUrl(config.repository_url)) {
      return;
    }

    try {
      setSaving(true);
      setError(null);
      await invoke("set_repository_url", { url: config.repository_url });
      onSave();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to save configuration: ${errorMessage}`);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="repository-config-overlay">
        <div className="repository-config-modal">
          <div className="modal-loading">
            <div className="spinner"></div>
            <p>Loading configuration...</p>
          </div>
        </div>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="repository-config-overlay">
        <div className="repository-config-modal">
          <div className="modal-error">
            <p>Failed to load configuration</p>
            <button onClick={onClose} className="btn-secondary">
              Close
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="repository-config-overlay" onClick={onClose}>
      <div
        className="repository-config-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>Skills Configuration</h3>
          <button className="modal-close" onClick={onClose}>
            ✕
          </button>
        </div>

        {error && (
          <div className="modal-error-banner">
            <span className="error-icon">⚠</span>
            <span className="error-message">{error}</span>
          </div>
        )}

        <div className="modal-content">
          <div className="config-section">
            <label htmlFor="repository-url" className="config-label">
              Repository URL
            </label>
            <input
              id="repository-url"
              type="text"
              value={config.repository_url}
              onChange={handleUrlChange}
              placeholder="https://github.com/alirezarezvani/claude-skills"
              className={`config-input ${urlError ? "error" : ""}`}
            />
            {urlError && <span className="input-error">{urlError}</span>}
            <p className="config-help">
              URL to the skills repository. Skills will be discovered from this
              location.
            </p>
          </div>

          <div className="config-section">
            <label htmlFor="max-skills" className="config-label">
              Maximum Skills to Select
            </label>
            <input
              id="max-skills"
              type="number"
              min="1"
              max="20"
              value={config.max_skills}
              onChange={handleMaxSkillsChange}
              className="config-input"
            />
            <p className="config-help">
              Maximum number of skills to select for a query (1-20).
            </p>
          </div>

          <div className="config-section">
            <label htmlFor="confidence-threshold" className="config-label">
              Confidence Threshold
            </label>
            <div className="threshold-input-group">
              <input
                id="confidence-threshold"
                type="range"
                min="0"
                max="1"
                step="0.1"
                value={config.confidence_threshold}
                onChange={handleThresholdChange}
                className="config-slider"
              />
              <span className="threshold-value">
                {(config.confidence_threshold * 100).toFixed(0)}%
              </span>
            </div>
            <p className="config-help">
              Minimum confidence score for skill selection (0-100%).
            </p>
          </div>

          <div className="config-section">
            <div className="config-info">
              <h4>Current Settings</h4>
              <ul>
                <li>
                  <strong>Enabled Skills:</strong>{" "}
                  {config.enabled_skills.length}
                </li>
                <li>
                  <strong>Cache TTL:</strong>{" "}
                  {(config.cache_ttl / 3600).toFixed(0)} hours
                </li>
              </ul>
            </div>
          </div>
        </div>

        <div className="modal-footer">
          <button onClick={onClose} className="btn-secondary">
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={saving || !!urlError}
            className="btn-primary"
          >
            {saving ? "Saving..." : "Save & Refresh"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default RepositoryConfig;
