import React, { useState } from "react";
import "./SkillItem.css";

interface Skill {
  manifest: {
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    requirements: string[];
    dependencies: Array<{ name: string; version: string }>;
  };
  path: string;
  status: { status: string; reason?: string };
  enabled: boolean;
  cached: boolean;
}

interface SkillItemProps {
  skill: Skill;
  isSelected: boolean;
  onToggle: (skillName: string, enabled: boolean) => void;
  onSelect: (skillName: string) => void;
}

const SkillItem: React.FC<SkillItemProps> = ({
  skill,
  isSelected,
  onToggle,
  onSelect,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const handleToggle = (e: React.ChangeEvent<HTMLInputElement>) => {
    e.stopPropagation();
    onToggle(skill.manifest.name, e.target.checked);
  };

  const handleClick = () => {
    onSelect(skill.manifest.name);
    setIsExpanded(!isExpanded);
  };

  const isAvailable = skill.status.status === "Available";
  const isUnavailable = skill.status.status === "Unavailable";
  const isDisabled = skill.status.status === "Disabled";

  return (
    <div className={`skill-item ${isSelected ? "selected" : ""}`}>
      <div className="skill-item-header" onClick={handleClick}>
        <div className="skill-item-checkbox">
          <input
            type="checkbox"
            checked={skill.enabled}
            onChange={handleToggle}
            disabled={isUnavailable || isDisabled}
            title={isUnavailable ? `Unavailable: ${skill.status.reason}` : ""}
          />
        </div>

        <div className="skill-item-info">
          <div className="skill-item-title">
            <span className="skill-name">{skill.manifest.name}</span>
            <span className="skill-version">v{skill.manifest.version}</span>
            {isUnavailable && (
              <span
                className="skill-status-badge unavailable"
                title={skill.status.reason}
              >
                ⚠ Unavailable
              </span>
            )}
            {isDisabled && (
              <span className="skill-status-badge disabled">⊘ Disabled</span>
            )}
            {skill.cached && (
              <span className="skill-cached-badge" title="Loaded from cache">
                💾 Cached
              </span>
            )}
          </div>
          <div className="skill-description">{skill.manifest.description}</div>
          <div className="skill-meta">
            <span className="skill-author">by {skill.manifest.author}</span>
          </div>
        </div>

        <div className="skill-item-expand">
          <span className={`expand-icon ${isExpanded ? "expanded" : ""}`}>
            ▶
          </span>
        </div>
      </div>

      {isExpanded && (
        <div className="skill-item-details">
          <div className="skill-details-section">
            <h4>Capabilities</h4>
            <div className="skill-capabilities">
              {skill.manifest.capabilities.length > 0 ? (
                skill.manifest.capabilities.map((cap, idx) => (
                  <span key={idx} className="capability-tag">
                    {cap}
                  </span>
                ))
              ) : (
                <span className="empty-text">No capabilities</span>
              )}
            </div>
          </div>

          {skill.manifest.requirements.length > 0 && (
            <div className="skill-details-section">
              <h4>Requirements</h4>
              <div className="skill-requirements">
                {skill.manifest.requirements.map((req, idx) => (
                  <span key={idx} className="requirement-tag">
                    {req}
                  </span>
                ))}
              </div>
            </div>
          )}

          {skill.manifest.dependencies.length > 0 && (
            <div className="skill-details-section">
              <h4>Dependencies</h4>
              <div className="skill-dependencies">
                {skill.manifest.dependencies.map((dep, idx) => (
                  <div key={idx} className="dependency-item">
                    <span className="dependency-name">{dep.name}</span>
                    <span className="dependency-version">{dep.version}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="skill-details-section">
            <h4>Details</h4>
            <div className="skill-details-grid">
              <div className="detail-item">
                <span className="detail-label">Path:</span>
                <span className="detail-value">{skill.path}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Status:</span>
                <span
                  className={`detail-value status-${skill.status.status.toLowerCase()}`}
                >
                  {skill.status.status}
                </span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Enabled:</span>
                <span className="detail-value">
                  {skill.enabled ? "Yes" : "No"}
                </span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Cached:</span>
                <span className="detail-value">
                  {skill.cached ? "Yes" : "No"}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default SkillItem;
