use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Clone)]
pub struct TerminalSession {
    #[allow(dead_code)]
    pub shell_type: String,
    #[allow(dead_code)]
    pub cwd: PathBuf,
}

impl TerminalSession {
    pub fn new(shell_type: String, cwd: PathBuf) -> Self {
        Self { shell_type, cwd }
    }

    #[allow(dead_code)]
    pub fn update_cwd(&mut self, new_cwd: PathBuf) {
        self.cwd = new_cwd;
    }

    #[allow(dead_code)]
    pub fn get_shell_command(&self) -> &str {
        match self.shell_type.as_str() {
            "powershell" | "pwsh" => "powershell",
            "cmd" => "cmd",
            "bash" => "bash",
            "zsh" => "zsh",
            "fish" => "fish",
            _ => "bash",
        }
    }
}

pub struct AppState {
    pub workspace_path: Option<PathBuf>,
    pub app_handle: Option<AppHandle>,
    pub detected_shell: String,
    pub terminal_session: Option<TerminalSession>,
    pub tool_inputs: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::process::ChildStdin>>>,
    pub tool_killers: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

impl AppState {
    pub fn new() -> Self {
        let detected_shell = Self::detect_shell();
        AppState {
            workspace_path: None,
            app_handle: None,
            detected_shell,
            terminal_session: None,
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
        self.workspace_path = Some(path.clone());
        // Initialize terminal session with workspace as cwd
        self.terminal_session = Some(TerminalSession::new(
            self.detected_shell.clone(),
            path,
        ));
    }

    pub fn get_workspace(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    #[allow(dead_code)]
    pub fn get_terminal_session(&self) -> Option<&TerminalSession> {
        self.terminal_session.as_ref()
    }

    #[allow(dead_code)]
    pub fn get_terminal_session_mut(&mut self) -> Option<&mut TerminalSession> {
        self.terminal_session.as_mut()
    }
}
