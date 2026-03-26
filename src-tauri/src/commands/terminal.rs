use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, Command};
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

struct TerminalSession {
    instance: TerminalInstance,
    process_id: u32,
    stdin: Arc<tokio::sync::Mutex<ChildStdin>>,
}

pub struct TerminalManager {
    terminals: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn now() -> i64 {
        Utc::now().timestamp()
    }

    fn resolve_cwd(cwd: Option<String>) -> String {
        cwd.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }).unwrap_or_else(|| {
            std::env::current_dir()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "C:\\".to_string()
                    } else {
                        "/".to_string()
                    }
                })
        })
    }

    fn shell_command(shell_type: &str) -> (String, Vec<String>) {
        #[cfg(target_os = "windows")]
        {
            match shell_type {
                "cmd" => ("cmd.exe".to_string(), vec![]),
                "powershell" | "pwsh" => (
                    if shell_type == "pwsh" { "pwsh.exe".to_string() } else { "powershell.exe".to_string() },
                    vec!["-NoLogo".to_string(), "-NoProfile".to_string(), "-NoExit".to_string()],
                ),
                _ => ("powershell.exe".to_string(), vec!["-NoLogo".to_string(), "-NoProfile".to_string(), "-NoExit".to_string()]),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            match shell_type {
                "zsh" => ("zsh".to_string(), vec!["-i".to_string()]),
                "sh" => ("sh".to_string(), vec!["-i".to_string()]),
                _ => ("bash".to_string(), vec!["-i".to_string()]),
            }
        }
    }

    fn remove_session(&self, id: &str) -> Option<TerminalSession> {
        self.terminals.lock().ok().and_then(|mut sessions| sessions.remove(id))
    }

    fn get_session(&self, id: &str) -> Result<TerminalSessionHandle, String> {
        let sessions = self.terminals.lock().map_err(|_| "Terminal session lock poisoned".to_string())?;
        let session = sessions.get(id).ok_or_else(|| format!("Terminal {} not found", id))?;
        Ok(TerminalSessionHandle {
            process_id: session.process_id,
            stdin: session.stdin.clone(),
        })
    }

    fn get_sessions(&self) -> Vec<TerminalInstance> {
        self.terminals
            .lock()
            .map(|sessions| sessions.values().map(|session| session.instance.clone()).collect())
            .unwrap_or_default()
    }

    fn kill_process_by_id(process_id: u32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("taskkill")
                .args(["/PID", &process_id.to_string(), "/T", "/F"])
                .output()
                .map_err(|error| format!("Failed to terminate terminal process: {}", error))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &process_id.to_string()])
                .output()
                .map_err(|error| format!("Failed to terminate terminal process: {}", error))?;
        }

        Ok(())
    }

    pub fn create_terminal(
        &self,
        config: TerminalConfig,
        app: AppHandle,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let cwd = Self::resolve_cwd(config.cwd);
        let (shell, args) = Self::shell_command(&config.shell_type);

        let cwd_path = PathBuf::from(&cwd);
        let mut command = Command::new(&shell);
        command
            .args(&args)
            .current_dir(&cwd_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| format!("Failed to start terminal shell: {}", error))?;

        let process_id = child.id().unwrap_or_default();
        let stdin = child.stdin.take().ok_or_else(|| "Terminal stdin was not available".to_string())?;
        let stdout = child.stdout.take().ok_or_else(|| "Terminal stdout was not available".to_string())?;
        let stderr = child.stderr.take().ok_or_else(|| "Terminal stderr was not available".to_string())?;

        let instance = TerminalInstance {
            id: id.clone(),
            shell_type: config.shell_type,
            cwd,
            created_at: Self::now(),
            last_activity: Self::now(),
        };

        let stdin = Arc::new(tokio::sync::Mutex::new(stdin));
        {
            let mut sessions = self
                .terminals
                .lock()
                .map_err(|_| "Terminal session lock poisoned".to_string())?;
            sessions.insert(
                id.clone(),
                TerminalSession {
                    instance: instance.clone(),
                    process_id,
                    stdin: stdin.clone(),
                },
            );
        }

        let sessions = self.terminals.clone();
        let app_for_stdout = app.clone();
        let app_for_stderr = app.clone();
        let stdout_id = id.clone();
        let stderr_id = id.clone();
        let exit_id = id.clone();

        tauri::async_runtime::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buffer = vec![0u8; 4096];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(bytes_read) => {
                        let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                        if !data.is_empty() {
                            let _ = app_for_stdout.emit(&format!("terminal:data:{stdout_id}"), data);
                        }
                    }
                    Err(error) => {
                            let _ = app_for_stdout.emit(
                            &format!("terminal:data:{stdout_id}"),
                            format!("\r\n[terminal stdout error] {}\r\n", error),
                        );
                        break;
                    }
                }
            }
        });

        tauri::async_runtime::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buffer = vec![0u8; 4096];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(bytes_read) => {
                        let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                        if !data.is_empty() {
                            let _ = app_for_stderr.emit(&format!("terminal:data:{stderr_id}"), data);
                        }
                    }
                    Err(error) => {
                            let _ = app_for_stderr.emit(
                            &format!("terminal:data:{stderr_id}"),
                            format!("\r\n[terminal stderr error] {}\r\n", error),
                        );
                        break;
                    }
                }
            }
        });

        tauri::async_runtime::spawn(async move {
            let status = child.wait().await;
            if let Ok(exit_status) = status {
                let code = exit_status.code().unwrap_or(-1);
                let _ = app.emit(&format!("terminal:exit:{exit_id}"), code);
            } else {
                let _ = app.emit(&format!("terminal:exit:{exit_id}"), -1);
            }
            if let Ok(mut map) = sessions.lock() {
                map.remove(&exit_id);
            }
        });

        Ok(id)
    }

    fn close_terminal(&self, id: &str) -> Result<(), String> {
        let session = self.remove_session(id).ok_or_else(|| format!("Terminal {} not found", id))?;
        Self::kill_process_by_id(session.process_id)
    }
}

