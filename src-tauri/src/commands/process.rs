use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use sysinfo::System;
use tauri::State;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub process_type: String,
    pub workspace_related: bool,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCheckResult {
    pub processes: Vec<RunningProcess>,
    pub port_conflicts: Vec<u16>,
    pub dev_servers_running: bool,
    pub timestamp: i64,
}

pub struct ProcessManager {
    process_history: Arc<Mutex<Vec<RunningProcess>>>,
    system: Arc<Mutex<System>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            process_history: Arc::new(Mutex::new(Vec::new())),
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    pub fn check_for_running_instances(
        &self,
        workspace_path: &str,
    ) -> Result<ProcessCheckResult, String> {
        let mut system = self.system.lock().unwrap();
        system.refresh_all();

        let mut processes = Vec::new();
        let mut port_conflicts = Vec::new();
        let mut dev_servers_running = false;

        for (_, process) in system.processes() {
            let cmd = process.cmd().join(" ");
            let name = process.name().to_string();

            // Check if process is workspace-related
            let workspace_related = cmd.contains(workspace_path) || 
                                   name.contains("node") || 
                                   name.contains("npm") ||
                                   name.contains("yarn") ||
                                   name.contains("webpack") ||
                                   name.contains("vite");

            if workspace_related {
                let process_type = Self::classify_process_type(&cmd);
                
                if process_type == "dev-server" {
                    dev_servers_running = true;
                }

                let port = Self::extract_port_from_command(&cmd);
                if let Some(p) = port {
                    port_conflicts.push(p);
                }

                processes.push(RunningProcess {
                    pid: process.pid().as_u32(),
                    name,
                    command: cmd,
                    process_type,
                    workspace_related: true,
                    port,
                });
            }
        }

        // Remove duplicates from port_conflicts
        port_conflicts.sort();
        port_conflicts.dedup();

        let result = ProcessCheckResult {
            processes,
            port_conflicts,
            dev_servers_running,
            timestamp: Utc::now().timestamp(),
        };

        Ok(result)
    }

    pub fn stop_processes(&self, pids: Vec<u32>) -> Result<(u32, u32, Vec<String>), String> {
        let mut stopped = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for pid in pids {
            match Self::stop_process(pid) {
                Ok(_) => stopped += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(e);
                }
            }
        }

        Ok((stopped, failed, errors))
    }

    fn stop_process(pid: u32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        Ok(())
    }

    fn classify_process_type(command: &str) -> String {
        let lower = command.to_lowercase();
        if lower.contains("dev") || lower.contains("serve") || lower.contains("watch") {
            "dev-server".to_string()
        } else if lower.contains("build") {
            "build".to_string()
        } else if lower.contains("test") {
            "test".to_string()
        } else {
            "other".to_string()
        }
    }

    fn extract_port_from_command(command: &str) -> Option<u16> {
        // Look for common port patterns
        let patterns = vec![
            r"--port\s+(\d+)",
            r"-p\s+(\d+)",
            r":(\d{4,5})",
            r"PORT=(\d+)",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(command) {
                    if let Ok(port) = caps[1].parse::<u16>() {
                        return Some(port);
                    }
                }
            }
        }
        None
    }

    pub fn get_running_processes_summary(&self) -> Result<String, String> {
        let history = self.process_history.lock().unwrap();
        if history.is_empty() {
            return Ok("No processes tracked".to_string());
        }

        let mut summary = String::from("Running Processes:\n");
        for proc in history.iter() {
            summary.push_str(&format!(
                "- {} (PID: {}, Type: {})\n",
                proc.name, proc.pid, proc.process_type
            ));
        }
        Ok(summary)
    }

    pub fn clear_tracked_processes(&self) -> Result<(), String> {
        let mut history = self.process_history.lock().unwrap();
        history.clear();
        Ok(())
    }
}

// Tauri Commands

#[tauri::command]
pub fn process_check(
    workspace_path: String,
    state: State<'_, Arc<Mutex<ProcessManager>>>,
) -> Result<ProcessCheckResult, String> {
    let manager = state.lock().unwrap();
    manager.check_for_running_instances(&workspace_path)
}

#[tauri::command]
pub fn process_stop(
    pids: Vec<u32>,
    state: State<'_, Arc<Mutex<ProcessManager>>>,
) -> Result<(u32, u32, Vec<String>), String> {
    let manager = state.lock().unwrap();
    manager.stop_processes(pids)
}

#[tauri::command]
pub fn process_list(
    workspace_path: String,
    state: State<'_, Arc<Mutex<ProcessManager>>>,
) -> Result<Vec<RunningProcess>, String> {
    let manager = state.lock().unwrap();
    let result = manager.check_for_running_instances(&workspace_path)?;
    Ok(result.processes)
}

#[tauri::command]
pub fn process_summary(
    state: State<'_, Arc<Mutex<ProcessManager>>>,
) -> Result<String, String> {
    let manager = state.lock().unwrap();
    manager.get_running_processes_summary()
}

#[tauri::command]
pub fn process_clear(
    state: State<'_, Arc<Mutex<ProcessManager>>>,
) -> Result<(), String> {
    let manager = state.lock().unwrap();
    manager.clear_tracked_processes()
}
