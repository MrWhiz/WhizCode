use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub data: T,
    pub created_at: i64,
    pub last_accessed: i64,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
}

/// Unified Caching System for WhizCode
pub struct WhizCache<T: Clone + Serialize + for<'de> Deserialize<'de>> {
    entries: Arc<Mutex<HashMap<String, CacheEntry<T>>>>,
    stats: Arc<Mutex<CacheStats>>,
    default_ttl: u64,
}

impl<T: Clone + Serialize + for<'de> Deserialize<'de>> WhizCache<T> {
    pub fn new(default_ttl: u64) -> Self {
        WhizCache {
            entries: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats {
                total_entries: 0,
                hit_count: 0,
                miss_count: 0,
            })),
            default_ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let mut entries = self.entries.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        if let Some(entry) = entries.get_mut(key) {
            let now = Utc::now().timestamp();
            if now - entry.created_at > entry.ttl_seconds as i64 {
                entries.remove(key);
                stats.miss_count += 1;
                return None;
            }

            entry.last_accessed = now;
            stats.hit_count += 1;
            Some(entry.data.clone())
        } else {
            stats.miss_count += 1;
            None
        }
    }

    pub fn set(&self, key: String, data: T, ttl: Option<u64>) {
        let mut entries = self.entries.lock().unwrap();
        let now = Utc::now().timestamp();
        
        entries.insert(key.clone(), CacheEntry {
            key,
            data,
            created_at: now,
            last_accessed: now,
            ttl_seconds: ttl.unwrap_or(self.default_ttl),
        });
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
    }

    pub fn get_stats(&self) -> CacheStats {
        let entries = self.entries.lock().unwrap();
        let stats = self.stats.lock().unwrap();
        CacheStats {
            total_entries: entries.len(),
            hit_count: stats.hit_count,
            miss_count: stats.miss_count,
        }
    }
}
