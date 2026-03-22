use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub result: serde_json::Value,
    pub created_at: i64,
    pub last_accessed: i64,
    pub access_count: u32,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size_bytes: usize,
    pub default_ttl_seconds: u64,
    pub cleanup_interval_seconds: u64,
}

pub struct ToolResultCache {
    entries: Arc<Mutex<HashMap<String, CacheEntry>>>,
    stats: Arc<Mutex<CacheStats>>,
    config: CacheConfig,
}

impl ToolResultCache {
    pub fn new(config: Option<CacheConfig>) -> Self {
        let default_config = CacheConfig {
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            default_ttl_seconds: 3600,           // 1 hour
            cleanup_interval_seconds: 300,       // 5 minutes
        };

        let config = config.unwrap_or(default_config);

        ToolResultCache {
            entries: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats {
                total_entries: 0,
                total_size_bytes: 0,
                hit_count: 0,
                miss_count: 0,
                hit_rate: 0.0,
            })),
            config,
        }
    }

    pub fn get(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        if let Some(entry) = entries.get_mut(key) {
            let now = Utc::now().timestamp() as u64;
            let age = now - entry.created_at as u64;

            // Check if entry has expired
            if age > entry.ttl_seconds {
                entries.remove(key);
                stats.miss_count += 1;
                Self::update_hit_rate(&mut stats);
                return Ok(None);
            }

            // Update access info
            entry.last_accessed = Utc::now().timestamp();
            entry.access_count += 1;

            stats.hit_count += 1;
            Self::update_hit_rate(&mut stats);

            Ok(Some(entry.result.clone()))
        } else {
            stats.miss_count += 1;
            Self::update_hit_rate(&mut stats);
            Ok(None)
        }
    }

    pub fn set(
        &self,
        key: String,
        result: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl_seconds);
        let size = Self::estimate_size(&result);

        // Check if we need to evict entries
        if stats.total_size_bytes + size > self.config.max_size_bytes {
            Self::evict_lru(&mut entries, &mut stats, size);
        }

        let entry = CacheEntry {
            key: key.clone(),
            result,
            created_at: Utc::now().timestamp(),
            last_accessed: Utc::now().timestamp(),
            access_count: 0,
            ttl_seconds: ttl,
        };

        stats.total_size_bytes += size;
        stats.total_entries = entries.len() + 1;

        entries.insert(key, entry);

        Ok(())
    }

    pub fn invalidate(&self, key: Option<&str>) -> Result<u32, String> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let count = if let Some(k) = key {
            if entries.remove(k).is_some() {
                1
            } else {
                0
            }
        } else {
            let count = entries.len() as u32;
            entries.clear();
            count
        };

        stats.total_entries = entries.len();
        Ok(count)
    }

    pub fn cleanup(&self) -> Result<u32, String> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();
        let now = Utc::now().timestamp() as u64;

        let mut removed = 0;
        entries.retain(|_, entry| {
            let age = now - entry.created_at as u64;
            if age > entry.ttl_seconds {
                removed += 1;
                false
            } else {
                true
            }
        });

        stats.total_entries = entries.len();
        Ok(removed)
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        entries.clear();
        stats.total_entries = 0;
        stats.total_size_bytes = 0;
        stats.hit_count = 0;
        stats.miss_count = 0;
        stats.hit_rate = 0.0;

        Ok(())
    }

    pub fn get_stats(&self) -> Result<CacheStats, String> {
        let stats = self.stats.lock().unwrap();
        Ok(stats.clone())
    }

    fn estimate_size(value: &serde_json::Value) -> usize {
        serde_json::to_string(value)
            .map(|s| s.len())
            .unwrap_or(1024)
    }

    fn evict_lru(
        entries: &mut HashMap<String, CacheEntry>,
        stats: &mut CacheStats,
        required_size: usize,
    ) {
        let mut sorted: Vec<String> = entries
            .iter()
            .map(|(k, _)| k.clone())
            .collect();
        sorted.sort_by_key(|key| {
            entries
                .get(key)
                .map(|e| e.last_accessed)
                .unwrap_or(0)
        });

        let mut freed = 0;
        for key in sorted {
            if freed >= required_size {
                break;
            }
            if let Some(entry) = entries.get(&key) {
                freed += Self::estimate_size(&entry.result);
            }
            entries.remove(&key);
        }

        stats.total_entries = entries.len();
    }

    fn update_hit_rate(stats: &mut CacheStats) {
        let total = stats.hit_count + stats.miss_count;
        if total > 0 {
            stats.hit_rate = stats.hit_count as f64 / total as f64;
        }
    }
}

// Tauri Commands

#[tauri::command]
pub fn cache_get(
    key: String,
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<Option<serde_json::Value>, String> {
    let cache = state.lock().unwrap();
    cache.get(&key)
}

#[tauri::command]
pub fn cache_set(
    key: String,
    result: serde_json::Value,
    ttl_seconds: Option<u64>,
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    let cache = state.lock().unwrap();
    cache.set(key, result, ttl_seconds)
}

#[tauri::command]
pub fn cache_invalidate(
    key: Option<String>,
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<u32, String> {
    let cache = state.lock().unwrap();
    cache.invalidate(key.as_deref())
}

#[tauri::command]
pub fn cache_cleanup(
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<u32, String> {
    let cache = state.lock().unwrap();
    cache.cleanup()
}

#[tauri::command]
pub fn cache_clear(
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<(), String> {
    let cache = state.lock().unwrap();
    cache.clear()
}

#[tauri::command]
pub fn cache_get_stats(
    state: State<'_, Arc<Mutex<ToolResultCache>>>,
) -> Result<CacheStats, String> {
    let cache = state.lock().unwrap();
    cache.get_stats()
}
