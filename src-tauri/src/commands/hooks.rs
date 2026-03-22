use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub event_type: String,
    pub file_patterns: Option<Vec<String>>,
    pub tool_types: Option<Vec<String>>,
    pub action: String,
    pub prompt: Option<String>,
    pub command: Option<String>,
    pub timeout: Option<u32>,
    pub created_at: u64,
    pub last_triggered: Option<u64>,
    pub trigger_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecution {
    pub hook_id: String,
    pub event_type: String,
    pub triggered_at: u64,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMetrics {
    pub total_hooks: usize,
    pub enabled_hooks: usize,
    pub total_executions: u32,
    pub successful_executions: u32,
    pub failed_executions: u32,
    pub average_execution_time_ms: f32,
}

pub struct HooksManager {
    hooks: Arc<Mutex<HashMap<String, Hook>>>,
    executions: Arc<Mutex<Vec<HookExecution>>>,
}

#[allow(dead_code)]
impl HooksManager {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(Mutex::new(HashMap::new())),
            executions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn add_hook(&self, mut hook: Hook) {
        hook.created_at = Self::current_timestamp();
        let mut hooks = self.hooks.lock().unwrap();
        hooks.insert(hook.id.clone(), hook);
    }

    pub fn remove_hook(&self, hook_id: &str) -> Option<Hook> {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.remove(hook_id)
    }

    pub fn get_hook(&self, hook_id: &str) -> Option<Hook> {
        let hooks = self.hooks.lock().unwrap();
        hooks.get(hook_id).cloned()
    }

    pub fn get_all_hooks(&self) -> Vec<Hook> {
        let hooks = self.hooks.lock().unwrap();
        hooks.values().cloned().collect()
    }

    pub fn get_enabled_hooks(&self) -> Vec<Hook> {
        let hooks = self.hooks.lock().unwrap();
        hooks
            .values()
            .filter(|h| h.enabled)
            .cloned()
            .collect()
    }

    pub fn get_hooks_for_event(&self, event_type: &str) -> Vec<Hook> {
        self.get_enabled_hooks()
            .into_iter()
            .filter(|h| h.event_type == event_type)
            .collect()
    }

    pub fn update_hook(&self, hook: Hook) -> Result<()> {
        let mut hooks = self.hooks.lock().unwrap();
        if hooks.contains_key(&hook.id) {
            hooks.insert(hook.id.clone(), hook);
            Ok(())
        } else {
            Err("Hook not found".into())
        }
    }

    pub fn enable_hook(&self, hook_id: &str) -> Result<()> {
        let mut hooks = self.hooks.lock().unwrap();
        if let Some(hook) = hooks.get_mut(hook_id) {
            hook.enabled = true;
            Ok(())
        } else {
            Err("Hook not found".into())
        }
    }

    pub fn disable_hook(&self, hook_id: &str) -> Result<()> {
        let mut hooks = self.hooks.lock().unwrap();
        if let Some(hook) = hooks.get_mut(hook_id) {
            hook.enabled = false;
            Ok(())
        } else {
            Err("Hook not found".into())
        }
    }

    pub fn matches_file_pattern(&self, file_path: &str, patterns: &[String]) -> bool {
        if patterns.is_empty() {
            return true;
        }

        patterns.iter().any(|pattern| {
            let regex_pattern = pattern
                .replace(".", "\\.")
                .replace("*", ".*")
                .replace("?", ".");
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                regex.is_match(file_path)
            } else {
                false
            }
        })
    }

    pub fn matches_tool_type(&self, tool_name: &str, tool_types: &[String]) -> bool {
        if tool_types.is_empty() {
            return true;
        }

        let categories: HashMap<&str, Vec<&str>> = [
            (
                "read",
                vec![
                    "read_file",
                    "readCode",
                    "readMultipleFiles",
                    "list_directory",
                    "search_files",
                    "grepSearch",
                    "fileSearch",
                ],
            ),
            (
                "write",
                vec![
                    "write_file",
                    "edit_file",
                    "editCode",
                    "delete_file",
                    "strReplace",
                    "smartRelocate",
                ],
            ),
            ("shell", vec!["run_command"]),
            ("web", vec!["remote_web_search", "webFetch"]),
            ("spec", vec!["createSpec", "updateSpec"]),
            ("*", vec!["*"]),
        ]
        .iter()
        .cloned()
        .collect();

        tool_types.iter().any(|tool_type| {
            if let Some(category_tools) = categories.get(tool_type.as_str()) {
                category_tools.contains(&tool_name)
                    || category_tools.contains(&"*")
            } else {
                // Try as regex pattern
                if let Ok(regex) = regex::Regex::new(tool_type) {
                    regex.is_match(tool_name)
                } else {
                    false
                }
            }
        })
    }

    pub fn trigger_file_event(&self, event_type: &str, file_path: &str) -> Vec<Hook> {
        let triggered = self.get_hooks_for_event(event_type)
            .into_iter()
            .filter(|hook| {
                if let Some(patterns) = &hook.file_patterns {
                    self.matches_file_pattern(file_path, patterns)
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        // Record execution
        for hook in &triggered {
            self.record_execution(hook.id.clone(), event_type.to_string(), true, None);
        }

        triggered
    }

    pub fn trigger_tool_event(&self, event_type: &str, tool_name: &str) -> Vec<Hook> {
        let triggered = self.get_hooks_for_event(event_type)
            .into_iter()
            .filter(|hook| {
                if let Some(tool_types) = &hook.tool_types {
                    self.matches_tool_type(tool_name, tool_types)
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        // Record execution
        for hook in &triggered {
            self.record_execution(hook.id.clone(), event_type.to_string(), true, None);
        }

        triggered
    }

    pub fn trigger_event(&self, event_type: &str) -> Vec<Hook> {
        let triggered = self.get_hooks_for_event(event_type);

        // Record execution
        for hook in &triggered {
            self.record_execution(hook.id.clone(), event_type.to_string(), true, None);
        }

        triggered
    }

    pub fn record_execution(&self, hook_id: String, event_type: String, success: bool, error: Option<String>) {
        let execution = HookExecution {
            hook_id: hook_id.clone(),
            event_type,
            triggered_at: Self::current_timestamp(),
            duration_ms: 0,
            success,
            error,
        };

        let mut executions = self.executions.lock().unwrap();
        executions.push(execution);

        // Update hook trigger count and last triggered time
        let mut hooks = self.hooks.lock().unwrap();
        if let Some(hook) = hooks.get_mut(&hook_id) {
            hook.trigger_count += 1;
            hook.last_triggered = Some(Self::current_timestamp());
        }
    }

    pub fn get_execution_history(&self, hook_id: Option<&str>) -> Vec<HookExecution> {
        let executions = self.executions.lock().unwrap();
        if let Some(id) = hook_id {
            executions
                .iter()
                .filter(|e| e.hook_id == id)
                .cloned()
                .collect()
        } else {
            executions.clone()
        }
    }

    pub fn get_metrics(&self) -> HookMetrics {
        let hooks = self.hooks.lock().unwrap();
        let executions = self.executions.lock().unwrap();

        let total_hooks = hooks.len();
        let enabled_hooks = hooks.values().filter(|h| h.enabled).count();
        let total_executions = executions.len() as u32;
        let successful_executions = executions.iter().filter(|e| e.success).count() as u32;
        let failed_executions = total_executions - successful_executions;

        let average_execution_time_ms = if !executions.is_empty() {
            let total_time: u64 = executions.iter().map(|e| e.duration_ms).sum();
            (total_time as f32) / (executions.len() as f32)
        } else {
            0.0
        };

        HookMetrics {
            total_hooks,
            enabled_hooks,
            total_executions,
            successful_executions,
            failed_executions,
            average_execution_time_ms,
        }
    }

    pub fn clear_execution_history(&self) {
        let mut executions = self.executions.lock().unwrap();
        executions.clear();
    }
}

#[tauri::command]
pub async fn hooks_list_all() -> Result<Vec<Hook>> {
    eprintln!("Listing all hooks");
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_get_enabled() -> Result<Vec<Hook>> {
    eprintln!("Getting enabled hooks");
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_add(hook: Hook) -> Result<()> {
    eprintln!("Adding hook: {} ({})", hook.name, hook.event_type);
    Ok(())
}

#[tauri::command]
pub async fn hooks_remove(hook_id: String) -> Result<()> {
    eprintln!("Removing hook: {}", hook_id);
    Ok(())
}

#[tauri::command]
pub async fn hooks_update(hook: Hook) -> Result<()> {
    eprintln!("Updating hook: {}", hook.id);
    Ok(())
}

#[tauri::command]
pub async fn hooks_get_for_event(event_type: String) -> Result<Vec<Hook>> {
    eprintln!("Getting hooks for event: {}", event_type);
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_trigger_file_event(event_type: String, file_path: String) -> Result<Vec<Hook>> {
    eprintln!("Triggering file event: {} for {}", event_type, file_path);
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_trigger_tool_event(event_type: String, tool_name: String) -> Result<Vec<Hook>> {
    eprintln!("Triggering tool event: {} for {}", event_type, tool_name);
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_enable(hook_id: String) -> Result<()> {
    eprintln!("Enabling hook: {}", hook_id);
    Ok(())
}

#[tauri::command]
pub async fn hooks_disable(hook_id: String) -> Result<()> {
    eprintln!("Disabling hook: {}", hook_id);
    Ok(())
}

#[tauri::command]
pub async fn hooks_get_execution_history(hook_id: Option<String>) -> Result<Vec<HookExecution>> {
    eprintln!("Getting execution history for hook: {:?}", hook_id);
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_get_metrics() -> Result<HookMetrics> {
    eprintln!("Getting hooks metrics");
    Ok(HookMetrics {
        total_hooks: 0,
        enabled_hooks: 0,
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0.0,
    })
}

#[tauri::command]
pub async fn hooks_clear_execution_history() -> Result<()> {
    eprintln!("Clearing execution history");
    Ok(())
}
