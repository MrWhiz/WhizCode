use tauri::{AppHandle, Emitter, State};
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::error::{Result, ApiError};
use crate::utils;

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub isDirectory: bool,
    pub size: Option<u64>,
}

#[derive(Serialize, Clone)]
struct FileChangedPayload {
    path: String,
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
        .map_err(ApiError::from)
}

#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    handle: AppHandle,
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
        .map_err(ApiError::from)?;

    // Emit event to trigger UI refresh
    handle.emit("file:changed", FileChangedPayload { path: path.clone() }).map_err(|e| e.to_string())?;
    
    Ok(())
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
    
    let dir_path = PathBuf::from(&path);
    let workspace_path = PathBuf::from(&workspace);
    let resolved = utils::validate_path_in_workspace(&dir_path, &workspace_path)?;
    
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&resolved).await
        .map_err(|e| ApiError::from(e))?;
    
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
        
        let metadata = entry.metadata().map_err(|e| ApiError::from(e))?;
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
    handle: AppHandle,
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
        .map_err(ApiError::from)?;

    handle.emit("file:changed", FileChangedPayload { path }).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_directory(
    path: String,
    handle: AppHandle,
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
        .map_err(ApiError::from)?;

    handle.emit("file:changed", FileChangedPayload { path }).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_file(
    path: String,
    handle: AppHandle,
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
    
    #[cfg(target_os = "windows")]
    {
        let mut retries = 0;
        let max_retries = 3;
        loop {
            match tokio::fs::remove_file(&resolved).await {
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) => {
                    if retries >= max_retries {
                        return Err(ApiError::from(e));
                    }
                    retries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => return Err(ApiError::from(e)),
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        tokio::fs::remove_file(&resolved)
            .await
            .map_err(ApiError::from)?;
    }
    
    handle.emit("file:changed", FileChangedPayload { path }).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_directory(
    path: String,
    handle: AppHandle,
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
    
    #[cfg(target_os = "windows")]
    {
        let mut retries = 0;
        let max_retries = 10;
        loop {
            match tokio::fs::remove_dir_all(&resolved).await {
                Ok(_) => break,
                Err(e) if e.to_string().contains("used by another process") || e.to_string().contains("Access is denied") || e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) => {
                    if retries >= max_retries {
                        return Err(ApiError::from(e));
                    }
                    retries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => return Err(ApiError::from(e)),
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        tokio::fs::remove_dir_all(&resolved)
            .await
            .map_err(ApiError::from)?;
    }
    
    handle.emit("file:changed", FileChangedPayload { path }).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn rename_file(
    old_path: String,
    new_path: String,
    handle: AppHandle,
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
    
    #[cfg(target_os = "windows")]
    {
        let mut retries = 0;
        let max_retries = 3;
        loop {
            match tokio::fs::rename(&resolved_old, &resolved_new).await {
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) => {
                    if retries >= max_retries {
                        return Err(ApiError::from(e));
                    }
                    retries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => return Err(ApiError::from(e)),
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        tokio::fs::rename(&resolved_old, &resolved_new)
            .await
            .map_err(ApiError::from)?;
    }
    
    handle.emit("file:changed", FileChangedPayload { path: old_path }).map_err(|e| e.to_string())?;
    handle.emit("file:changed", FileChangedPayload { path: new_path }).map_err(|e| e.to_string())?;
    Ok(())
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

#[tauri::command]
pub async fn watch_directory(
    path: String,
    handle: AppHandle,
) -> Result<()> {
    use notify::{Watcher, RecursiveMode, recommended_watcher, Event};
    use std::sync::mpsc;
    use std::time::Duration;

    let watch_path = PathBuf::from(&path);
    if !watch_path.exists() {
        return Err("Path does not exist".into());
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = recommended_watcher(tx).map_err(|e| e.to_string())?;
    watcher.watch(&watch_path, RecursiveMode::Recursive).map_err(|e| e.to_string())?;

    // Spawn a background thread to forward events to the frontend
    std::thread::spawn(move || {
        // Keep watcher alive in this thread
        let _watcher = watcher;
        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(event)) => {
                    // Debounce: only emit for create/remove/modify events
                    use notify::EventKind;
                    let should_emit = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
                    );
                    if should_emit {
                        for p in &event.paths {
                            let path_str = p.to_string_lossy().to_string();
                            let _ = handle.emit("file:changed", FileChangedPayload { path: path_str });
                        }
                    }
                }
                Ok(Err(_)) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(())
}
