use crate::error::Result;
use crate::commands::task_manager::{TaskManager, TaskStatus};
use serde_json::json;

/// Get task progress statistics
#[tauri::command]
pub async fn get_task_progress(workspace_path: String) -> Result<serde_json::Value> {
    match TaskManager::get_task_progress(&workspace_path) {
        Ok((completed, pending, total)) => {
            let percentage = if total > 0 { (completed * 100) / total } else { 0 };
            Ok(json!({
                "completed": completed,
                "pending": pending,
                "total": total,
                "percentage": percentage
            }))
        }
        Err(e) => Err(format!("Failed to get task progress: {}", e).into()),
    }
}

/// Get all tasks grouped by status
#[tauri::command]
pub async fn get_tasks_by_status(workspace_path: String) -> Result<serde_json::Value> {
    match TaskManager::get_tasks_by_status(&workspace_path) {
        Ok((completed, pending, failed)) => {
            Ok(json!({
                "completed": completed.iter().map(|t| {
                    json!({
                        "id": t.id,
                        "description": t.description,
                        "status": "completed"
                    })
                }).collect::<Vec<_>>(),
                "pending": pending.iter().map(|t| {
                    json!({
                        "id": t.id,
                        "description": t.description,
                        "status": if t.status == TaskStatus::InProgress { "in_progress" } else { "not_started" }
                    })
                }).collect::<Vec<_>>(),
                "failed": failed.iter().map(|t| {
                    json!({
                        "id": t.id,
                        "description": t.description,
                        "status": "failed"
                    })
                }).collect::<Vec<_>>()
            }))
        }
        Err(e) => Err(format!("Failed to get tasks by status: {}", e).into()),
    }
}

/// Update task status
#[tauri::command]
pub async fn update_task_status(
    workspace_path: String,
    task_id: String,
    status: String,
    result: Option<String>,
) -> Result<serde_json::Value> {
    let task_status = match status.as_str() {
        "completed" => TaskStatus::Completed,
        "in_progress" => TaskStatus::InProgress,
        "failed" => TaskStatus::Failed,
        "skipped" => TaskStatus::Skipped,
        _ => TaskStatus::NotStarted,
    };

    match TaskManager::update_task_status(&workspace_path, &task_id, task_status, result) {
        Ok(_) => {
            eprintln!("[TaskCommands] Updated task {} to {}", task_id, status);
            Ok(json!({
                "success": true,
                "task_id": task_id,
                "status": status
            }))
        }
        Err(e) => Err(format!("Failed to update task status: {}", e).into()),
    }
}

/// Load tasks markdown
#[tauri::command]
pub async fn load_tasks_markdown(workspace_path: String) -> Result<String> {
    match TaskManager::load_tasks_file(&workspace_path) {
        Ok(task_file) => Ok(task_file.to_markdown()),
        Err(e) => Err(format!("Failed to load tasks: {}", e).into()),
    }
}

/// Load the raw tasks snapshot for live UI rendering
#[tauri::command]
pub async fn load_tasks_snapshot(workspace_path: String) -> Result<serde_json::Value> {
    match TaskManager::load_tasks_file(&workspace_path) {
        Ok(task_file) => Ok(serde_json::to_value(task_file)?),
        Err(_) => Ok(json!(null)),
    }
}

/// Check if tasks exist
#[tauri::command]
pub async fn tasks_exist(workspace_path: String) -> Result<bool> {
    Ok(TaskManager::tasks_exist(&workspace_path))
}

/// Get pending tasks count
#[tauri::command]
pub async fn get_pending_tasks_count(workspace_path: String) -> Result<usize> {
    match TaskManager::get_pending_tasks(&workspace_path) {
        Ok(tasks) => Ok(tasks.len()),
        Err(e) => Err(format!("Failed to get pending tasks: {}", e).into()),
    }
}

/// Get completed tasks count
#[tauri::command]
pub async fn get_completed_tasks_count(workspace_path: String) -> Result<usize> {
    match TaskManager::get_completed_tasks(&workspace_path) {
        Ok(tasks) => Ok(tasks.len()),
        Err(e) => Err(format!("Failed to get completed tasks: {}", e).into()),
    }
}
