use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringFile {
    pub id: String,
    pub path: String,
    pub name: String,
    pub content: String,
    pub inclusion_type: String, // "auto", "manual", "fileMatch"
    pub file_match_pattern: Option<String>,
    pub enabled: bool,
    pub created_at: u64,
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FrontMatter {
    pub inclusion: String,
    pub file_match_pattern: Option<String>,
    pub priority: Option<u32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringContext {
    pub workspace_path: String,
    pub active_steering_files: Vec<String>,
    pub injected_context: String,
    pub total_context_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringMetrics {
    pub total_steering_files: usize,
    pub enabled_steering_files: usize,
    pub auto_inclusion_files: usize,
    pub manual_inclusion_files: usize,
    pub file_match_files: usize,
    pub total_context_injected: usize,
}

#[allow(dead_code)]
pub struct SteeringSystem {
    steering_files: Arc<Mutex<HashMap<String, SteeringFile>>>,
    active_contexts: Arc<Mutex<HashMap<String, SteeringContext>>>,
}

#[allow(dead_code)]
impl SteeringSystem {
    pub fn new() -> Self {
        Self {
            steering_files: Arc::new(Mutex::new(HashMap::new())),
            active_contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn add_steering_file(&self, mut file: SteeringFile) -> Result<()> {
        file.created_at = Self::current_timestamp();
        file.last_modified = Self::current_timestamp();
        let mut files = self.steering_files.lock().unwrap();
        files.insert(file.id.clone(), file);
        Ok(())
    }

    pub fn remove_steering_file(&self, file_id: &str) -> Result<()> {
        let mut files = self.steering_files.lock().unwrap();
        files.remove(file_id);
        Ok(())
    }

    pub fn get_steering_file(&self, file_id: &str) -> Option<SteeringFile> {
        let files = self.steering_files.lock().unwrap();
        files.get(file_id).cloned()
    }

    pub fn get_all_steering_files(&self) -> Vec<SteeringFile> {
        let files = self.steering_files.lock().unwrap();
        files.values().cloned().collect()
    }

    pub fn get_enabled_steering_files(&self) -> Vec<SteeringFile> {
        let files = self.steering_files.lock().unwrap();
        files
            .values()
            .filter(|f| f.enabled)
            .cloned()
            .collect()
    }

    pub fn enable_steering_file(&self, file_id: &str) -> Result<()> {
        let mut files = self.steering_files.lock().unwrap();
        if let Some(file) = files.get_mut(file_id) {
            file.enabled = true;
            file.last_modified = Self::current_timestamp();
            Ok(())
        } else {
            Err("Steering file not found".into())
        }
    }

    pub fn disable_steering_file(&self, file_id: &str) -> Result<()> {
        let mut files = self.steering_files.lock().unwrap();
        if let Some(file) = files.get_mut(file_id) {
            file.enabled = false;
            file.last_modified = Self::current_timestamp();
            Ok(())
        } else {
            Err("Steering file not found".into())
        }
    }

    pub fn update_steering_file(&self, file: SteeringFile) -> Result<()> {
        let mut files = self.steering_files.lock().unwrap();
        if files.contains_key(&file.id) {
            let mut updated_file = file;
            updated_file.last_modified = Self::current_timestamp();
            files.insert(updated_file.id.clone(), updated_file);
            Ok(())
        } else {
            Err("Steering file not found".into())
        }
    }

    pub fn parse_front_matter(&self, content: &str) -> Option<(FrontMatter, String)> {
        if !content.starts_with("---") {
            return None;
        }

        let lines: Vec<&str> = content.lines().collect();
        let mut end_index = 0;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_index = i;
                break;
            }
        }

        if end_index == 0 {
            return None;
        }

        let front_matter_str = lines[1..end_index].join("\n");
        let body = lines[end_index + 1..].join("\n");

        // Simple YAML-like parsing
        let mut front_matter = FrontMatter {
            inclusion: "manual".to_string(),
            file_match_pattern: None,
            priority: None,
            tags: None,
        };

        for line in front_matter_str.lines() {
            if line.starts_with("inclusion:") {
                front_matter.inclusion = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("fileMatchPattern:") {
                front_matter.file_match_pattern = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
            } else if line.starts_with("priority:") {
                if let Ok(p) = line.split(':').nth(1).unwrap_or("").trim().parse() {
                    front_matter.priority = Some(p);
                }
            }
        }

        Some((front_matter, body))
    }

    pub fn load_steering_files_for_context(&self, workspace_path: &str, current_file: Option<&str>) -> Result<SteeringContext> {
        let files = self.get_enabled_steering_files();
        let mut active_files = vec![];
        let mut injected_context = String::new();
        let mut total_size = 0;

        for file in files {
            let should_include = match file.inclusion_type.as_str() {
                "auto" => true,
                "manual" => false,
                "fileMatch" => {
                    if let (Some(pattern), Some(current)) = (&file.file_match_pattern, current_file) {
                        self.matches_pattern(current, pattern)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if should_include {
                active_files.push(file.id.clone());
                injected_context.push_str(&format!("\n\n--- {} ---\n{}\n", file.name, file.content));
                total_size += file.content.len();
            }
        }

        let context = SteeringContext {
            workspace_path: workspace_path.to_string(),
            active_steering_files: active_files,
            injected_context,
            total_context_size: total_size,
        };

        let mut contexts = self.active_contexts.lock().unwrap();
        contexts.insert(workspace_path.to_string(), context.clone());

        Ok(context)
    }

    fn matches_pattern(&self, file_path: &str, pattern: &str) -> bool {
        let regex_pattern = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("?", ".");
        if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(file_path)
        } else {
            false
        }
    }

    pub fn get_injected_context(&self, workspace_path: &str) -> Option<String> {
        let contexts = self.active_contexts.lock().unwrap();
        contexts.get(workspace_path).map(|c| c.injected_context.clone())
    }

    pub fn get_metrics(&self) -> SteeringMetrics {
        let files = self.steering_files.lock().unwrap();
        let total = files.len();
        let enabled = files.values().filter(|f| f.enabled).count();
        let auto = files.values().filter(|f| f.inclusion_type == "auto").count();
        let manual = files.values().filter(|f| f.inclusion_type == "manual").count();
        let file_match = files.values().filter(|f| f.inclusion_type == "fileMatch").count();

        let total_context: usize = files.values().map(|f| f.content.len()).sum();

        SteeringMetrics {
            total_steering_files: total,
            enabled_steering_files: enabled,
            auto_inclusion_files: auto,
            manual_inclusion_files: manual,
            file_match_files: file_match,
            total_context_injected: total_context,
        }
    }

    pub fn clear_context(&self, workspace_path: &str) {
        let mut contexts = self.active_contexts.lock().unwrap();
        contexts.remove(workspace_path);
    }
}

#[tauri::command]
pub async fn steering_add_file(file: SteeringFile) -> Result<()> {
    eprintln!("Adding steering file: {} ({})", file.name, file.inclusion_type);
    Ok(())
}

#[tauri::command]
pub async fn steering_remove_file(file_id: String) -> Result<()> {
    eprintln!("Removing steering file: {}", file_id);
    Ok(())
}

#[tauri::command]
pub async fn steering_get_file(file_id: String) -> Result<Option<SteeringFile>> {
    eprintln!("Getting steering file: {}", file_id);
    Ok(None)
}

#[tauri::command]
pub async fn steering_list_all() -> Result<Vec<SteeringFile>> {
    eprintln!("Listing all steering files");
    Ok(vec![])
}

#[tauri::command]
pub async fn steering_get_enabled() -> Result<Vec<SteeringFile>> {
    eprintln!("Getting enabled steering files");
    Ok(vec![])
}

#[tauri::command]
pub async fn steering_enable_file(file_id: String) -> Result<()> {
    eprintln!("Enabling steering file: {}", file_id);
    Ok(())
}

#[tauri::command]
pub async fn steering_disable_file(file_id: String) -> Result<()> {
    eprintln!("Disabling steering file: {}", file_id);
    Ok(())
}

#[tauri::command]
pub async fn steering_update_file(file: SteeringFile) -> Result<()> {
    eprintln!("Updating steering file: {}", file.id);
    Ok(())
}

#[tauri::command]
pub async fn steering_load_context(workspace_path: String, _current_file: Option<String>) -> Result<SteeringContext> {
    eprintln!("Loading steering context for: {}", workspace_path);
    Ok(SteeringContext {
        workspace_path,
        active_steering_files: vec![],
        injected_context: String::new(),
        total_context_size: 0,
    })
}

#[tauri::command]
pub async fn steering_get_injected_context(workspace_path: String) -> Result<Option<String>> {
    eprintln!("Getting injected context for: {}", workspace_path);
    Ok(None)
}

#[tauri::command]
pub async fn steering_get_metrics() -> Result<SteeringMetrics> {
    eprintln!("Getting steering metrics");
    Ok(SteeringMetrics {
        total_steering_files: 0,
        enabled_steering_files: 0,
        auto_inclusion_files: 0,
        manual_inclusion_files: 0,
        file_match_files: 0,
        total_context_injected: 0,
    })
}

#[tauri::command]
pub async fn steering_clear_context(workspace_path: String) -> Result<()> {
    eprintln!("Clearing steering context for: {}", workspace_path);
    Ok(())
}
