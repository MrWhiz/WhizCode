use serde::{Deserialize, Serialize};
use crate::error::{Result, ApiError};

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
        .map_err(|e| ApiError::from(format!("Failed to open URL: {}", e)))
}

#[tauri::command]
pub async fn reveal_in_folder(path: String) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let path = path.replace("/", "\\");
        
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(|e| ApiError::from(format!("Failed to reveal in explorer: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| ApiError::from(format!("Failed to reveal in Finder: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        let p = std::path::Path::new(&path);
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| ApiError::from(format!("Failed to open directory: {}", e)))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_terminal_at(path: String) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .arg("/c")
            .arg("start")
            .arg("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg(format!("Set-Location -Path '{}'", path))
            .spawn()
            .map_err(|e| ApiError::from(format!("Failed to open terminal: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(path)
            .spawn()
            .map_err(|e| ApiError::from(format!("Failed to open terminal: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("x-terminal-emulator")
            .arg("--working-directory")
            .arg(path)
            .spawn()
            .or_else(|_| {
                std::process::Command::new("gnome-terminal")
                    .arg("--working-directory")
                    .arg(path)
                    .spawn()
            })
            .map_err(|e| ApiError::from(format!("Failed to open terminal: {}", e)))?;
    }
    Ok(())
}
