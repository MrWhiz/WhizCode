use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
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
    pub status: String, // 'pending' | 'in_progress' | 'completed' | 'failed'
    pub actual_duration: Option<u32>,
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
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub status: String, // 'created' | 'in_progress' | 'completed' | 'failed'
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<WhizCodeTask>,
    pub status: String, // 'draft' | 'active' | 'completed' | 'archived'
    pub created_at: u64,
    pub updated_at: u64,
    pub completed_at: Option<u64>,
    pub progress: f32, // 0.0 to 100.0
}

#[derive(Serialize, Deserialize)]
pub struct PlanningContext {
    pub user_request: String,
    pub workspace_path: String,
    pub active_file: Option<serde_json::Value>,
    pub recent_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_plans: usize,
    pub completed_plans: usize,
    pub failed_plans: usize,
    pub average_execution_time: f32,
    pub total_specs: usize,
    pub active_specs: usize,
    pub completed_specs: usize,
}

#[allow(dead_code)]
pub struct WhizCodePlanner {
    plans: Arc<Mutex<HashMap<String, WhizCodePlan>>>,
    specs: Arc<Mutex<HashMap<String, Spec>>>,
}

#[allow(dead_code)]
impl WhizCodePlanner {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(Mutex::new(HashMap::new())),
            specs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn create_plan_static(context: &PlanningContext) -> Result<WhizCodePlan> {
        let objective = Self::extract_objective(&context.user_request);
        let task_type = Self::classify_request(&context.user_request);

        // Always start with a planning task
        let mut tasks = vec![
            WhizCodeTask {
                id: "strategic-planning".to_string(),
                description: "Define execution strategy and decompose tasks".to_string(),
                task_type: "planning".to_string(),
                priority: 0,
                dependencies: vec![],
                estimated_duration: 10,
                status: "completed".to_string(),
                actual_duration: Some(10),
            }
        ];

        let typed_tasks = match task_type.as_str() {
            "bug-fix" => Self::plan_bug_fix(),
            "feature-implementation" => Self::plan_feature_implementation(),
            "refactoring" => Self::plan_refactoring(),
            "analysis" => Self::plan_analysis(),
            _ => Self::plan_generic_task(),
        };
        
        // Add typed tasks and make them depend on strategic-planning
        for mut t in typed_tasks {
            if t.dependencies.is_empty() {
                t.dependencies.push("strategic-planning".to_string());
            }
            tasks.push(t);
        }

        eprintln!("[PLANNER] Generated {} tasks for type {}", tasks.len(), task_type);

        // Initialize task status (except the ones we already set)
        for task in &mut tasks {
            if task.id != "strategic-planning" {
                task.status = "pending".to_string();
            }
        }

        let parallel_groups = Self::optimize_parallel_execution(&tasks);
        let estimated_duration = Self::estimate_duration(&tasks);
        let risk_level = Self::assess_risk(&tasks);
        let fallback_strategies = Self::generate_fallback_strategies(&tasks);

        Ok(WhizCodePlan {
            id: format!("plan_{}", Self::current_timestamp()),
            objective,
            tasks,
            parallel_groups,
            estimated_duration,
            risk_level,
            fallback_strategies,
            created_at: Self::current_timestamp(),
            started_at: Some(Self::current_timestamp()),
            completed_at: None,
            status: "in_progress".to_string(),
        })
    }

