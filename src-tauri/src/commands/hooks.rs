use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

pub struct HooksManager {
    hooks: HashMap<String, Hook>,
}

#[allow(dead_code)]
impl HooksManager {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn add_hook(&mut self, hook: Hook) {
        self.hooks.insert(hook.id.clone(), hook);
    }

    pub fn remove_hook(&mut self, hook_id: &str) -> Option<Hook> {
        self.hooks.remove(hook_id)
    }

    pub fn get_hook(&self, hook_id: &str) -> Option<&Hook> {
        self.hooks.get(hook_id)
    }

    pub fn get_all_hooks(&self) -> Vec<Hook> {
        self.hooks.values().cloned().collect()
    }

    pub fn get_enabled_hooks(&self) -> Vec<Hook> {
        self.hooks
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
        self.get_hooks_for_event(event_type)
            .into_iter()
            .filter(|hook| {
                if let Some(patterns) = &hook.file_patterns {
                    self.matches_file_pattern(file_path, patterns)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn trigger_tool_event(&self, event_type: &str, tool_name: &str) -> Vec<Hook> {
        self.get_hooks_for_event(event_type)
            .into_iter()
            .filter(|hook| {
                if let Some(tool_types) = &hook.tool_types {
                    self.matches_tool_type(tool_name, tool_types)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn trigger_event(&self, event_type: &str) -> Vec<Hook> {
        self.get_hooks_for_event(event_type)
    }
}

#[tauri::command]
pub async fn hooks_list_all() -> Result<Vec<Hook>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn hooks_get_enabled() -> Result<Vec<Hook>> {
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
