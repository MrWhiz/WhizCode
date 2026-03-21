use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct ShellInfo {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub async fn create_terminal(
    shell_type: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String> {
    let mut app_state = state.write();
    let workspace = app_state.get_workspace()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    
    let terminal_id = app_state.create_terminal(shell_type, workspace);
    Ok(terminal_id)
}

#[tauri::command]
pub async fn write_to_terminal(
    terminal_id: String,
    data: String,
    _state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    // TODO: Implement actual terminal I/O
    // This will be handled by a separate terminal server
    println!("Write to terminal {}: {}", terminal_id, data);
    Ok(())
}

#[tauri::command]
pub async fn resize_terminal(
    terminal_id: String,
    cols: u16,
    rows: u16,
    _state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    // TODO: Implement terminal resize
    println!("Resize terminal {} to {}x{}", terminal_id, cols, rows);
    Ok(())
}

#[tauri::command]
pub async fn close_terminal(
    terminal_id: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<()> {
    let mut app_state = state.write();
    app_state.remove_terminal(&terminal_id);
    Ok(())
}

#[tauri::command]
pub async fn get_available_shells() -> Result<Vec<ShellInfo>> {
    let shells = if cfg!(target_os = "windows") {
        vec![
            ShellInfo {
                name: "PowerShell".to_string(),
                path: "powershell.exe".to_string(),
            },
            ShellInfo {
                name: "Command Prompt".to_string(),
                path: "cmd.exe".to_string(),
            },
        ]
    } else {
        vec![
            ShellInfo {
                name: "Bash".to_string(),
                path: "/bin/bash".to_string(),
            },
            ShellInfo {
                name: "Zsh".to_string(),
                path: "/bin/zsh".to_string(),
            },
            ShellInfo {
                name: "Sh".to_string(),
                path: "/bin/sh".to_string(),
            },
        ]
    };
    
    Ok(shells)
}

#[tauri::command]
pub async fn get_default_shell() -> Result<String> {
    let shell = if cfg!(target_os = "windows") {
        "powershell.exe".to_string()
    } else {
        "/bin/bash".to_string()
    };
    
    Ok(shell)
}
