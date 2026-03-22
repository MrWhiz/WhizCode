use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_number: u32,
    pub change_type: String, // 'added' | 'removed' | 'unchanged'
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
    pub hunks: Vec<DiffHunk>,
    pub additions: u32,
    pub deletions: u32,
    pub generated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub id: String,
    pub file_path: String,
    pub diff: FileDiff,
    pub timestamp: u64,
    pub author: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub total_changes: usize,
    pub total_additions: u32,
    pub total_deletions: u32,
    pub files_changed: usize,
    pub last_change: u64,
}

#[allow(dead_code)]
pub struct DiffService {
    changes: Arc<Mutex<Vec<ChangeRecord>>>,
    file_history: Arc<Mutex<HashMap<String, Vec<ChangeRecord>>>>,
}

#[allow(dead_code)]
impl DiffService {
    pub fn new() -> Self {
        Self {
            changes: Arc::new(Mutex::new(Vec::new())),
            file_history: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn generate_diff(
        file_path: String,
        old_content: String,
        new_content: String,
    ) -> Result<FileDiff> {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let mut hunks = vec![];
        let mut additions = 0;
        let mut deletions = 0;

        // Simple diff algorithm (Myers-like)
        let mut i = 0;
        let mut j = 0;

        while i < old_lines.len() || j < new_lines.len() {
            if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
                i += 1;
                j += 1;
            } else if i < old_lines.len() {
                deletions += 1;
                i += 1;
            } else if j < new_lines.len() {
                additions += 1;
                j += 1;
            }
        }

        // Create a simple hunk
        let mut lines = vec![];
        for (idx, line) in old_lines.iter().enumerate() {
            lines.push(DiffLine {
                line_number: (idx + 1) as u32,
                change_type: "removed".to_string(),
                content: line.to_string(),
            });
        }
        for (idx, line) in new_lines.iter().enumerate() {
            lines.push(DiffLine {
                line_number: (idx + 1) as u32,
                change_type: "added".to_string(),
                content: line.to_string(),
            });
        }

        if !lines.is_empty() {
            hunks.push(DiffHunk {
                old_start: 1,
                old_count: old_lines.len() as u32,
                new_start: 1,
                new_count: new_lines.len() as u32,
                lines,
            });
        }

        Ok(FileDiff {
            file_path,
            old_content,
            new_content,
            hunks,
            additions,
            deletions,
            generated_at: Self::current_timestamp(),
        })
    }

    pub fn record_change(
        &self,
        file_path: String,
        diff: FileDiff,
        author: Option<String>,
        message: Option<String>,
    ) -> Result<()> {
        let record = ChangeRecord {
            id: format!("change_{}", Self::current_timestamp()),
            file_path: file_path.clone(),
            diff,
            timestamp: Self::current_timestamp(),
            author,
            message,
        };

        let mut changes = self.changes.lock().unwrap();
        changes.push(record.clone());

        let mut file_history = self.file_history.lock().unwrap();
        file_history
            .entry(file_path)
            .or_insert_with(Vec::new)
            .push(record);

        Ok(())
    }

    pub fn get_file_history(&self, file_path: &str) -> Vec<ChangeRecord> {
        let file_history = self.file_history.lock().unwrap();
        file_history
            .get(file_path)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    pub fn get_all_changes(&self) -> Vec<ChangeRecord> {
        let changes = self.changes.lock().unwrap();
        changes.clone()
    }

    pub fn rollback_change(&self, change_id: &str) -> Result<String> {
        let changes = self.changes.lock().unwrap();
        if let Some(record) = changes.iter().find(|c| c.id == change_id) {
            Ok(record.diff.old_content.clone())
        } else {
            Err("Change not found".into())
        }
    }

    pub fn get_stats(&self) -> DiffStats {
        let changes = self.changes.lock().unwrap();
        let file_history = self.file_history.lock().unwrap();

        let total_changes = changes.len();
        let total_additions: u32 = changes.iter().map(|c| c.diff.additions).sum();
        let total_deletions: u32 = changes.iter().map(|c| c.diff.deletions).sum();
        let files_changed = file_history.len();
        let last_change = changes
            .last()
            .map(|c| c.timestamp)
            .unwrap_or_else(Self::current_timestamp);

        DiffStats {
            total_changes,
            total_additions,
            total_deletions,
            files_changed,
            last_change,
        }
    }

    pub fn clear_history(&self) -> Result<()> {
        let mut changes = self.changes.lock().unwrap();
        changes.clear();

        let mut file_history = self.file_history.lock().unwrap();
        file_history.clear();

        Ok(())
    }
}

#[tauri::command]
pub async fn diff_generate(
    file_path: String,
    old_content: String,
    new_content: String,
) -> Result<FileDiff> {
    eprintln!("Generating diff for: {}", file_path);
    DiffService::generate_diff(file_path, old_content, new_content)
}

#[tauri::command]
pub async fn diff_record_change(
    file_path: String,
    _diff: FileDiff,
    _author: Option<String>,
    _message: Option<String>,
) -> Result<()> {
    eprintln!("Recording change for: {}", file_path);
    Ok(())
}

#[tauri::command]
pub async fn diff_get_file_history(file_path: String) -> Result<Vec<ChangeRecord>> {
    eprintln!("Getting history for: {}", file_path);
    Ok(vec![])
}

#[tauri::command]
pub async fn diff_get_all_changes() -> Result<Vec<ChangeRecord>> {
    eprintln!("Getting all changes");
    Ok(vec![])
}

#[tauri::command]
pub async fn diff_rollback_change(change_id: String) -> Result<String> {
    eprintln!("Rolling back change: {}", change_id);
    Ok(String::new())
}

#[tauri::command]
pub async fn diff_get_stats() -> Result<DiffStats> {
    eprintln!("Getting diff statistics");
    Ok(DiffStats {
        total_changes: 0,
        total_additions: 0,
        total_deletions: 0,
        files_changed: 0,
        last_change: 0,
    })
}

#[tauri::command]
pub async fn diff_clear_history() -> Result<()> {
    eprintln!("Clearing diff history");
    Ok(())
}
