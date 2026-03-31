use serde::{Deserialize, Serialize};
use crate::error::Result;

/// A clarification question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    pub id: String,
    pub question: String,
    pub context: String,
    pub suggested_answers: Vec<String>,
    pub priority: u8, // 1-10, higher = more important
}

/// A potential blocker identified in the task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialBlocker {
    pub blocker: String,
    pub severity: String, // "low", "medium", "high"
    pub mitigation: String,
}

/// Acceptance criteria extracted from the task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub criterion: String,
    pub priority: String, // "must", "should", "nice-to-have"
    pub measurable: bool,
}

/// Complete task clarification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClarification {
    pub task_id: String,
    pub questions: Vec<ClarificationQuestion>,
    pub identified_blockers: Vec<PotentialBlocker>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub assumptions: Vec<String>,
    pub estimated_complexity: String, // "Simple", "Moderate", "Complex", "VeryComplex"
    pub recommended_approach: String,
    pub estimated_duration_minutes: u32,
}

/// Task type classification
#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    FeatureImplementation,
    BugFix,
    Refactoring,
    Performance,
    Documentation,
    Testing,
    Analysis,
    Unknown,
}

impl TaskType {
    fn from_task(task: &str) -> Self {
        let lower = task.to_lowercase();
        
        if lower.contains("feature") || lower.contains("add") || lower.contains("implement") {
            TaskType::FeatureImplementation
        } else if lower.contains("bug") || lower.contains("fix") || lower.contains("error") {
            TaskType::BugFix
        } else if lower.contains("refactor") || lower.contains("improve") || lower.contains("clean") {
            TaskType::Refactoring
        } else if lower.contains("performance") || lower.contains("optimize") || lower.contains("speed") {
            TaskType::Performance
        } else if lower.contains("doc") || lower.contains("comment") || lower.contains("readme") {
            TaskType::Documentation
        } else if lower.contains("test") || lower.contains("spec") {
            TaskType::Testing
        } else if lower.contains("analyze") || lower.contains("understand") || lower.contains("review") {
            TaskType::Analysis
        } else {
            TaskType::Unknown
        }
    }
}

/// Task Clarification Engine
pub struct TaskClarificationEngine;

