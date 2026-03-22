use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatThread {
    pub id: String,
    pub title: String,
    pub messages: Vec<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatThreadMetadata {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
}

pub struct HistoryService {
    history_dir: PathBuf,
    threads: Arc<Mutex<HashMap<String, ChatThread>>>,
}

impl HistoryService {
    pub fn new(workspace_path: &str) -> Result<Self, String> {
        let history_dir = Path::new(workspace_path)
            .join(".whizcode")
            .join("history");

        // Create directory if it doesn't exist
        fs::create_dir_all(&history_dir)
            .map_err(|e| format!("Failed to create history directory: {}", e))?;

        Ok(HistoryService {
            history_dir,
            threads: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn save_thread(
        &self,
        id: String,
        title: String,
        messages: Vec<serde_json::Value>,
    ) -> Result<String, String> {
        let now = Utc::now().timestamp();
        let thread = ChatThread {
            id: id.clone(),
            title,
            messages,
            created_at: now,
            updated_at: now,
        };

        // Save to file
        let file_path = self.history_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&thread)
            .map_err(|e| format!("Failed to serialize thread: {}", e))?;

        fs::write(&file_path, json)
            .map_err(|e| format!("Failed to write thread file: {}", e))?;

        // Update in-memory cache
        let mut threads = self.threads.lock().unwrap();
        threads.insert(id.clone(), thread);

        Ok(file_path.to_string_lossy().to_string())
    }

    pub fn list_threads(&self) -> Result<Vec<ChatThreadMetadata>, String> {
        let mut threads = Vec::new();

        // Read from disk
        let entries = fs::read_dir(&self.history_dir)
            .map_err(|e| format!("Failed to read history directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(thread) = serde_json::from_str::<ChatThread>(&content) {
                        threads.push(ChatThreadMetadata {
                            id: thread.id,
                            title: thread.title,
                            created_at: thread.created_at,
                            updated_at: thread.updated_at,
                            message_count: thread.messages.len(),
                        });
                    }
                }
            }
        }

        // Sort by updated_at descending
        threads.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(threads)
    }

    pub fn get_thread(&self, id: &str) -> Result<ChatThread, String> {
        // Check in-memory cache first
        {
            let threads = self.threads.lock().unwrap();
            if let Some(thread) = threads.get(id) {
                return Ok(thread.clone());
            }
        }

        // Load from disk
        let file_path = self.history_dir.join(format!("{}.json", id));
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read thread file: {}", e))?;

        let thread = serde_json::from_str::<ChatThread>(&content)
            .map_err(|e| format!("Failed to parse thread: {}", e))?;

        // Update cache
        let mut threads = self.threads.lock().unwrap();
        threads.insert(id.to_string(), thread.clone());

        Ok(thread)
    }

    pub fn delete_thread(&self, id: &str) -> Result<(), String> {
        let file_path = self.history_dir.join(format!("{}.json", id));
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete thread file: {}", e))?;

        // Remove from cache
        let mut threads = self.threads.lock().unwrap();
        threads.remove(id);

        Ok(())
    }

    pub fn search_threads(&self, query: &str) -> Result<Vec<ChatThreadMetadata>, String> {
        let all_threads = self.list_threads()?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<ChatThreadMetadata> = all_threads
            .into_iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&query_lower)
                    || t.id.to_lowercase().contains(&query_lower)
            })
            .collect();

        Ok(filtered)
    }

    pub fn update_thread(
        &self,
        id: String,
        title: Option<String>,
        messages: Option<Vec<serde_json::Value>>,
    ) -> Result<(), String> {
        let mut thread = self.get_thread(&id)?;

        if let Some(new_title) = title {
            thread.title = new_title;
        }

        if let Some(new_messages) = messages {
            thread.messages = new_messages;
        }

        thread.updated_at = Utc::now().timestamp();

        // Save updated thread
        let file_path = self.history_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&thread)
            .map_err(|e| format!("Failed to serialize thread: {}", e))?;

        fs::write(&file_path, json)
            .map_err(|e| format!("Failed to write thread file: {}", e))?;

        // Update cache
        let mut threads = self.threads.lock().unwrap();
        threads.insert(id, thread);

        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), String> {
        let mut threads = self.threads.lock().unwrap();
        threads.clear();
        Ok(())
    }
}

// Tauri Commands

#[tauri::command]
pub fn history_save(
    id: String,
    title: String,
    messages: Vec<serde_json::Value>,
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<String, String> {
    let service = state.lock().unwrap();
    service.save_thread(id, title, messages)
}

#[tauri::command]
pub fn history_list(
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<Vec<ChatThreadMetadata>, String> {
    let service = state.lock().unwrap();
    service.list_threads()
}

#[tauri::command]
pub fn history_get(
    id: String,
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<ChatThread, String> {
    let service = state.lock().unwrap();
    service.get_thread(&id)
}

#[tauri::command]
pub fn history_delete(
    id: String,
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.delete_thread(&id)
}

#[tauri::command]
pub fn history_search(
    query: String,
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<Vec<ChatThreadMetadata>, String> {
    let service = state.lock().unwrap();
    service.search_threads(&query)
}

#[tauri::command]
pub fn history_update(
    id: String,
    title: Option<String>,
    messages: Option<Vec<serde_json::Value>>,
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.update_thread(id, title, messages)
}

#[tauri::command]
pub fn history_clear_cache(
    state: State<'_, Arc<Mutex<HistoryService>>>,
) -> Result<(), String> {
    let service = state.lock().unwrap();
    service.clear_cache()
}
