import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SteeringFilesPanel.css";

interface SteeringFiles {
  product: {
    purpose: string;
    target_users: string[];
    key_features: string[];
    business_objectives: string[];
  };
  tech: {
    frameworks: string[];
    libraries: string[];
    development_tools: string[];
    technical_constraints: string[];
  };
  structure: {
    file_organization: string;
    naming_conventions: string;
    import_patterns: string;
    architectural_decisions: string[];
  };
}

interface ValidationError {
  file: string;
  field: string;
  message: string;
}

export const SteeringFilesPanel: React.FC = () => {
  const [steering, setSteering] = useState<SteeringFiles | null>(null);
  const [errors, setErrors] = useState<ValidationError[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<"product" | "tech" | "structure">(
    "product",
  );
  const [workspacePath, setWorkspacePath] = useState("");

  useEffect(() => {
    loadSteeringFiles();
  }, []);

  const loadSteeringFiles = async () => {
    try {
      setLoading(true);
      const workspace = await invoke<string>("get_workspace");
      setWorkspacePath(workspace);

      const result = await invoke<any>("validate_steering_files", {
        workspacePath: workspace,
      });

      setSteering(result.steering);
      setErrors(result.errors);
    } catch (error) {
      console.error("Failed to load steering files:", error);
    } finally {
      setLoading(false);
    }
  };

  const createDefaultFiles = async () => {
    try {
      setLoading(true);
      await invoke("create_default_steering_files", {
        workspacePath: workspacePath,
      });
      await loadSteeringFiles();
    } catch (error) {
      console.error("Failed to create default steering files:", error);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="steering-panel loading">Loading steering files...</div>
    );
  }

  if (!steering) {
    return (
      <div className="steering-panel empty">
        <div className="empty-state">
          <h3>No Steering Files Found</h3>
          <p>
            Create default steering files to guide your project development.
          </p>
          <button onClick={createDefaultFiles} className="btn-primary">
            Create Default Steering Files
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="steering-panel">
      <div className="steering-header">
        <h2>Project Steering</h2>
        {errors.length > 0 && (
          <div className="validation-warning">
            ⚠️ {errors.length} validation issue{errors.length !== 1 ? "s" : ""}
          </div>
        )}
      </div>

      <div className="steering-tabs">
        <button
          className={`tab ${activeTab === "product" ? "active" : ""}`}
          onClick={() => setActiveTab("product")}
        >
          Product
        </button>
        <button
          className={`tab ${activeTab === "tech" ? "active" : ""}`}
          onClick={() => setActiveTab("tech")}
        >
          Technology
        </button>
        <button
          className={`tab ${activeTab === "structure" ? "active" : ""}`}
          onClick={() => setActiveTab("structure")}
        >
          Structure
        </button>
      </div>

      <div className="steering-content">
        {activeTab === "product" && (
          <div className="tab-content">
            <div className="section">
              <h3>Purpose</h3>
              <p className="value">
                {steering.product.purpose || "Not defined"}
              </p>
            </div>

            <div className="section">
              <h3>Target Users</h3>
              <ul className="list">
                {steering.product.target_users.length > 0 ? (
                  steering.product.target_users.map((user, idx) => (
                    <li key={idx}>{user}</li>
                  ))
                ) : (
                  <li className="empty">No target users defined</li>
                )}
              </ul>
            </div>

            <div className="section">
              <h3>Key Features</h3>
              <ul className="list">
                {steering.product.key_features.length > 0 ? (
                  steering.product.key_features.map((feature, idx) => (
                    <li key={idx}>{feature}</li>
                  ))
                ) : (
                  <li className="empty">No key features defined</li>
                )}
              </ul>
            </div>

            <div className="section">
              <h3>Business Objectives</h3>
              <ul className="list">
                {steering.product.business_objectives.length > 0 ? (
                  steering.product.business_objectives.map((obj, idx) => (
                    <li key={idx}>{obj}</li>
                  ))
                ) : (
                  <li className="empty">No business objectives defined</li>
                )}
              </ul>
            </div>
          </div>
        )}

        {activeTab === "tech" && (
          <div className="tab-content">
            <div className="section">
              <h3>Frameworks</h3>
              <ul className="list">
                {steering.tech.frameworks.length > 0 ? (
                  steering.tech.frameworks.map((fw, idx) => (
                    <li key={idx}>{fw}</li>
                  ))
                ) : (
                  <li className="empty">No frameworks defined</li>
                )}
              </ul>
            </div>

            <div className="section">
              <h3>Libraries</h3>
              <ul className="list">
                {steering.tech.libraries.length > 0 ? (
                  steering.tech.libraries.map((lib, idx) => (
                    <li key={idx}>{lib}</li>
                  ))
                ) : (
                  <li className="empty">No libraries defined</li>
                )}
              </ul>
            </div>

            <div className="section">
              <h3>Development Tools</h3>
              <ul className="list">
                {steering.tech.development_tools.length > 0 ? (
                  steering.tech.development_tools.map((tool, idx) => (
                    <li key={idx}>{tool}</li>
                  ))
                ) : (
                  <li className="empty">No development tools defined</li>
                )}
              </ul>
            </div>

            <div className="section">
              <h3>Technical Constraints</h3>
              <ul className="list">
                {steering.tech.technical_constraints.length > 0 ? (
                  steering.tech.technical_constraints.map((constraint, idx) => (
                    <li key={idx}>{constraint}</li>
                  ))
                ) : (
                  <li className="empty">No technical constraints defined</li>
                )}
              </ul>
            </div>
          </div>
        )}

        {activeTab === "structure" && (
          <div className="tab-content">
            <div className="section">
              <h3>File Organization</h3>
              <p className="value">
                {steering.structure.file_organization || "Not defined"}
              </p>
            </div>

            <div className="section">
              <h3>Naming Conventions</h3>
              <p className="value">
                {steering.structure.naming_conventions || "Not defined"}
              </p>
            </div>

            <div className="section">
              <h3>Import Patterns</h3>
              <p className="value">
                {steering.structure.import_patterns || "Not defined"}
              </p>
            </div>

            <div className="section">
              <h3>Architectural Decisions</h3>
              <ul className="list">
                {steering.structure.architectural_decisions.length > 0 ? (
                  steering.structure.architectural_decisions.map(
                    (decision, idx) => <li key={idx}>{decision}</li>,
                  )
                ) : (
                  <li className="empty">No architectural decisions defined</li>
                )}
              </ul>
            </div>
          </div>
        )}
      </div>

      {errors.length > 0 && (
        <div className="validation-errors">
          <h3>Validation Issues</h3>
          {errors.map((error, idx) => (
            <div key={idx} className="error-item">
              <strong>{error.file}</strong> - {error.field}: {error.message}
            </div>
          ))}
        </div>
      )}

      <div className="steering-actions">
        <button onClick={loadSteeringFiles} className="btn-secondary">
          Refresh
        </button>
      </div>
    </div>
  );
};
