use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::error::Result;
use crate::utils;

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub isDirectory: bool,
    pub size: Option<u64>,
}

#[tauri::command]
pub async fn read_file(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let file_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&file_path, &workspace_path)?;
    
    if utils::is_binary_file(&resolved).await? {
        return Err("Cannot read binary file".into());
    }
    
    tokio::fs::read_to_string(&resolved)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let file_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&file_path, &workspace_path)?;
    
    tokio::fs::write(&resolved, content)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn read_directory(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<FileEntry>> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set in backend")?
            .display()
            .to_string()
    };
    
    eprintln!("[read_directory] workspace={}, path={}", workspace, path);
    
    let dir_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&dir_path, &workspace_path)?;
    
    eprintln!("[read_directory] resolved={}", resolved.display());
    
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&resolved).await
        .map_err(|e| format!("Failed to read directory {}: {}", resolved.display(), e))?;
    
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();
        
        entries.push(FileEntry {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            isDirectory: metadata.is_dir(),
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
        });
    }
    
    eprintln!("[read_directory] found {} entries", entries.len());
    Ok(entries)
}

#[tauri::command]
pub async fn read_directory_recursive(
    path: String,
    max_files: Option<usize>,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<FileEntry>> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let dir_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&dir_path, &workspace_path)?;
    
    let max = max_files.unwrap_or(2000);
    let mut entries = Vec::new();
    
    for entry in walkdir::WalkDir::new(&resolved)
        .into_iter()
        .filter_map(|e| e.ok())
        .take(max)
    {
        if utils::should_skip_file(entry.path()) {
            continue;
        }
        
        let metadata = entry.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?;
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            isDirectory: metadata.is_dir(),
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
        });
    }
    
    Ok(entries)
}

#[tauri::command]
pub async fn create_file(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let file_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&file_path, &workspace_path)?;
    
    tokio::fs::write(&resolved, "")
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn create_directory(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let dir_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&dir_path, &workspace_path)?;
    
    tokio::fs::create_dir_all(&resolved)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn delete_file(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let file_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&file_path, &workspace_path)?;
    
    tokio::fs::remove_file(&resolved)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn delete_directory(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let dir_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&dir_path, &workspace_path)?;
    
    tokio::fs::remove_dir_all(&resolved)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn rename_file(
    old_path: String,
    new_path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let workspace = {
        let app_state = state.read();
        app_state.get_workspace()
            .ok_or("No workspace set")?
            .display()
            .to_string()
    };
    
    let old = PathBuf::from(&old_path);
    let new = PathBuf::from(&new_path);
    let workspace_path = PathBuf::from(&workspace);
    
    let resolved_old = utils::validate_path_in_workspace(&old, &workspace_path)?;
    let resolved_new = utils::validate_path_in_workspace(&new, &workspace_path)?;
    
    tokio::fs::rename(&resolved_old, &resolved_new)
        .await
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn check_file_exists(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<bool> {
    let app_state = state.read();
    let workspace = app_state.get_workspace()
        .ok_or("No workspace set")?;
    
    let file_path = PathBuf::from(&path);
    let resolved = utils::validate_path_in_workspace(&file_path, &workspace)?;
    
    Ok(resolved.exists())
}
