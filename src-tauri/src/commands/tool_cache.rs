use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub hit_rate: f32,
    pub miss_rate: f32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub timestamp: u64,
    pub hits: u32,
}

pub struct ToolCache {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    stats: Arc<Mutex<CacheStats>>,
}

impl ToolCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        ToolCache {
            cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats {
                total_entries: 0,
                total_size_bytes: 0,
                hit_rate: 0.0,
                miss_rate: 0.0,
            })),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(key) {
            entry.hits += 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn set(&self, key: String, value: String) {
        let mut cache = self.cache.lock().unwrap();
        let size = value.len();
        let key_clone = key.clone();
        cache.insert(
            key,
            CacheEntry {
                key: key_clone,
                value,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                hits: 0,
            },
        );

        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = cache.len();
        stats.total_size_bytes += size;
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
pub fn tool_cache_get(
    key: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolCache>>>,
) -> Option<String> {
    state.lock().ok().and_then(|cache| cache.get(&key))
}

#[tauri::command]
pub fn tool_cache_clear(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolCache>>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.clear())
}

#[tauri::command]
pub fn tool_cache_get_stats(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolCache>>>,
) -> Result<CacheStats, String> {
    state
        .lock()
        .map_err(|e| e.to_string())
        .map(|cache| cache.get_stats())
}
