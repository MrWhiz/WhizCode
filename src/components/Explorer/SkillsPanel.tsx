import React, { useState, useEffect } from "react";
import "./SkillsPanel.css";
import { useSkills } from "../../hooks/useSkills";
import { invoke } from "@tauri-apps/api/core";

interface SkillsPanelState {
  refreshing: boolean;
  selectedSkill: string | null;
  analyzingWorkspace: boolean;
}

interface WorkspaceSkill {
  name: string;
  description: string;
  version: string;
  author: string;
  enabled: boolean;
  confidence?: number;
}

const SkillsPanel: React.FC = () => {
  const { skills, loading, error, loadSkills, refreshSkills } = useSkills();
  const [state, setState] = useState<SkillsPanelState>({
    refreshing: false,
    selectedSkill: null,
    analyzingWorkspace: false,
  });
  const [workspaceSkills, setWorkspaceSkills] = useState<WorkspaceSkill[]>([]);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);

  // Analyze workspace on component mount
  useEffect(() => {
    analyzeWorkspaceSkills();
  }, []);

  const analyzeWorkspaceSkills = async () => {
    try {
      setState((prev) => ({ ...prev, analyzingWorkspace: true }));
      setWorkspaceError(null);

      // Get workspace context (simplified - in real app would get from workspace state)
      const workspacePath = "/current/workspace"; // Placeholder
      const projectType = "typescript"; // Placeholder - would detect from files
      const files = ["src/main.ts", "src/components/App.tsx"]; // Placeholder

      const selectedSkills = await invoke<WorkspaceSkill[]>(
        "analyze_workspace_skills",
        {
          workspace_path: workspacePath,
          project_type: projectType,
          files: files,
        },
      );

      setWorkspaceSkills(selectedSkills);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setWorkspaceError(`Failed to analyze workspace: ${errorMessage}`);
      console.error("Error analyzing workspace skills:", err);
    } finally {
      setState((prev) => ({ ...prev, analyzingWorkspace: false }));
    }
  };

  const handleRefresh = async () => {
    try {
      setState((prev) => ({ ...prev, refreshing: true }));
      await refreshSkills();
      // Re-analyze workspace after refresh
      await analyzeWorkspaceSkills();
    } finally {
      setState((prev) => ({ ...prev, refreshing: false }));
    }
  };

  const handleSelectSkill = (skillName: string) => {
    setState((prev) => ({
      ...prev,
      selectedSkill: prev.selectedSkill === skillName ? null : skillName,
    }));
  };

  return (
    <div className="skills-panel">
      <div className="skills-panel-header">
        <h2>Skills</h2>
        <div className="skills-panel-actions">
          <button
            className="skills-refresh-btn"
            onClick={handleRefresh}
            disabled={state.refreshing || loading}
            title="Refresh skills from repository"
          >
            {state.refreshing ? "⟳ Refreshing..." : "⟳ Refresh"}
          </button>
        </div>
      </div>

      {/* Workspace Skills Section */}
      <div className="workspace-skills-section">
        <h3>Recommended for Workspace</h3>
        {state.analyzingWorkspace ? (
          <div className="skills-loading">
            <div className="spinner"></div>
            <p>Analyzing workspace...</p>
          </div>
        ) : workspaceError ? (
          <div className="skills-error">
            <span className="error-icon">⚠</span>
            <span className="error-message">{workspaceError}</span>
          </div>
        ) : workspaceSkills.length === 0 ? (
          <div className="skills-empty">
            <p>No recommended skills for this workspace</p>
          </div>
        ) : (
          <div className="workspace-skills-list">
            {workspaceSkills.map((skill) => (
              <div key={skill.name} className="workspace-skill-item">
                <div className="skill-header">
                  <span className="skill-badge">⭐</span>
                  <span className="skill-name">{skill.name}</span>
                  <span className="skill-version">v{skill.version}</span>
                </div>
                <div className="skill-description">{skill.description}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* All Skills Section */}
      <div className="all-skills-section">
        <h3>All Available Skills ({skills.length})</h3>

        {error && (
          <div className="skills-error">
            <span className="error-icon">⚠</span>
            <span className="error-message">{error}</span>
          </div>
        )}

        {loading ? (
          <div className="skills-loading">
            <div className="spinner"></div>
            <p>Loading skills...</p>
          </div>
        ) : skills.length === 0 ? (
          <div className="skills-empty">
            <p>No skills discovered</p>
            <p style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
              Skills are cached globally in ~/.whizcode/skills/cache/
            </p>
            <button onClick={handleRefresh} className="skills-discover-btn">
              Discover Skills
            </button>
          </div>
        ) : (
          <div className="skills-list">
            {skills.map((skill) => (
              <div
                key={skill.name}
                className={`skill-item ${
                  state.selectedSkill === skill.name ? "selected" : ""
                }`}
                onClick={() => handleSelectSkill(skill.name)}
              >
                <div className="skill-name">⚡ {skill.name}</div>
                {skill.description && (
                  <div className="skill-description">{skill.description}</div>
                )}
                {skill.version && (
                  <div className="skill-version">v{skill.version}</div>
                )}
                {skill.author && (
                  <div className="skill-author">by {skill.author}</div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default SkillsPanel;
