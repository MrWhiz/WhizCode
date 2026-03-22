use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanTask {
    pub id: String,
    pub description: String,
    pub task_type: String, // analysis, edit, command, review
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub estimated_duration: u32, // in seconds
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionPlan {
    pub id: String,
    pub objective: String,
    pub tasks: Vec<PlanTask>,
    pub parallel_groups: Vec<Vec<PlanTask>>,
    pub estimated_duration: u32,
    pub risk_level: String, // low, medium, high
    pub fallback_strategies: Vec<String>,
}

pub struct PlanningSystem {
    plan_history: Vec<ExecutionPlan>,
}

impl PlanningSystem {
    pub fn new() -> Self {
        Self {
            plan_history: Vec::new(),
        }
    }

    pub fn create_plan(&mut self, user_request: &str, _workspace_path: &Option<String>) -> ExecutionPlan {
        let objective = self.extract_objective(user_request);
        let task_type = self.classify_request(user_request);

        let tasks = match task_type.as_str() {
            "bug-fix" => self.plan_bug_fix(),
            "feature-implementation" => self.plan_feature_implementation(),
            "refactoring" => self.plan_refactoring(),
            "analysis" => self.plan_analysis(),
            _ => self.plan_generic_task(),
        };

        let parallel_groups = self.optimize_parallel_execution(&tasks);
        let estimated_duration = tasks.iter().map(|t| t.estimated_duration).sum();
        let risk_level = self.assess_risk(&tasks);
        let fallback_strategies = self.generate_fallback_strategies(&tasks);

        let plan = ExecutionPlan {
            id: format!("plan_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            objective,
            tasks,
            parallel_groups,
            estimated_duration,
            risk_level,
            fallback_strategies,
        };

        self.plan_history.push(plan.clone());
        plan
    }

    fn extract_objective(&self, request: &str) -> String {
        request.lines().next().unwrap_or("").to_string()
    }

    fn classify_request(&self, request: &str) -> String {
        let lower = request.to_lowercase();
        if lower.contains("fix") || lower.contains("bug") || lower.contains("error") {
            "bug-fix".to_string()
        } else if lower.contains("add") || lower.contains("implement") || lower.contains("create") {
            "feature-implementation".to_string()
        } else if lower.contains("refactor") || lower.contains("improve") || lower.contains("optimize") {
            "refactoring".to_string()
        } else if lower.contains("analyze") || lower.contains("check") || lower.contains("review") {
            "analysis".to_string()
        } else {
            "generic".to_string()
        }
    }

    fn plan_bug_fix(&self) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "analyze-bug".to_string(),
                description: "Analyze the bug and understand the issue".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 30,
            },
            PlanTask {
                id: "locate-source".to_string(),
                description: "Locate the source of the bug in the codebase".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["analyze-bug".to_string()],
                estimated_duration: 20,
            },
            PlanTask {
                id: "implement-fix".to_string(),
                description: "Implement the fix".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["locate-source".to_string()],
                estimated_duration: 25,
            },
            PlanTask {
                id: "verify-fix".to_string(),
                description: "Verify the fix works correctly".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-fix".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn plan_feature_implementation(&self) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "design-feature".to_string(),
                description: "Design the feature architecture".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 40,
            },
            PlanTask {
                id: "create-files".to_string(),
                description: "Create necessary files and structure".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["design-feature".to_string()],
                estimated_duration: 20,
            },
            PlanTask {
                id: "implement-feature".to_string(),
                description: "Implement the feature".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["create-files".to_string()],
                estimated_duration: 60,
            },
            PlanTask {
                id: "test-feature".to_string(),
                description: "Test the feature".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-feature".to_string()],
                estimated_duration: 30,
            },
        ]
    }

    fn plan_refactoring(&self) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "analyze-code".to_string(),
                description: "Analyze code for refactoring opportunities".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 30,
            },
            PlanTask {
                id: "refactor-code".to_string(),
                description: "Refactor the code".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["analyze-code".to_string()],
                estimated_duration: 45,
            },
            PlanTask {
                id: "verify-refactor".to_string(),
                description: "Verify refactoring maintains functionality".to_string(),
                task_type: "command".to_string(),
                priority: 3,
                dependencies: vec!["refactor-code".to_string()],
                estimated_duration: 20,
            },
        ]
    }

    fn plan_analysis(&self) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "gather-info".to_string(),
                description: "Gather information about the codebase".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 25,
            },
            PlanTask {
                id: "analyze-patterns".to_string(),
                description: "Analyze patterns and structure".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["gather-info".to_string()],
                estimated_duration: 20,
            },
            PlanTask {
                id: "generate-report".to_string(),
                description: "Generate analysis report".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["analyze-patterns".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn plan_generic_task(&self) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "understand-request".to_string(),
                description: "Understand the request".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 20,
            },
            PlanTask {
                id: "execute-task".to_string(),
                description: "Execute the task".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["understand-request".to_string()],
                estimated_duration: 40,
            },
            PlanTask {
                id: "verify-result".to_string(),
                description: "Verify the result".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["execute-task".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn optimize_parallel_execution(&self, tasks: &[PlanTask]) -> Vec<Vec<PlanTask>> {
        let mut groups: Vec<Vec<PlanTask>> = Vec::new();
        let mut completed = std::collections::HashSet::new();

        while completed.len() < tasks.len() {
            let mut current_group = Vec::new();

            for task in tasks {
                if completed.contains(&task.id) {
                    continue;
                }
                if task.dependencies.iter().all(|dep| completed.contains(dep)) {
                    current_group.push(task.clone());
                }
            }

            if current_group.is_empty() {
                break;
            }

            for task in &current_group {
                completed.insert(task.id.clone());
            }

            groups.push(current_group);
        }

        groups
    }

    fn assess_risk(&self, tasks: &[PlanTask]) -> String {
        let edit_tasks = tasks.iter().filter(|t| t.task_type == "edit").count();
        let total_tasks = tasks.len();
        let ratio = edit_tasks as f32 / total_tasks as f32;

        if ratio > 0.7 {
            "high".to_string()
        } else if ratio > 0.4 {
            "medium".to_string()
        } else {
            "low".to_string()
        }
    }

    fn generate_fallback_strategies(&self, tasks: &[PlanTask]) -> Vec<String> {
        let mut strategies = Vec::new();

        if tasks.iter().any(|t| t.task_type == "edit") {
            strategies.push("Create backups before making changes".to_string());
        }

        if tasks.iter().any(|t| t.task_type == "command") {
            strategies.push("Run tests to verify changes".to_string());
        }

        if tasks.len() > 3 {
            strategies.push("Break down into smaller steps if needed".to_string());
        }

        strategies
    }

    #[allow(dead_code)]
    pub fn get_plan_history(&self) -> Vec<ExecutionPlan> {
        self.plan_history.clone()
    }
}

impl Default for PlanningSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub fn create_execution_plan(
    user_request: String,
    workspace_path: Option<String>,
) -> Result<ExecutionPlan> {
    let mut planner = PlanningSystem::new();
    Ok(planner.create_plan(&user_request, &workspace_path))
}
