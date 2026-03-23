use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone)]
pub struct TerminalSession {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub shell_type: String,
    #[allow(dead_code)]
    pub cwd: PathBuf,
    #[allow(dead_code)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct AppState {
    pub workspace_path: Option<PathBuf>,
    pub terminals: HashMap<String, TerminalSession>,
    pub app_handle: Option<AppHandle>,
    pub detected_shell: String,
    // --- STDIN REGISTRY FOR INTERACTIVE TOOLS ---
    pub tool_inputs: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::process::ChildStdin>>>,
    pub tool_killers: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

#[allow(dead_code)]
impl AppState {
    pub fn new() -> Self {
        let detected_shell = Self::detect_shell();
        AppState {
            workspace_path: None,
            terminals: HashMap::new(),
            app_handle: None,
            detected_shell,
            tool_inputs: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            tool_killers: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn detect_shell() -> String {
        // Try to detect the shell from environment variables
        if let Ok(shell) = std::env::var("SHELL") {
            // Unix-like systems
            if shell.contains("bash") {
                return "bash".to_string();
            } else if shell.contains("zsh") {
                return "zsh".to_string();
            } else if shell.contains("fish") {
                return "fish".to_string();
            } else if shell.contains("sh") {
                return "sh".to_string();
            }
        }
        
        // Windows detection
        if cfg!(windows) {
            // Check for PowerShell
            if let Ok(ps_path) = std::env::var("PSModulePath") {
                if !ps_path.is_empty() {
                    return "powershell".to_string();
                }
            }
            // Check for pwsh (PowerShell Core)
            if let Ok(_) = std::process::Command::new("pwsh")
                .arg("-NoProfile")
                .arg("-Command")
                .arg("$PSVersionTable.PSVersion")
                .output()
            {
                return "pwsh".to_string();
            }
            // Default to cmd on Windows
            return "cmd".to_string();
        }
        
        // Default to bash for Unix-like systems
        "bash".to_string()
    }

    pub fn get_shell(&self) -> &str {
        &self.detected_shell
    }

    pub fn set_workspace(&mut self, path: PathBuf) {
        self.workspace_path = Some(path);
    }

    pub fn get_workspace(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    pub fn create_terminal(&mut self, shell_type: String, cwd: PathBuf) -> String {
        let id = Uuid::new_v4().to_string();
        self.terminals.insert(
            id.clone(),
            TerminalSession {
                id: id.clone(),
                shell_type,
                cwd,
                created_at: chrono::Utc::now(),
            },
        );
        id
    }

    pub fn remove_terminal(&mut self, id: &str) -> Option<TerminalSession> {
        self.terminals.remove(id)
    }

    #[allow(dead_code)]
    pub fn get_terminal(&self, id: &str) -> Option<&TerminalSession> {
        self.terminals.get(id)
    }
}
