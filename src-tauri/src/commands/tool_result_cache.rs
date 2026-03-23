use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub timestamp: u64,
    pub ttl: Option<u64>,
    pub hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub hit_rate: f32,
    pub miss_rate: f32,
    pub expired_entries: usize,
}

pub struct ToolResultCache {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    stats: Arc<Mutex<CacheStats>>,
}

impl ToolResultCache {
    pub fn new(_max_size: Option<usize>) -> Self {
        ToolResultCache {
            cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats {
                total_entries: 0,
                total_size_bytes: 0,
                hit_rate: 0.0,
                miss_rate: 0.0,
                expired_entries: 0,
            })),
        }
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(key) {
            // Check if expired
            if let Some(ttl) = entry.ttl {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now > entry.timestamp + ttl {
                    cache.remove(key);
                    return None;
                }
            }
            entry.hits += 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn set(&self, key: String, value: serde_json::Value, ttl: Option<u64>) {
        let mut cache = self.cache.lock().unwrap();
        let size = value.to_string().len();
        cache.insert(
            key.clone(),
            CacheEntry {
                key,
                value,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                ttl,
                hits: 0,
            },
        );

        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = cache.len();
        stats.total_size_bytes += size;
    }

    pub fn invalidate(&self, key: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = cache.len();
    }

    pub fn cleanup(&self) {
        let mut cache = self.cache.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let _expired_count = cache
            .retain(|_, entry| {
                if let Some(ttl) = entry.ttl {
                    now <= entry.timestamp + ttl
                } else {
                    true
                }
            });

        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = cache.len();
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = 0;
        stats.total_size_bytes = 0;
    }

    pub fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }
}

#[tauri::command]
pub fn cache_get(
    key: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Option<serde_json::Value> {
    state.lock().ok().and_then(|cache| cache.get(&key))
}

#[tauri::command]
pub fn cache_set(
    key: String,
    value: serde_json::Value,
    ttl: Option<u64>,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.set(key, value, ttl))
}

#[tauri::command]
pub fn cache_invalidate(
    key: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.invalidate(&key))
}

#[tauri::command]
pub fn cache_cleanup(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.cleanup())
}

#[tauri::command]
pub fn cache_clear(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.clear())
}

#[tauri::command]
pub fn cache_get_stats(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolResultCache>>>,
) -> Result<CacheStats, String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.get_stats())
}
