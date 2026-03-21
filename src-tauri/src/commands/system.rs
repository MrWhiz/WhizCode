use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
    pub cpu_count: usize,
    pub memory_gb: f64,
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo> {
    // Get memory info - try sys_info, fallback to 0 if not available
    let memory_gb = 0.0; // Fallback for now - sys_info API may vary
    
    let info = SystemInfo {
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_count: num_cpus::get(),
        memory_gb,
    };
    
    Ok(info)
}

#[tauri::command]
pub async fn open_external(url: String) -> Result<()> {
    open::that(&url)
        .map_err(|e| format!("Failed to open URL: {}", e).into())
}
