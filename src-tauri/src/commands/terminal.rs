use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalInstance {
    pub id: String,
    pub shell_type: String,
    pub cwd: String,
    pub created_at: i64,
    pub last_activity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub shell_type: String,
    pub cwd: Option<String>,
}

pub struct TerminalManager {
    terminals: Arc<Mutex<HashMap<String, TerminalInstance>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        TerminalManager {
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_terminal(&self, config: TerminalConfig) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let cwd = config.cwd.unwrap_or_else(|| std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string()));

        let now = chrono::Utc::now().timestamp();
        let terminal = TerminalInstance {
            id: id.clone(),
            shell_type: config.shell_type,
            cwd,
            created_at: now,
            last_activity: now,
        };

        let mut terminals = self.terminals.lock().unwrap();
        terminals.insert(id.clone(), terminal);

        Ok(id)
    }

    pub fn list_terminals(&self) -> Result<Vec<TerminalInstance>, String> {
        let terminals = self.terminals.lock().unwrap();
        Ok(terminals.values().cloned().collect())
    }

    pub fn get_terminal(&self, id: &str) -> Result<TerminalInstance, String> {
        let terminals = self.terminals.lock().unwrap();
        terminals
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Terminal {} not found", id))
    }

    pub fn close_terminal(&self, id: &str) -> Result<(), String> {
        let mut terminals = self.terminals.lock().unwrap();
        terminals.remove(id);
        Ok(())
    }

    pub fn get_available_shells() -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            vec!["powershell".to_string(), "cmd".to_string()]
        }
        #[cfg(target_os = "macos")]
        {
            vec!["bash".to_string(), "zsh".to_string(), "sh".to_string()]
        }
        #[cfg(target_os = "linux")]
        {
            vec!["bash".to_string(), "sh".to_string()]
        }
    }

    pub fn get_default_shell() -> String {
        #[cfg(target_os = "windows")]
        {
            "powershell".to_string()
        }
        #[cfg(target_os = "macos")]
        {
            "zsh".to_string()
        }
        #[cfg(target_os = "linux")]
        {
            "bash".to_string()
        }
    }
}

// Tauri Commands

#[tauri::command]
pub fn terminal_create(
    config: TerminalConfig,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<String, String> {
    let manager = state.lock().unwrap();
    manager.create_terminal(config)
}

#[tauri::command]
pub fn terminal_list(
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<Vec<TerminalInstance>, String> {
    let manager = state.lock().unwrap();
    manager.list_terminals()
}

#[tauri::command]
pub fn terminal_get(
    id: String,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<TerminalInstance, String> {
    let manager = state.lock().unwrap();
    manager.get_terminal(&id)
}

#[tauri::command]
pub fn terminal_close(
    id: String,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<(), String> {
    let manager = state.lock().unwrap();
    manager.close_terminal(&id)
}

#[tauri::command]
pub fn terminal_get_available_shells() -> Vec<String> {
    TerminalManager::get_available_shells()
}

#[tauri::command]
pub fn terminal_get_default_shell() -> String {
    TerminalManager::get_default_shell()
}

#[tauri::command]
pub fn terminal_write(_id: String, _data: String) -> Result<(), String> {
    // Placeholder for actual terminal write implementation
    // In a real implementation, this would write to the PTY
    Ok(())
}

#[tauri::command]
pub fn terminal_resize(_id: String, _cols: u32, _rows: u32) -> Result<(), String> {
    // Placeholder for actual terminal resize implementation
    Ok(())
}
