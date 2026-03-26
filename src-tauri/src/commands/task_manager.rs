use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use chrono::Utc;
use crate::commands::problem_identifier::TaskWorkingState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    #[serde(rename = "not_started")]
    NotStarted,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub subtasks: Vec<SubTask>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub description: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub id: String,
    pub description: String,
    pub completed_at: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFile {
    pub project_name: String,
    pub original_query: String,
    pub created_at: String,
    pub status: String, // "in_progress", "completed"
    pub phases: Vec<Phase>,
    pub completed_tasks: Vec<CompletedTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateRecord {
    pub workspace_path: String,
    pub original_query: String,
    pub updated_at: String,
    pub state: TaskWorkingState,
}

impl TaskFile {
    pub fn new(project_name: String, query: String) -> Self {
        Self {
            project_name,
            original_query: query,
            created_at: Utc::now().to_rfc3339(),
            status: "in_progress".to_string(),
            phases: Vec::new(),
            completed_tasks: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }

    pub fn get_pending_tasks(&self) -> Vec<Task> {
        self.phases
            .iter()
            .flat_map(|p| p.tasks.iter().cloned())
            .filter(|t| t.status == TaskStatus::NotStarted || t.status == TaskStatus::InProgress)
            .collect()
    }

    pub fn get_completed_tasks(&self) -> Vec<Task> {
        self.phases
            .iter()
            .flat_map(|p| p.tasks.iter().cloned())
            .filter(|t| t.status == TaskStatus::Completed)
            .collect()
    }

    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus, result: Option<String>) -> bool {
        for phase in &mut self.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.status = status.clone();
                    if status == TaskStatus::Completed {
                        task.completed_at = Some(Utc::now().to_rfc3339());
                        if let Some(r) = result {
                            self.completed_tasks.push(CompletedTask {
                                id: task_id.to_string(),
                                description: task.description.clone(),
                                completed_at: Utc::now().to_rfc3339(),
                                result: r,
                            });
                        }
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Project Tasks - {}\n\n", self.project_name));
        md.push_str(&format!("## Overview\n"));
        md.push_str(&format!("- **Query**: {}\n", self.original_query));
        md.push_str(&format!("- **Created**: {}\n", self.created_at));
        md.push_str(&format!("- **Status**: {}\n\n", self.status));

        for (phase_idx, phase) in self.phases.iter().enumerate() {
            md.push_str(&format!("## Phase {}: {}\n", phase_idx + 1, phase.name));
            md.push_str(&format!("{}\n\n", phase.description));

            for task in &phase.tasks {
                let checkbox = match task.status {
                    TaskStatus::Completed => "[x]",
                    TaskStatus::NotStarted => "[ ]",
                    TaskStatus::InProgress => "[~]",
                    TaskStatus::Failed => "[!]",
                    TaskStatus::Skipped => "[-]",
                };
                md.push_str(&format!("- {} {}\n", checkbox, task.description));

                for subtask in &task.subtasks {
                    let sub_checkbox = match subtask.status {
                        TaskStatus::Completed => "[x]",
                        TaskStatus::NotStarted => "[ ]",
                        TaskStatus::InProgress => "[~]",
                        TaskStatus::Failed => "[!]",
                        TaskStatus::Skipped => "[-]",
                    };
                    md.push_str(&format!("  - {} {}\n", sub_checkbox, subtask.description));
                }
            }
            md.push_str("\n");
        }

        if !self.completed_tasks.is_empty() {
            md.push_str("## Completed Tasks\n\n");
            for completed in &self.completed_tasks {
                md.push_str(&format!("- **{}** ({})\n", completed.description, completed.completed_at));
                if !completed.result.is_empty() {
                    md.push_str(&format!("  Result: {}\n", completed.result));
                }
            }
        }

        md
    }
}

pub struct TaskManager;

impl TaskManager {
    pub fn get_tasks_path(workspace_path: &str) -> String {
        let path = Path::new(workspace_path).join(".whizcode").join("tasks.md");
        path.to_string_lossy().to_string()
    }

    pub fn get_tasks_json_path(workspace_path: &str) -> String {
        let path = Path::new(workspace_path).join(".whizcode").join("tasks.json");
        path.to_string_lossy().to_string()
    }

    pub fn get_task_state_path(workspace_path: &str) -> String {
        let path = Path::new(workspace_path).join(".whizcode").join("task_state.json");
        path.to_string_lossy().to_string()
    }

    pub fn load_tasks_file(workspace_path: &str) -> Result<TaskFile, String> {
        let json_path = Self::get_tasks_json_path(workspace_path);
        
        if !Path::new(&json_path).exists() {
            return Err("Tasks file not found".to_string());
        }

        let content = fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read tasks file: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse tasks file: {}", e))
    }

    pub fn load_task_state(workspace_path: &str) -> Result<Option<TaskStateRecord>, String> {
        let state_path = Self::get_task_state_path(workspace_path);
        if !Path::new(&state_path).exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&state_path)
            .map_err(|e| format!("Failed to read task state: {}", e))?;

        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| format!("Failed to parse task state: {}", e))
    }

    #[allow(dead_code)]
    pub fn create_tasks_file(
        workspace_path: &str,
        project_name: String,
        query: String,
    ) -> Result<TaskFile, String> {
        let whizcode_dir = Path::new(workspace_path).join(".whizcode");
        fs::create_dir_all(&whizcode_dir)
            .map_err(|e| format!("Failed to create .whizcode directory: {}", e))?;

        let tasks = TaskFile::new(project_name, query);
        Self::save_tasks_file(workspace_path, &tasks)?;
        Ok(tasks)
    }

    pub fn save_tasks_file(workspace_path: &str, tasks: &TaskFile) -> Result<(), String> {
        let whizcode_dir = Path::new(workspace_path).join(".whizcode");
        fs::create_dir_all(&whizcode_dir)
            .map_err(|e| format!("Failed to create .whizcode directory: {}", e))?;

        // Save JSON
        let json_path = Self::get_tasks_json_path(workspace_path);
        let json_content = serde_json::to_string_pretty(tasks)
            .map_err(|e| format!("Failed to serialize tasks: {}", e))?;
        fs::write(&json_path, json_content)
            .map_err(|e| format!("Failed to write tasks JSON: {}", e))?;

        // Save Markdown
        let md_path = Self::get_tasks_path(workspace_path);
        let md_content = tasks.to_markdown();
        fs::write(&md_path, md_content)
            .map_err(|e| format!("Failed to write tasks markdown: {}", e))?;

        eprintln!("[TaskManager] Saved tasks to {} and {}", json_path, md_path);
        Ok(())
    }

    pub fn save_task_state(workspace_path: &str, record: &TaskStateRecord) -> Result<(), String> {
        let whizcode_dir = Path::new(workspace_path).join(".whizcode");
        fs::create_dir_all(&whizcode_dir)
            .map_err(|e| format!("Failed to create .whizcode directory: {}", e))?;

        let state_path = Self::get_task_state_path(workspace_path);
        let content = serde_json::to_string_pretty(record)
            .map_err(|e| format!("Failed to serialize task state: {}", e))?;
        fs::write(&state_path, content)
            .map_err(|e| format!("Failed to write task state: {}", e))?;

        eprintln!("[TaskManager] Saved task state to {}", state_path);
        Ok(())
    }

    pub fn update_task_status(
        workspace_path: &str,
        task_id: &str,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), String> {
        let mut tasks = Self::load_tasks_file(workspace_path)?;
        tasks.update_task_status(task_id, status, result);
        Self::save_tasks_file(workspace_path, &tasks)?;
        Ok(())
    }

    pub fn get_pending_tasks(workspace_path: &str) -> Result<Vec<Task>, String> {
        let tasks = Self::load_tasks_file(workspace_path)?;
        Ok(tasks.get_pending_tasks())
    }

    pub fn get_completed_tasks(workspace_path: &str) -> Result<Vec<Task>, String> {
        let tasks = Self::load_tasks_file(workspace_path)?;
        Ok(tasks.get_completed_tasks())
    }

    pub fn tasks_exist(workspace_path: &str) -> bool {
        let json_path = Self::get_tasks_json_path(workspace_path);
        Path::new(&json_path).exists()
    }

    /// Get task progress statistics
    pub fn get_task_progress(workspace_path: &str) -> Result<(usize, usize, usize), String> {
        let tasks = Self::load_tasks_file(workspace_path)?;
        let total = tasks.phases.iter().map(|p| p.tasks.len()).sum::<usize>();
        let completed = tasks.get_completed_tasks().len();
        let pending = tasks.get_pending_tasks().len();
        Ok((completed, pending, total))
    }

    /// Get all tasks grouped by status
    pub fn get_tasks_by_status(workspace_path: &str) -> Result<(Vec<Task>, Vec<Task>, Vec<Task>), String> {
        let tasks = Self::load_tasks_file(workspace_path)?;
        let completed = tasks.get_completed_tasks();
        let pending = tasks.get_pending_tasks();
        let failed = tasks.phases
            .iter()
            .flat_map(|p| p.tasks.iter().cloned())
            .filter(|t| t.status == TaskStatus::Failed)
            .collect();
        Ok((completed, pending, failed))
    }
}
