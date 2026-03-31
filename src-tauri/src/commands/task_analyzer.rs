use serde::{Deserialize, Serialize};

/// Task type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    FeatureImplementation,
    BugFix,
    Refactoring,
    PerformanceImprovement,
    Documentation,
    Testing,
    Unknown,
}

impl TaskType {
    pub fn to_string(&self) -> String {
        match self {
            TaskType::FeatureImplementation => "Feature Implementation".to_string(),
            TaskType::BugFix => "Bug Fix".to_string(),
            TaskType::Refactoring => "Refactoring".to_string(),
            TaskType::PerformanceImprovement => "Performance Improvement".to_string(),
            TaskType::Documentation => "Documentation".to_string(),
            TaskType::Testing => "Testing".to_string(),
            TaskType::Unknown => "Unknown".to_string(),
        }
    }
}

/// Complexity level of the task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

impl Complexity {
    pub fn to_string(&self) -> String {
        match self {
            Complexity::Simple => "Simple".to_string(),
            Complexity::Moderate => "Moderate".to_string(),
            Complexity::Complex => "Complex".to_string(),
            Complexity::VeryComplex => "Very Complex".to_string(),
        }
    }

    pub fn iteration_estimate(&self) -> u32 {
        match self {
            Complexity::Simple => 5,
            Complexity::Moderate => 10,
            Complexity::Complex => 15,
            Complexity::VeryComplex => 20,
        }
    }
}

/// Acceptance criteria for task completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub description: String,
    pub priority: String, // "must-have", "should-have", "nice-to-have"
    pub verified: bool,
}

/// Potential blocker or risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialBlocker {
    pub id: String,
    pub description: String,
    pub severity: String, // "low", "medium", "high"
    pub mitigation: Option<String>,
}

/// Task analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub task_type: TaskType,
    pub complexity: Complexity,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub required_files: Vec<String>,
    pub potential_blockers: Vec<PotentialBlocker>,
    pub assumptions: Vec<String>,
    pub estimated_iterations: u32,
    pub clarification_questions: Vec<String>,
}

impl TaskAnalysis {
    pub fn new() -> Self {
        TaskAnalysis {
            task_type: TaskType::Unknown,
            complexity: Complexity::Moderate,
            acceptance_criteria: Vec::new(),
            required_files: Vec::new(),
            potential_blockers: Vec::new(),
            assumptions: Vec::new(),
            estimated_iterations: 10,
            clarification_questions: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("📋 Task Analysis Summary\n"));
        summary.push_str(&format!("========================\n\n"));
        summary.push_str(&format!("Task Type: {}\n", self.task_type.to_string()));
        summary.push_str(&format!("Complexity: {}\n", self.complexity.to_string()));
        summary.push_str(&format!("Estimated Iterations: {}\n\n", self.estimated_iterations));

        if !self.acceptance_criteria.is_empty() {
            summary.push_str("✅ Acceptance Criteria:\n");
            for (i, criterion) in self.acceptance_criteria.iter().enumerate() {
                summary.push_str(&format!("  {}. {} [{}]\n", i + 1, criterion.description, criterion.priority));
            }
            summary.push_str("\n");
        }

        if !self.assumptions.is_empty() {
            summary.push_str("📌 Assumptions:\n");
            for (i, assumption) in self.assumptions.iter().enumerate() {
                summary.push_str(&format!("  {}. {}\n", i + 1, assumption));
            }
            summary.push_str("\n");
        }

        if !self.potential_blockers.is_empty() {
            summary.push_str("⚠️ Potential Blockers:\n");
            for blocker in self.potential_blockers.iter() {
                summary.push_str(&format!("  - {} [{}]\n", blocker.description, blocker.severity));
                if let Some(mitigation) = &blocker.mitigation {
                    summary.push_str(&format!("    Mitigation: {}\n", mitigation));
                }
            }
            summary.push_str("\n");
        }

        if !self.clarification_questions.is_empty() {
            summary.push_str("❓ Clarification Questions:\n");
            for (i, question) in self.clarification_questions.iter().enumerate() {
                summary.push_str(&format!("  {}. {}\n", i + 1, question));
            }
        }

        summary
    }
}

/// Task analyzer for understanding requirements
pub struct TaskAnalyzer;

