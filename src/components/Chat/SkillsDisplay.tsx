import React from "react";
import "./SkillsDisplay.css";

interface SelectedSkill {
  name: string;
  confidence: number;
  capabilities: string[];
}

interface SkillsDisplayProps {
  skills: SelectedSkill[];
  isLoading?: boolean;
  error?: string | null;
}

/**
 * Component to display selected skills for the current query
 *
 * Shows which skills are being used by the agent with their confidence scores
 * and capabilities.
 */
export const SkillsDisplay: React.FC<SkillsDisplayProps> = ({
  skills,
  isLoading = false,
  error = null,
}) => {
  if (error) {
    return (
      <div className="skills-display error">
        <span className="error-icon">⚠️</span>
        <span className="error-text">{error}</span>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="skills-display loading">
        <div className="spinner"></div>
        <span>Analyzing skills...</span>
      </div>
    );
  }

  if (!skills || skills.length === 0) {
    return null;
  }

  return (
    <div className="skills-display">
      <div className="skills-header">
        <span className="skills-icon">⚡</span>
        <span className="skills-title">
          Using {skills.length} skill{skills.length !== 1 ? "s" : ""}
        </span>
      </div>

      <div className="skills-list">
        {skills.map((skill) => (
          <div key={skill.name} className="skill-badge">
            <div className="skill-name">{skill.name}</div>
            <div className="skill-confidence">
              {Math.round(skill.confidence * 100)}%
            </div>
            {skill.capabilities.length > 0 && (
              <div className="skill-capabilities">
                {skill.capabilities.slice(0, 2).map((cap) => (
                  <span key={cap} className="capability-tag">
                    {cap}
                  </span>
                ))}
                {skill.capabilities.length > 2 && (
                  <span className="capability-tag more">
                    +{skill.capabilities.length - 2}
                  </span>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default SkillsDisplay;
