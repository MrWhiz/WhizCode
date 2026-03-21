use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
}

#[tauri::command]
pub async fn set_workspace(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace_path = PathBuf::from(&path);
    
    if !workspace_path.exists() {
        return Err("Workspace path does not exist".into());
    }
    
    if !workspace_path.is_dir() {
        return Err("Workspace path is not a directory".into());
    }
    
    let mut app_state = state.write();
    app_state.set_workspace(workspace_path);
    
    Ok(())
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Option<WorkspaceInfo>> {
    let app_state = state.read();
    
    Ok(app_state.get_workspace().map(|p| WorkspaceInfo {
        path: p.to_string_lossy().to_string(),
    }))
}