impl TaskAnalyzer {
    /// Analyze a task description and extract key information
    pub fn analyze(task_description: &str) -> TaskAnalysis {
        let mut analysis = TaskAnalysis::new();

        // Detect task type
        analysis.task_type = Self::detect_task_type(task_description);

        // Estimate complexity
        analysis.complexity = Self::estimate_complexity(task_description);

        // Extract acceptance criteria
        analysis.acceptance_criteria = Self::extract_acceptance_criteria(task_description);

        // Identify potential blockers
        analysis.potential_blockers = Self::identify_blockers(task_description);

        // Extract assumptions
        analysis.assumptions = Self::extract_assumptions(task_description);

        // Generate clarification questions
        analysis.clarification_questions = Self::generate_clarification_questions(&analysis);

        // Estimate iterations
        analysis.estimated_iterations = analysis.complexity.iteration_estimate();

        analysis
    }

    fn detect_task_type(description: &str) -> TaskType {
        let lower = description.to_lowercase();

        if lower.contains("create") || lower.contains("add") || lower.contains("implement") || lower.contains("build") {
            TaskType::FeatureImplementation
        } else if lower.contains("fix") || lower.contains("bug") || lower.contains("error") || lower.contains("issue") {
            TaskType::BugFix
        } else if lower.contains("refactor") || lower.contains("improve code") || lower.contains("clean up") {
            TaskType::Refactoring
        } else if lower.contains("optimize") || lower.contains("performance") || lower.contains("speed") {
            TaskType::PerformanceImprovement
        } else if lower.contains("document") || lower.contains("doc") || lower.contains("readme") {
            TaskType::Documentation
        } else if lower.contains("test") || lower.contains("unit test") || lower.contains("integration test") {
            TaskType::Testing
        } else {
            TaskType::Unknown
        }
    }

    fn estimate_complexity(description: &str) -> Complexity {
        let lower = description.to_lowercase();
        let word_count = description.split_whitespace().count();

        // Simple heuristics for complexity
        let complexity_indicators = [
            ("simple", 1),
            ("basic", 1),
            ("small", 1),
            ("quick", 1),
            ("complex", 3),
            ("multiple", 2),
            ("integration", 3),
            ("refactor", 2),
            ("optimize", 2),
            ("database", 3),
            ("api", 2),
            ("authentication", 3),
            ("security", 3),
        ];

        let mut score = 0;
        for (indicator, weight) in &complexity_indicators {
            if lower.contains(indicator) {
                score += weight;
            }
        }

        // Adjust by word count
        if word_count > 100 {
            score += 2;
        } else if word_count > 50 {
            score += 1;
        }

        match score {
            0..=2 => Complexity::Simple,
            3..=5 => Complexity::Moderate,
            6..=8 => Complexity::Complex,
            _ => Complexity::VeryComplex,
        }
    }

    fn extract_acceptance_criteria(description: &str) -> Vec<AcceptanceCriterion> {
        let mut criteria = Vec::new();

        // Look for common acceptance criteria patterns
        let patterns = [
            ("must", "must-have"),
            ("should", "should-have"),
            ("can", "nice-to-have"),
            ("requirement", "must-have"),
            ("feature", "should-have"),
        ];

        let lines: Vec<&str> = description.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let lower = line.to_lowercase();
            for (pattern, priority) in &patterns {
                if lower.contains(pattern) && line.len() > 10 {
                    criteria.push(AcceptanceCriterion {
                        id: format!("ac_{}", i),
                        description: line.trim().to_string(),
                        priority: priority.to_string(),
                        verified: false,
                    });
                    break;
                }
            }
        }

        // If no criteria found, generate default ones
        if criteria.is_empty() {
            criteria.push(AcceptanceCriterion {
                id: "ac_0".to_string(),
                description: "Code compiles without errors".to_string(),
                priority: "must-have".to_string(),
                verified: false,
            });
            criteria.push(AcceptanceCriterion {
                id: "ac_1".to_string(),
                description: "Implementation matches requirements".to_string(),
                priority: "must-have".to_string(),
                verified: false,
            });
        }

