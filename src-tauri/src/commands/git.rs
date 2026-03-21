use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub changes: Vec<GitChange>,
}

#[derive(Serialize, Deserialize)]
pub struct GitChange {
    pub file: String,
    pub status: String,
}

#[tauri::command]
pub async fn git_status(path: String) -> Result<GitStatus> {
    let workspace_path = PathBuf::from(&path);
    
    if !workspace_path.exists() || !workspace_path.is_dir() {
        return Ok(GitStatus {
            branch: "unknown".to_string(),
            changes: vec![],
        });
    }
    
    // For now, return empty git status to avoid cancelation errors
    // Git integration can be added later with proper error handling
    Ok(GitStatus {
        branch: "main".to_string(),
        changes: vec![],
    })
}
