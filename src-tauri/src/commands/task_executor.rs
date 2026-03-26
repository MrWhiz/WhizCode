use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::commands::planning::PlanTask;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub task_description: String,
    pub agent_used: String,
    pub status: String, // "completed", "failed", "skipped"
    pub output: String,
    pub duration_ms: u128,
    pub failed: bool,
    pub error_message: Option<String>,
}

pub struct TaskExecutor {
    #[allow(dead_code)]
    workspace_path: String,
}

#[allow(dead_code)]
impl TaskExecutor {
    pub fn new(workspace_path: String) -> Self {
        Self { workspace_path }
    }

    /// Select appropriate agent for task type
    pub fn select_agent_for_task(&self, task: &PlanTask) -> String {
        if !task.owner_agent.trim().is_empty() {
            return task.owner_agent.clone();
        }

        match task.task_type.as_str() {
            "analysis" => "context-gatherer".to_string(),
            "spec" => "product-manager".to_string(),
            "design" => "architect".to_string(),
            "implementation" | "edit" => "general-task-execution".to_string(),
            "testing" => "test-engineer".to_string(),
            "command" => "test-engineer".to_string(),
            "review" => "code-reviewer".to_string(),
            "security" => "security-expert".to_string(),
            "ux" => "ux-designer".to_string(),
            "optimization" => "architect".to_string(),
            _ => "general-task-execution".to_string(),
        }
    }

    /// Create task-specific prompt
    pub fn create_task_prompt(&self, task: &PlanTask, agent: &str) -> String {
        let agent_context = match agent {
            "architect" => "You are an architect. Design the system architecture and structure.",
            "context-gatherer" => "You are a context gatherer. Analyze and understand the codebase.",
            "test-engineer" => "You are a test engineer. Write comprehensive tests.",
            "code-reviewer" => "You are a code reviewer. Review and improve code quality.",
            "security-expert" => "You are a security expert. Identify and fix security issues.",
            "ux-designer" => "You are a UX designer. Improve styling and user experience.",
            _ => "You are a general-purpose agent. Execute the task.",
        };

        format!(
            "{}\n\n\
             TASK: {}\n\
             Type: {}\n\
             Owner Agent: {}\n\
             Priority: {}\n\
             Estimated Duration: {}s\n\
             Dependencies: {}\n\n\
             Deliverable: {}\n\
             Acceptance Criteria: {}\n\n\
             Complete this task and report results.",
            agent_context,
            task.description,
            task.task_type,
            task.owner_agent,
            task.priority,
            task.estimated_duration,
            if task.dependencies.is_empty() {
                "None".to_string()
            } else {
                task.dependencies.join(", ")
            },
            task.deliverable,
            if task.acceptance_criteria.is_empty() {
                "None".to_string()
            } else {
                task.acceptance_criteria.join(" | ")
            }
        )
    }

    /// Check if task dependencies are met
    pub fn dependencies_met(
        task: &PlanTask,
        completed: &HashSet<String>,
    ) -> bool {
        task.dependencies.iter().all(|dep| completed.contains(dep))
    }

    /// Execute tasks respecting dependencies
    pub async fn execute_plan_with_dependencies(
        &self,
        tasks: Vec<PlanTask>,
    ) -> Vec<TaskResult> {
        let mut results = Vec::new();
        let mut completed = HashSet::new();

        while completed.len() < tasks.len() {
            let mut executed_this_round = false;

            for task in &tasks {
                if completed.contains(&task.id) {
                    continue;
                }

                if Self::dependencies_met(task, &completed) {
                    let agent = self.select_agent_for_task(task);

                    eprintln!(
                        "[Task] Executing: {} (Agent: {})",
                        task.description, agent
                    );

                    // Tasks are just planning steps - actual execution happens in main agent loop
                    // This just marks them as planned, not executed
                    let result = TaskResult {
                        task_id: task.id.clone(),
                        task_description: task.description.clone(),
                        agent_used: agent,
                        status: "planned".to_string(),
                        output: format!("Task planned for execution"),
                        duration_ms: 0,
                        failed: false,
                        error_message: None,
                    };

                    results.push(result);
                    completed.insert(task.id.clone());
                    executed_this_round = true;
                }
            }

            if !executed_this_round {
                eprintln!("[Task] Circular dependency detected or all tasks blocked");
                break;
            }
        }

        results
    }

    /// Get task execution order
    #[allow(dead_code)]
    pub fn get_execution_order(&self, tasks: &[PlanTask]) -> Vec<Vec<String>> {
        let mut groups = Vec::new();
        let mut completed = HashSet::new();

        while completed.len() < tasks.len() {
            let mut current_group = Vec::new();

            for task in tasks {
                if completed.contains(&task.id) {
                    continue;
                }

                if task.dependencies.iter().all(|dep| completed.contains(dep)) {
                    current_group.push(task.id.clone());
                }
            }

            if current_group.is_empty() {
                break;
            }

            for id in &current_group {
                completed.insert(id.clone());
            }

            groups.push(current_group);
        }

        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_selection() {
        let executor = TaskExecutor::new(".".to_string());
        let task = PlanTask {
            id: "1".to_string(),
            description: "Design system".to_string(),
            task_type: "design".to_string(),
            priority: 1,
            dependencies: vec![],
            estimated_duration: 30,
            owner_agent: "architect".to_string(),
            deliverable: "Design".to_string(),
            acceptance_criteria: vec![],
            requires_write: false,
        };

        assert_eq!(executor.select_agent_for_task(&task), "architect");
    }

    #[test]
    fn test_dependencies_met() {
        let task = PlanTask {
            id: "2".to_string(),
            description: "Implement".to_string(),
            task_type: "implementation".to_string(),
            priority: 2,
            dependencies: vec!["1".to_string()],
            estimated_duration: 60,
            owner_agent: "general-task-execution".to_string(),
            deliverable: "Implementation".to_string(),
            acceptance_criteria: vec![],
            requires_write: true,
        };

        let mut completed = HashSet::new();
        assert!(!TaskExecutor::dependencies_met(&task, &completed));

        completed.insert("1".to_string());
        assert!(TaskExecutor::dependencies_met(&task, &completed));
    }
}