        criteria
    }

    fn identify_blockers(description: &str) -> Vec<PotentialBlocker> {
        let mut blockers = Vec::new();
        let lower = description.to_lowercase();

        // Check for common blockers
        if lower.contains("database") || lower.contains("sql") {
            blockers.push(PotentialBlocker {
                id: "blocker_db".to_string(),
                description: "Database schema changes may be required".to_string(),
                severity: "medium".to_string(),
                mitigation: Some("Review existing schema and plan migrations".to_string()),
            });
        }

        if lower.contains("api") || lower.contains("external") {
            blockers.push(PotentialBlocker {
                id: "blocker_api".to_string(),
                description: "External API integration may have rate limits or authentication issues".to_string(),
                severity: "medium".to_string(),
                mitigation: Some("Check API documentation and test authentication".to_string()),
            });
        }

        if lower.contains("authentication") || lower.contains("security") {
            blockers.push(PotentialBlocker {
                id: "blocker_auth".to_string(),
                description: "Security considerations must be carefully implemented".to_string(),
                severity: "high".to_string(),
                mitigation: Some("Follow security best practices and conduct code review".to_string()),
            });
        }

        if lower.contains("performance") || lower.contains("optimize") {
            blockers.push(PotentialBlocker {
                id: "blocker_perf".to_string(),
                description: "Performance testing may be required".to_string(),
                severity: "medium".to_string(),
                mitigation: Some("Set up performance benchmarks before and after changes".to_string()),
            });
        }

        blockers
    }

    fn extract_assumptions(description: &str) -> Vec<String> {
        let mut assumptions = Vec::new();

        // Common assumptions
        if !description.to_lowercase().contains("no database") {
            assumptions.push("Existing project structure is available".to_string());
        }

        if description.to_lowercase().contains("react") || description.to_lowercase().contains("frontend") {
            assumptions.push("React and related dependencies are already installed".to_string());
        }

        if description.to_lowercase().contains("node") || description.to_lowercase().contains("npm") {
            assumptions.push("Node.js and npm are available in the environment".to_string());
        }

        if description.to_lowercase().contains("git") {
            assumptions.push("Git repository is initialized and accessible".to_string());
        }

        assumptions
    }

    fn generate_clarification_questions(analysis: &TaskAnalysis) -> Vec<String> {
        let mut questions = Vec::new();

        // Generate questions based on task type
        match analysis.task_type {
            TaskType::FeatureImplementation => {
                questions.push("What is the expected user interface/behavior?".to_string());
                questions.push("Are there any specific performance requirements?".to_string());
                questions.push("Should this feature be backward compatible?".to_string());
            }
            TaskType::BugFix => {
                questions.push("What is the exact error or unexpected behavior?".to_string());
                questions.push("Can you provide steps to reproduce the issue?".to_string());
                questions.push("What is the expected behavior?".to_string());
            }
            TaskType::Refactoring => {
                questions.push("What are the main goals of this refactoring?".to_string());
                questions.push("Should existing tests be updated?".to_string());
                questions.push("Are there any performance considerations?".to_string());
            }
            TaskType::PerformanceImprovement => {
                questions.push("What are the current performance metrics?".to_string());
                questions.push("What are the target performance metrics?".to_string());
                questions.push("Which parts of the system are the bottleneck?".to_string());
            }
            _ => {
                questions.push("What are the main objectives?".to_string());
                questions.push("Are there any constraints or limitations?".to_string());
            }
        }

        // Add complexity-based questions
        if analysis.complexity >= Complexity::Complex {
            questions.push("Should this be broken down into smaller tasks?".to_string());
        }

        questions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_task_type() {
        assert_eq!(TaskAnalyzer::detect_task_type("Create a new feature"), TaskType::FeatureImplementation);
        assert_eq!(TaskAnalyzer::detect_task_type("Fix the bug"), TaskType::BugFix);
        assert_eq!(TaskAnalyzer::detect_task_type("Refactor the code"), TaskType::Refactoring);
    }

    #[test]
    fn test_estimate_complexity() {
        let simple = TaskAnalyzer::estimate_complexity("Add a button");
        let complex = TaskAnalyzer::estimate_complexity("Implement complex authentication system with database integration");
        assert!(simple <= complex);
    }

    #[test]
    fn test_task_analysis() {
        let analysis = TaskAnalyzer::analyze("Create a React component for user profile");
        assert_eq!(analysis.task_type, TaskType::FeatureImplementation);
        assert!(!analysis.acceptance_criteria.is_empty());
    }
}
