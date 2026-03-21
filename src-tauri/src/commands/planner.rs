use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WhizCodeTask {
    pub id: String,
    pub description: String,
    #[serde(rename = "type")]
    pub task_type: String, // 'analysis' | 'edit' | 'command' | 'review' | 'planning'
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub estimated_duration: u32, // in seconds
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WhizCodePlan {
    pub id: String,
    pub objective: String,
    pub tasks: Vec<WhizCodeTask>,
    pub parallel_groups: Vec<Vec<WhizCodeTask>>,
    pub estimated_duration: u32,
    pub risk_level: String, // 'low' | 'medium' | 'high'
    pub fallback_strategies: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PlanningContext {
    pub user_request: String,
    pub workspace_path: String,
    pub active_file: Option<serde_json::Value>,
    pub recent_context: Option<String>,
}

pub struct WhizCodePlanner;

impl WhizCodePlanner {
    pub fn create_plan(context: &PlanningContext) -> Result<WhizCodePlan> {
        let objective = Self::extract_objective(&context.user_request);
        let task_type = Self::classify_request(&context.user_request);

        let tasks = match task_type.as_str() {
            "bug-fix" => Self::plan_bug_fix(),
            "feature-implementation" => Self::plan_feature_implementation(),
            "refactoring" => Self::plan_refactoring(),
            "analysis" => Self::plan_analysis(),
            _ => Self::plan_generic_task(),
        };

        let parallel_groups = Self::optimize_parallel_execution(&tasks);
        let estimated_duration = Self::estimate_duration(&tasks);
        let risk_level = Self::assess_risk(&tasks);
        let fallback_strategies = Self::generate_fallback_strategies(&tasks);

        Ok(WhizCodePlan {
            id: format!("plan_{}", chrono::Utc::now().timestamp_millis()),
            objective,
            tasks,
            parallel_groups,
            estimated_duration,
            risk_level,
            fallback_strategies,
        })
    }

    fn extract_objective(request: &str) -> String {
        request.lines().next().unwrap_or("").chars().take(100).collect()
    }

    fn classify_request(request: &str) -> String {
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

    fn plan_bug_fix() -> Vec<WhizCodeTask> {
        vec![
            WhizCodeTask {
                id: "analyze-bug".to_string(),
                description: "Analyze the bug and understand the issue".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 30,
            },
            WhizCodeTask {
                id: "locate-source".to_string(),
                description: "Locate the source of the bug in the codebase".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["analyze-bug".to_string()],
                estimated_duration: 20,
            },
            WhizCodeTask {
                id: "implement-fix".to_string(),
                description: "Implement the fix".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["locate-source".to_string()],
                estimated_duration: 25,
            },
            WhizCodeTask {
                id: "verify-fix".to_string(),
                description: "Verify the fix works correctly".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-fix".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn plan_feature_implementation() -> Vec<WhizCodeTask> {
        vec![
            WhizCodeTask {
                id: "design-feature".to_string(),
                description: "Design the feature architecture".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 40,
            },
            WhizCodeTask {
                id: "create-files".to_string(),
                description: "Create necessary files and structure".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["design-feature".to_string()],
                estimated_duration: 20,
            },
            WhizCodeTask {
                id: "implement-feature".to_string(),
                description: "Implement the feature".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["create-files".to_string()],
                estimated_duration: 60,
            },
            WhizCodeTask {
                id: "test-feature".to_string(),
                description: "Test the feature".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-feature".to_string()],
                estimated_duration: 30,
            },
        ]
    }

    fn plan_refactoring() -> Vec<WhizCodeTask> {
        vec![
            WhizCodeTask {
                id: "analyze-code".to_string(),
                description: "Analyze code for refactoring opportunities".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 30,
            },
            WhizCodeTask {
                id: "refactor-code".to_string(),
                description: "Refactor the code".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["analyze-code".to_string()],
                estimated_duration: 45,
            },
            WhizCodeTask {
                id: "verify-refactor".to_string(),
                description: "Verify refactoring maintains functionality".to_string(),
                task_type: "command".to_string(),
                priority: 3,
                dependencies: vec!["refactor-code".to_string()],
                estimated_duration: 20,
            },
        ]
    }

    fn plan_analysis() -> Vec<WhizCodeTask> {
        vec![
            WhizCodeTask {
                id: "gather-info".to_string(),
                description: "Gather information about the codebase".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 25,
            },
            WhizCodeTask {
                id: "analyze-patterns".to_string(),
                description: "Analyze patterns and structure".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["gather-info".to_string()],
                estimated_duration: 20,
            },
            WhizCodeTask {
                id: "generate-report".to_string(),
                description: "Generate analysis report".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["analyze-patterns".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn plan_generic_task() -> Vec<WhizCodeTask> {
        vec![
            WhizCodeTask {
                id: "understand-request".to_string(),
                description: "Understand the request".to_string(),
                task_type: "analysis".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 20,
            },
            WhizCodeTask {
                id: "execute-task".to_string(),
                description: "Execute the task".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["understand-request".to_string()],
                estimated_duration: 40,
            },
            WhizCodeTask {
                id: "verify-result".to_string(),
                description: "Verify the result".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["execute-task".to_string()],
                estimated_duration: 15,
            },
        ]
    }

    fn optimize_parallel_execution(tasks: &[WhizCodeTask]) -> Vec<Vec<WhizCodeTask>> {
        let mut groups: Vec<Vec<WhizCodeTask>> = vec![];
        let mut completed = std::collections::HashSet::new();

        while completed.len() < tasks.len() {
            let mut current_group = vec![];

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

    fn estimate_duration(tasks: &[WhizCodeTask]) -> u32 {
        tasks.iter().map(|t| t.estimated_duration).sum()
    }

    fn assess_risk(tasks: &[WhizCodeTask]) -> String {
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

    fn generate_fallback_strategies(tasks: &[WhizCodeTask]) -> Vec<String> {
        let mut strategies = vec![];

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
}

#[tauri::command]
pub async fn create_plan(context: PlanningContext) -> Result<WhizCodePlan> {
    WhizCodePlanner::create_plan(&context)
}
