use serde::{Deserialize, Serialize};
use crate::error::Result;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedToolResult {
    pub tool: String,
    pub args: serde_json::Value,
    pub result: String,
    pub timestamp: u64,
    pub ttl_seconds: u64,
}

pub struct ToolResultCache {
    cache: HashMap<String, CachedToolResult>,
}

impl ToolResultCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_cache_key(tool: &str, args: &serde_json::Value) -> String {
        format!("{}:{}", tool, serde_json::to_string(args).unwrap_or_default())
    }

    #[allow(dead_code)]
    pub fn get(&self, tool: &str, args: &serde_json::Value) -> Option<String> {
        let key = Self::get_cache_key(tool, args);
        
        if let Some(cached) = self.cache.get(&key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if now - cached.timestamp < cached.ttl_seconds {
                return Some(cached.result.clone());
            }
        }
        
        None
    }

    #[allow(dead_code)]
    pub fn set(&mut self, tool: &str, args: serde_json::Value, result: String, ttl_seconds: u64) {
        let key = Self::get_cache_key(tool, &args);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.cache.insert(key, CachedToolResult {
            tool: tool.to_string(),
            args,
            result,
            timestamp,
            ttl_seconds,
        });
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "total_entries": self.cache.len(),
            "entries": self.cache.values().map(|c| serde_json::json!({
                "tool": c.tool,
                "timestamp": c.timestamp,
                "ttl": c.ttl_seconds,
            })).collect::<Vec<_>>(),
        })
    }
}

#[tauri::command]
pub async fn tool_cache_get(_tool: String, _args: serde_json::Value) -> Result<Option<String>> {
    // This would need to be called with shared state
    Ok(None)
}

#[tauri::command]
pub async fn tool_cache_clear() -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn tool_cache_get_stats() -> Result<serde_json::Value> {
    Ok(serde_json::json!({"status": "cache_stats"}))
}
