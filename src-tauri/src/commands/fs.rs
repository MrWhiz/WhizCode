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
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
}

#[tauri::command]
pub async fn read_file(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
    cache: State<'_, Arc<std::sync::Mutex<crate::commands::tool_result_cache::ToolResultCache>>>,
) -> Result<String> {
    // ── 1. CACHE LOOKUP ───────────────────────────────────────────────────
    let cache_key = format!("fs:read:{}", path);
    if let Ok(c) = cache.lock() {
        if let Some(cached) = c.get(&cache_key) {
            if let Some(content) = cached.as_str() {
                return Ok(content.to_string());
            }
        }
    }

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
    
    let content = tokio::fs::read_to_string(&resolved)
        .await
        .map_err(ApiError::from)?;

    // ── 2. CACHE STORE ────────────────────────────────────────────────────
    if let Ok(c) = cache.lock() {
        let _ = c.set(cache_key, serde_json::Value::String(content.clone()), Some(30)); // 30s TTL
    }

    Ok(content)
}

#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    handle: AppHandle,
    state: State<'_, Arc<RwLock<AppState>>>,
    cache: State<'_, Arc<std::sync::Mutex<crate::commands::tool_result_cache::ToolResultCache>>>,
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

    // ── 1. CACHE INVALIDATION ─────────────────────────────────────────────
    if let Ok(c) = cache.lock() {
        let _ = c.invalidate(&format!("fs:read:{}", path));
        let _ = c.invalidate(&format!("fs:dir:{}", PathBuf::from(&path).parent().unwrap_or(std::path::Path::new("")).to_string_lossy()));
    }

    // Add small delay to prevent queue overflow
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path: path.clone(),
        kind: "modify".to_string(),
        old_path: None,
    }).map_err(|e| e.to_string())?;
    
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

    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path,
        kind: "create".to_string(),
        old_path: None,
    }).map_err(|e| e.to_string())?;
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

    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path,
        kind: "create".to_string(),
        old_path: None,
    }).map_err(|e| e.to_string())?;
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
    
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path,
        kind: "delete".to_string(),
        old_path: None,
    }).map_err(|e| e.to_string())?;
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
    
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path,
        kind: "delete".to_string(),
        old_path: None,
    }).map_err(|e| e.to_string())?;
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
    
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    handle.emit("file:changed", FileChangedPayload {
        path: new_path,
        kind: "rename".to_string(),
        old_path: Some(old_path),
    }).map_err(|e| e.to_string())?;
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

    // Spawn a background thread to forward events to the frontend with debouncing
    std::thread::spawn(move || {
        let _watcher = watcher;
        let mut pending_paths = std::collections::HashSet::new();
        let mut last_emit = std::time::Instant::now();
        let debounce_window = Duration::from_millis(1000);  // Honors "once per second" request

        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {  // Increased from 50ms to 100ms
                Ok(Ok(event)) => {
                    use notify::EventKind;
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)) {
                        let kind = match event.kind {
                            notify::EventKind::Create(_) => "create",
                            notify::EventKind::Remove(_) => "delete",
                            notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => "rename",
                            _ => "modify",
                        }.to_string();

                        for p in event.paths {
                            if !utils::should_skip_file(&p) {
                                pending_paths.insert((p.to_string_lossy().to_string(), kind.clone()));
                            }
                        }
                    }
                }
                _ => {}
            }

            // Emit batched events if window passed or set is large
            if !pending_paths.is_empty() && (last_emit.elapsed() >= debounce_window || pending_paths.len() > 20) {  // Reduced from 50 to 20
                for (path, kind) in pending_paths.drain() {
                    let _ = handle.emit("file:changed", FileChangedPayload { path, kind, old_path: None });
                }
                last_emit = std::time::Instant::now();
            }
        }
    });

    Ok(())
}