impl TaskClarificationEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analyze task and generate clarification
    pub fn analyze_task(
        &self,
        task: &str,
        workspace_context: Option<&str>,
    ) -> TaskClarification {
        let task_id = format!("task_{}", chrono::Utc::now().timestamp());
        let task_type = TaskType::from_task(task);

        let questions = self.generate_questions(task, &task_type);
        let blockers = self.identify_blockers(task, workspace_context);
        let criteria = self.extract_acceptance_criteria(task);
        let assumptions = self.extract_assumptions(task);
        let complexity = self.estimate_complexity(task, &task_type);
        let approach = self.recommend_approach(task, &task_type);
        let duration = self.estimate_duration(&task_type, &complexity);

        TaskClarification {
            task_id,
            questions,
            identified_blockers: blockers,
            acceptance_criteria: criteria,
            assumptions,
            estimated_complexity: complexity,
            recommended_approach: approach,
            estimated_duration_minutes: duration,
        }
    }

    /// Generate clarification questions based on task type
    fn generate_questions(&self, _task: &str, task_type: &TaskType) -> Vec<ClarificationQuestion> {
        let mut questions = Vec::new();

        match task_type {
            TaskType::FeatureImplementation => {
                questions.push(ClarificationQuestion {
                    id: "scope".to_string(),
                    question: "What is the exact scope of this feature? What should it include and exclude?".to_string(),
                    context: "Scope clarification prevents scope creep and ensures alignment".to_string(),
                    suggested_answers: vec![
                        "Just the core functionality".to_string(),
                        "Core + basic UI".to_string(),
                        "Full feature with all bells and whistles".to_string(),
                    ],
                    priority: 9,
                });

                questions.push(ClarificationQuestion {
                    id: "integration".to_string(),
                    question: "How should this integrate with existing code?".to_string(),
                    context: "Integration approach affects architecture and implementation".to_string(),
                    suggested_answers: vec![
                        "Standalone module".to_string(),
                        "Extend existing module".to_string(),
                        "Replace existing functionality".to_string(),
                    ],
                    priority: 8,
                });

                questions.push(ClarificationQuestion {
                    id: "dependencies".to_string(),
                    question: "Are there any external dependencies or APIs needed?".to_string(),
                    context: "Understanding dependencies helps plan the implementation".to_string(),
                    suggested_answers: vec![
                        "No external dependencies".to_string(),
                        "Uses existing internal APIs".to_string(),
                        "Requires new external APIs".to_string(),
                    ],
                    priority: 7,
                });
            }
            TaskType::BugFix => {
                questions.push(ClarificationQuestion {
                    id: "reproduction".to_string(),
                    question: "Can you provide steps to reproduce the bug?".to_string(),
                    context: "Reproduction steps help verify the fix works".to_string(),
                    suggested_answers: vec![
                        "Yes, here are clear steps".to_string(),
                        "Intermittent issue, hard to reproduce".to_string(),
                        "Unknown reproduction steps".to_string(),
                    ],
                    priority: 10,
                });

                questions.push(ClarificationQuestion {
                    id: "impact".to_string(),
                    question: "What is the impact of this bug?".to_string(),
                    context: "Impact determines priority and approach".to_string(),
                    suggested_answers: vec![
                        "Critical - blocks users".to_string(),
                        "High - affects functionality".to_string(),
                        "Medium - minor issue".to_string(),
                    ],
                    priority: 9,
                });

                questions.push(ClarificationQuestion {
                    id: "regression".to_string(),
                    question: "Are there any known workarounds or related issues?".to_string(),
                    context: "Understanding related issues prevents regressions".to_string(),
                    suggested_answers: vec![
                        "No known workarounds".to_string(),
                        "Yes, there's a workaround".to_string(),
                        "Related to other issues".to_string(),
                    ],
                    priority: 7,
                });
            }
            TaskType::Refactoring => {
                questions.push(ClarificationQuestion {
                    id: "goals".to_string(),
                    question: "What are the main goals of this refactoring?".to_string(),
                    context: "Clear goals ensure the refactoring is successful".to_string(),
                    suggested_answers: vec![
                        "Improve readability".to_string(),
                        "Improve performance".to_string(),
                        "Reduce technical debt".to_string(),
                        "Multiple goals".to_string(),
                    ],
                    priority: 9,
                });

                questions.push(ClarificationQuestion {
                    id: "scope_refactor".to_string(),
                    question: "What is the scope of the refactoring?".to_string(),
                    context: "Scope determines the effort and risk".to_string(),
                    suggested_answers: vec![
                        "Single file".to_string(),
                        "Single module".to_string(),
                        "Multiple modules".to_string(),
                    ],
                    priority: 8,
                });

                questions.push(ClarificationQuestion {
                    id: "breaking".to_string(),
                    question: "Should this refactoring maintain backward compatibility?".to_string(),
                    context: "Breaking changes affect users and dependencies".to_string(),
                    suggested_answers: vec![
                        "Must maintain compatibility".to_string(),
                        "Can break compatibility".to_string(),
                        "Doesn't matter".to_string(),
                    ],
                    priority: 8,
                });
            }
            _ => {
                // Generic questions for other task types
                questions.push(ClarificationQuestion {
                    id: "objective".to_string(),
                    question: "What is the main objective of this task?".to_string(),
                    context: "Clear objectives ensure alignment".to_string(),
                    suggested_answers: vec![
                        "Understand the current state".to_string(),
                        "Make a specific change".to_string(),
                        "Analyze and report".to_string(),
                    ],
                    priority: 9,
                });
            }
        }

        // Sort by priority (highest first)
        questions.sort_by(|a, b| b.priority.cmp(&a.priority));
        questions
    }

    /// Identify potential blockers
    fn identify_blockers(
        &self,
        task: &str,
        workspace_context: Option<&str>,
    ) -> Vec<PotentialBlocker> {
        let mut blockers = Vec::new();

        // Check for common blockers
        if task.to_lowercase().contains("database") {
            blockers.push(PotentialBlocker {
                blocker: "Database schema changes may be required".to_string(),
                severity: "high".to_string(),
                mitigation: "Plan database migration strategy upfront".to_string(),
            });
        }

        if task.to_lowercase().contains("api") {
            blockers.push(PotentialBlocker {
                blocker: "External API availability and rate limits".to_string(),
                severity: "medium".to_string(),
                mitigation: "Implement error handling and retry logic".to_string(),
            });
        }

        if task.to_lowercase().contains("performance") {
            blockers.push(PotentialBlocker {
                blocker: "Performance testing infrastructure may be needed".to_string(),
                severity: "medium".to_string(),
                mitigation: "Set up benchmarking tools before optimization".to_string(),
            });
        }

        if task.to_lowercase().contains("security") {
            blockers.push(PotentialBlocker {
                blocker: "Security review and compliance checks required".to_string(),
                severity: "high".to_string(),
                mitigation: "Plan security review process early".to_string(),
            });
        }

        // Check workspace context for blockers
        if let Some(context) = workspace_context {
            if !context.contains("package.json") && task.to_lowercase().contains("npm") {
                blockers.push(PotentialBlocker {
                    blocker: "Project dependencies may not be installed".to_string(),
                    severity: "medium".to_string(),
                    mitigation: "Run npm install before proceeding".to_string(),
                });
            }
        }

        blockers
    }

    /// Extract acceptance criteria from task
    fn extract_acceptance_criteria(&self, task: &str) -> Vec<AcceptanceCriterion> {
        let mut criteria = Vec::new();
        let lower = task.to_lowercase();

        // Look for common acceptance criteria patterns
        if lower.contains("should") {
            criteria.push(AcceptanceCriterion {
                criterion: "Functionality works as expected".to_string(),
                priority: "must".to_string(),
                measurable: true,
            });
        }

        if lower.contains("test") || lower.contains("spec") {
            criteria.push(AcceptanceCriterion {
                criterion: "All tests pass".to_string(),
                priority: "must".to_string(),
                measurable: true,
            });
        }

        if lower.contains("error") || lower.contains("bug") {
            criteria.push(AcceptanceCriterion {
                criterion: "Error is resolved and doesn't reoccur".to_string(),
                priority: "must".to_string(),
                measurable: true,
            });
        }

        if lower.contains("performance") || lower.contains("optimize") {
            criteria.push(AcceptanceCriterion {
                criterion: "Performance improvement is measurable".to_string(),
                priority: "must".to_string(),
                measurable: true,
            });
        }

        if lower.contains("document") || lower.contains("comment") {
            criteria.push(AcceptanceCriterion {
                criterion: "Code is well documented".to_string(),
                priority: "should".to_string(),
                measurable: false,
            });
        }

        // Add default criteria
        if criteria.is_empty() {
            criteria.push(AcceptanceCriterion {
                criterion: "Task is completed successfully".to_string(),
                priority: "must".to_string(),
                measurable: true,
            });
        }

        criteria
    }

    /// Extract assumptions from task
    fn extract_assumptions(&self, task: &str) -> Vec<String> {
        let mut assumptions = Vec::new();

        // Common assumptions
        assumptions.push("The workspace is properly initialized".to_string());
        assumptions.push("All dependencies are installed".to_string());
        assumptions.push("The codebase follows existing patterns".to_string());

        // Task-specific assumptions
        if task.to_lowercase().contains("frontend") {
            assumptions.push("React/UI framework is properly set up".to_string());
        }

        if task.to_lowercase().contains("backend") {
            assumptions.push("Server/API infrastructure is available".to_string());
        }

        if task.to_lowercase().contains("database") {
            assumptions.push("Database is accessible and configured".to_string());
        }

        assumptions
    }

    /// Estimate task complexity
    fn estimate_complexity(&self, task: &str, task_type: &TaskType) -> String {
        let lower = task.to_lowercase();
        let word_count = task.split_whitespace().count();

        // Base complexity by task type
        let base_complexity = match task_type {
            TaskType::BugFix => 1,
            TaskType::Documentation => 1,
            TaskType::Analysis => 2,
            TaskType::Testing => 2,
            TaskType::Performance => 3,
            TaskType::Refactoring => 3,
            TaskType::FeatureImplementation => 4,
            TaskType::Unknown => 2,
        };

        // Adjust based on keywords
        let mut complexity = base_complexity;

        if lower.contains("database") || lower.contains("api") {
            complexity += 1;
        }
        if lower.contains("security") || lower.contains("authentication") {
            complexity += 1;
        }
        if lower.contains("performance") || lower.contains("optimize") {
            complexity += 1;
        }
        if lower.contains("multiple") || lower.contains("several") {
            complexity += 1;
        }

        // Adjust based on task length
        if word_count > 50 {
            complexity += 1;
        }

        match complexity {
            0..=2 => "Simple".to_string(),
            3..=4 => "Moderate".to_string(),
            5..=6 => "Complex".to_string(),
            _ => "VeryComplex".to_string(),
        }
    }

    /// Recommend approach for the task
    fn recommend_approach(&self, _task: &str, task_type: &TaskType) -> String {
        match task_type {
            TaskType::FeatureImplementation => {
                "1. Understand requirements\n2. Design the feature\n3. Implement core functionality\n4. Add tests\n5. Integrate with existing code\n6. Document the feature".to_string()
            }
            TaskType::BugFix => {
                "1. Reproduce the bug\n2. Understand the root cause\n3. Implement the fix\n4. Verify the fix works\n5. Check for regressions\n6. Document the fix".to_string()
            }
            TaskType::Refactoring => {
                "1. Understand current code\n2. Plan the refactoring\n3. Refactor incrementally\n4. Run tests after each change\n5. Verify no regressions\n6. Document changes".to_string()
            }
            TaskType::Performance => {
                "1. Identify bottlenecks\n2. Measure current performance\n3. Implement optimizations\n4. Measure improvements\n5. Verify no regressions\n6. Document optimizations".to_string()
            }
            _ => {
                "1. Understand the task\n2. Plan the approach\n3. Execute the plan\n4. Verify the results\n5. Document the work".to_string()
            }
        }
    }

    /// Estimate task duration
    fn estimate_duration(&self, task_type: &TaskType, complexity: &str) -> u32 {
        let base_minutes = match task_type {
            TaskType::BugFix => 30,
            TaskType::Documentation => 20,
            TaskType::Analysis => 40,
            TaskType::Testing => 45,
            TaskType::Performance => 60,
            TaskType::Refactoring => 60,
            TaskType::FeatureImplementation => 120,
            TaskType::Unknown => 60,
        };

        let multiplier = match complexity {
            "Simple" => 0.5,
            "Moderate" => 1.0,
            "Complex" => 2.0,
            "VeryComplex" => 3.0,
            _ => 1.0,
        };

        (base_minutes as f32 * multiplier) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_implementation_clarification() {
        let engine = TaskClarificationEngine::new();
        let clarification = engine.analyze_task("Create a new user authentication feature", None);

        assert!(!clarification.questions.is_empty());
        assert_eq!(clarification.estimated_complexity, "Complex");
        assert!(clarification.identified_blockers.len() > 0);
    }

    #[test]
    fn test_bug_fix_clarification() {
        let engine = TaskClarificationEngine::new();
        let clarification = engine.analyze_task("Fix the login bug", None);

        assert!(!clarification.questions.is_empty());
        assert_eq!(clarification.estimated_complexity, "Simple");
    }

    #[test]
    fn test_task_type_detection() {
        assert_eq!(TaskType::from_task("Create a new feature"), TaskType::FeatureImplementation);
        assert_eq!(TaskType::from_task("Fix the bug"), TaskType::BugFix);
        assert_eq!(TaskType::from_task("Refactor the code"), TaskType::Refactoring);
    }
}


// ─────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────

/// Clarify a task by asking questions and identifying blockers
#[tauri::command]
pub fn clarify_task(
    task: String,
    workspace_context: Option<String>,
) -> Result<TaskClarification> {
    let engine = TaskClarificationEngine::new();
    let clarification = engine.analyze_task(&task, workspace_context.as_deref());
    Ok(clarification)
}