    pub fn create_plan(context: &PlanningContext) -> Result<WhizCodePlan> {
        Self::create_plan_static(context)
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
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "locate-source".to_string(),
                description: "Locate the source of the bug in the codebase".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["analyze-bug".to_string()],
                estimated_duration: 20,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "implement-fix".to_string(),
                description: "Implement the fix".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["locate-source".to_string()],
                estimated_duration: 25,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "verify-fix".to_string(),
                description: "Verify the fix works correctly".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-fix".to_string()],
                estimated_duration: 15,
                status: "pending".to_string(),
                actual_duration: None,
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
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "create-files".to_string(),
                description: "Create necessary files and structure".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["design-feature".to_string()],
                estimated_duration: 20,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "implement-feature".to_string(),
                description: "Implement the feature".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["create-files".to_string()],
                estimated_duration: 60,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "test-feature".to_string(),
                description: "Test the feature".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-feature".to_string()],
                estimated_duration: 30,
                status: "pending".to_string(),
                actual_duration: None,
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
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "refactor-code".to_string(),
                description: "Refactor the code".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["analyze-code".to_string()],
                estimated_duration: 45,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "verify-refactor".to_string(),
                description: "Verify refactoring maintains functionality".to_string(),
                task_type: "command".to_string(),
                priority: 3,
                dependencies: vec!["refactor-code".to_string()],
                estimated_duration: 20,
                status: "pending".to_string(),
                actual_duration: None,
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
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "analyze-patterns".to_string(),
                description: "Analyze patterns and structure".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["gather-info".to_string()],
                estimated_duration: 20,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "generate-report".to_string(),
                description: "Generate analysis report".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["analyze-patterns".to_string()],
                estimated_duration: 15,
                status: "pending".to_string(),
                actual_duration: None,
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
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "execute-task".to_string(),
                description: "Execute the task".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["understand-request".to_string()],
                estimated_duration: 40,
                status: "pending".to_string(),
                actual_duration: None,
            },
            WhizCodeTask {
                id: "verify-result".to_string(),
                description: "Verify the result".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["execute-task".to_string()],
                estimated_duration: 15,
                status: "pending".to_string(),
                actual_duration: None,
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

#[tauri::command]
pub async fn planner_save_plan(plan: WhizCodePlan) -> Result<()> {
    eprintln!("Saving plan: {}", plan.id);
    Ok(())
}

#[tauri::command]
pub async fn planner_get_plan(plan_id: String) -> Result<Option<WhizCodePlan>> {
    eprintln!("Getting plan: {}", plan_id);
    Ok(None)
}

#[tauri::command]
pub async fn planner_get_all_plans() -> Result<Vec<WhizCodePlan>> {
    eprintln!("Getting all plans");
    Ok(vec![])
}

#[tauri::command]
pub async fn planner_start_plan(plan_id: String) -> Result<()> {
    eprintln!("Starting plan: {}", plan_id);
    Ok(())
}

#[tauri::command]
pub async fn planner_complete_plan(plan_id: String) -> Result<()> {
    eprintln!("Completing plan: {}", plan_id);
    Ok(())
}

#[tauri::command]
pub async fn planner_update_task_status(_plan_id: String, task_id: String, status: String) -> Result<()> {
    eprintln!("Updating task {} status to {}", task_id, status);
    Ok(())
}

#[tauri::command]
pub async fn planner_delete_plan(plan_id: String) -> Result<()> {
    eprintln!("Deleting plan: {}", plan_id);
    Ok(())
}

#[tauri::command]
pub async fn planner_create_spec(spec: Spec) -> Result<()> {
    eprintln!("Creating spec: {}", spec.id);
    Ok(())
}

#[tauri::command]
pub async fn planner_get_spec(spec_id: String) -> Result<Option<Spec>> {
    eprintln!("Getting spec: {}", spec_id);
    Ok(None)
}

#[tauri::command]
pub async fn planner_get_all_specs() -> Result<Vec<Spec>> {
    eprintln!("Getting all specs");
    Ok(vec![])
}

#[tauri::command]
pub async fn planner_get_active_specs() -> Result<Vec<Spec>> {
    eprintln!("Getting active specs");
    Ok(vec![])
}

#[tauri::command]
pub async fn planner_update_spec_status(spec_id: String, status: String) -> Result<()> {
    eprintln!("Updating spec {} status to {}", spec_id, status);
    Ok(())
}

#[tauri::command]
pub async fn planner_update_spec_progress(spec_id: String, progress: f32) -> Result<()> {
    eprintln!("Updating spec {} progress to {}", spec_id, progress);
    Ok(())
}

#[tauri::command]
pub async fn planner_delete_spec(spec_id: String) -> Result<()> {
    eprintln!("Deleting spec: {}", spec_id);
    Ok(())
}

#[tauri::command]
pub async fn planner_get_metrics() -> Result<ExecutionMetrics> {
    eprintln!("Getting planner metrics");
    Ok(ExecutionMetrics {
        total_plans: 0,
        completed_plans: 0,
        failed_plans: 0,
        average_execution_time: 0.0,
        total_specs: 0,
        active_specs: 0,
        completed_specs: 0,
    })
}
