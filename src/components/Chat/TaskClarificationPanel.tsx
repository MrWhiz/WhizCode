import React, { useState } from "react";
import "./TaskClarificationPanel.css";

interface ClarificationQuestion {
  id: string;
  question: string;
  context: string;
  suggested_answers: string[];
  priority: number;
}

interface PotentialBlocker {
  blocker: string;
  severity: string;
  mitigation: string;
}

interface AcceptanceCriterion {
  criterion: string;
  priority: string;
  measurable: boolean;
}

interface TaskClarificationPanelProps {
  clarification: {
    task_id: string;
    questions: ClarificationQuestion[];
    identified_blockers: PotentialBlocker[];
    acceptance_criteria: AcceptanceCriterion[];
    assumptions: string[];
    estimated_complexity: string;
    recommended_approach: string;
    estimated_duration_minutes: number;
  };
  onApprove: (answers: Record<string, string>) => void;
  onModify: () => void;
  onCancel: () => void;
}

export const TaskClarificationPanel: React.FC<TaskClarificationPanelProps> = ({
  clarification,
  onApprove,
  onModify,
  onCancel,
}) => {
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [expandedSections, setExpandedSections] = useState<
    Record<string, boolean>
  >({
    questions: true,
    blockers: true,
    criteria: true,
    assumptions: false,
    approach: false,
  });

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const handleAnswerChange = (questionId: string, answer: string) => {
    setAnswers((prev) => ({
      ...prev,
      [questionId]: answer,
    }));
  };

  const allQuestionsAnswered = clarification.questions.every(
    (q) => answers[q.id] && answers[q.id].trim() !== "",
  );

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "high":
        return "#ef4444";
      case "medium":
        return "#f59e0b";
      case "low":
        return "#10b981";
      default:
        return "#6b7280";
    }
  };

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case "must":
        return "#ef4444";
      case "should":
        return "#f59e0b";
      case "nice-to-have":
        return "#10b981";
      default:
        return "#6b7280";
    }
  };

  return (
    <div className="task-clarification-panel">
      <div className="clarification-header">
        <h2>🔍 Task Clarification Required</h2>
        <p className="subtitle">
          Before I start working, I need to clarify some requirements to ensure
          success.
        </p>
      </div>

      {/* Questions Section */}
      <div className="clarification-section">
        <div
          className="section-header"
          onClick={() => toggleSection("questions")}
        >
          <span className="section-title">
            ❓ Clarifying Questions ({clarification.questions.length})
          </span>
          <span className="toggle-icon">
            {expandedSections.questions ? "▼" : "▶"}
          </span>
        </div>

        {expandedSections.questions && (
          <div className="section-content">
            {clarification.questions.map((q) => (
              <div key={q.id} className="question-item">
                <div className="question-header">
                  <label className="question-text">{q.question}</label>
                  <span
                    className="priority-badge"
                    style={{ opacity: q.priority / 10 }}
                  >
                    Priority: {q.priority}/10
                  </span>
                </div>
                <p className="question-context">{q.context}</p>
                <select
                  className="question-select"
                  value={answers[q.id] || ""}
                  onChange={(e) => handleAnswerChange(q.id, e.target.value)}
                >
                  <option value="">Select an answer...</option>
                  {q.suggested_answers.map((ans) => (
                    <option key={ans} value={ans}>
                      {ans}
                    </option>
                  ))}
                </select>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Blockers Section */}
      {clarification.identified_blockers.length > 0 && (
        <div className="clarification-section">
          <div
            className="section-header"
            onClick={() => toggleSection("blockers")}
          >
            <span className="section-title">
              ⚠️ Identified Blockers ({clarification.identified_blockers.length}
              )
            </span>
            <span className="toggle-icon">
              {expandedSections.blockers ? "▼" : "▶"}
            </span>
          </div>

          {expandedSections.blockers && (
            <div className="section-content">
              {clarification.identified_blockers.map((blocker, idx) => (
                <div key={idx} className="blocker-item">
                  <div className="blocker-header">
                    <span
                      className="severity-badge"
                      style={{
                        backgroundColor: getSeverityColor(blocker.severity),
                      }}
                    >
                      {blocker.severity.toUpperCase()}
                    </span>
                    <span className="blocker-text">{blocker.blocker}</span>
                  </div>
                  <p className="blocker-mitigation">
                    <strong>Mitigation:</strong> {blocker.mitigation}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Acceptance Criteria Section */}
      <div className="clarification-section">
        <div
          className="section-header"
          onClick={() => toggleSection("criteria")}
        >
          <span className="section-title">
            ✅ Acceptance Criteria ({clarification.acceptance_criteria.length})
          </span>
          <span className="toggle-icon">
            {expandedSections.criteria ? "▼" : "▶"}
          </span>
        </div>

        {expandedSections.criteria && (
          <div className="section-content">
            {clarification.acceptance_criteria.map((criterion, idx) => (
              <div key={idx} className="criterion-item">
                <div className="criterion-header">
                  <span
                    className="priority-badge"
                    style={{
                      backgroundColor: getPriorityColor(criterion.priority),
                    }}
                  >
                    {criterion.priority.toUpperCase()}
                  </span>
                  <span className="criterion-text">{criterion.criterion}</span>
                  {criterion.measurable && (
                    <span className="measurable-badge">📊 Measurable</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Assumptions Section */}
      {clarification.assumptions.length > 0 && (
        <div className="clarification-section">
          <div
            className="section-header"
            onClick={() => toggleSection("assumptions")}
          >
            <span className="section-title">
              💭 Assumptions ({clarification.assumptions.length})
            </span>
            <span className="toggle-icon">
              {expandedSections.assumptions ? "▼" : "▶"}
            </span>
          </div>

          {expandedSections.assumptions && (
            <div className="section-content">
              <ul className="assumptions-list">
                {clarification.assumptions.map((assumption, idx) => (
                  <li key={idx}>{assumption}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Complexity & Duration Section */}
      <div className="clarification-section">
        <div className="complexity-info">
          <div className="info-item">
            <span className="info-label">Estimated Complexity:</span>
            <span className="info-value">
              {clarification.estimated_complexity}
            </span>
          </div>
          <div className="info-item">
            <span className="info-label">Estimated Duration:</span>
            <span className="info-value">
              {clarification.estimated_duration_minutes} minutes
            </span>
          </div>
        </div>
      </div>

      {/* Recommended Approach Section */}
      <div className="clarification-section">
        <div
          className="section-header"
          onClick={() => toggleSection("approach")}
        >
          <span className="section-title">📋 Recommended Approach</span>
          <span className="toggle-icon">
            {expandedSections.approach ? "▼" : "▶"}
          </span>
        </div>

        {expandedSections.approach && (
          <div className="section-content">
            <div className="approach-text">
              {clarification.recommended_approach
                .split("\n")
                .map((line, idx) => (
                  <div key={idx} className="approach-line">
                    {line}
                  </div>
                ))}
            </div>
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div className="clarification-actions">
        <button
          className="btn btn-primary"
          onClick={() => onApprove(answers)}
          disabled={!allQuestionsAnswered}
          title={
            !allQuestionsAnswered
              ? "Please answer all questions before proceeding"
              : "Approve and proceed with the task"
          }
        >
          ✅ Approve & Proceed
        </button>
        <button className="btn btn-secondary" onClick={onModify}>
          ✏️ Modify Requirements
        </button>
        <button className="btn btn-tertiary" onClick={onCancel}>
          ❌ Cancel
        </button>
      </div>
    </div>
  );
};