struct TerminalSessionHandle {
    process_id: u32,
    stdin: Arc<tokio::sync::Mutex<ChildStdin>>,
}

async fn write_to_session(session: TerminalSessionHandle, data: String) -> Result<(), String> {
    if data.chars().any(|c| c == '\u{3}') {
        TerminalManager::kill_process_by_id(session.process_id)?;
        return Ok(());
    }

    let mut stdin = session.stdin.lock().await;
    stdin
        .write_all(data.as_bytes())
        .await
        .map_err(|error| format!("Failed to write to terminal: {}", error))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Failed to flush terminal input: {}", error))?;
    Ok(())
}

#[tauri::command]
pub fn terminal_create(
    config: TerminalConfig,
    app: AppHandle,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<String, String> {
    let manager = state.lock().map_err(|_| "Terminal manager lock poisoned".to_string())?;
    manager.create_terminal(config, app)
}

#[tauri::command]
pub fn terminal_list(
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<Vec<TerminalInstance>, String> {
    let manager = state.lock().map_err(|_| "Terminal manager lock poisoned".to_string())?;
    Ok(manager.get_sessions())
}

#[tauri::command]
pub fn terminal_get(
    id: String,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<TerminalInstance, String> {
    let manager = state.lock().map_err(|_| "Terminal manager lock poisoned".to_string())?;
    let sessions = manager.terminals.lock().map_err(|_| "Terminal session lock poisoned".to_string())?;
    sessions
        .get(&id)
        .map(|session| session.instance.clone())
        .ok_or_else(|| format!("Terminal {} not found", id))
}

#[tauri::command]
pub fn terminal_close(
    id: String,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<(), String> {
    let manager = state.lock().map_err(|_| "Terminal manager lock poisoned".to_string())?;
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
pub async fn terminal_write(
    terminal_id: String,
    data: String,
    state: State<'_, Arc<Mutex<TerminalManager>>>,
) -> Result<(), String> {
    let session = {
        let manager = state.lock().map_err(|_| "Terminal manager lock poisoned".to_string())?;
        manager.get_session(&terminal_id)?
    };
    write_to_session(session, data).await
}

#[tauri::command]
pub async fn terminal_resize(
    _terminal_id: String,
    _cols: u32,
    _rows: u32,
) -> Result<(), String> {
    Ok(())
}

impl TerminalManager {
    pub fn get_available_shells() -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            vec!["powershell".to_string(), "cmd".to_string()]
        }

        #[cfg(target_os = "macos")]
        {
            vec!["zsh".to_string(), "bash".to_string(), "sh".to_string()]
        }

        #[cfg(target_os = "linux")]
        {
            vec!["bash".to_string(), "zsh".to_string(), "sh".to_string()]
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
