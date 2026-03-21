use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use regex::Regex;

use crate::state::AppState;
use crate::error::Result;
use crate::utils;

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct FuzzyResult {
    pub path: String,
    pub score: u32,
}

#[tauri::command]
pub async fn search_files(
    pattern: String,
    include_glob: Option<String>,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<SearchResult>> {
    let app_state = state.read();
    let workspace = app_state.get_workspace()
        .ok_or("No workspace set")?;
    
    let regex = Regex::new(&pattern)
        .map_err(|e| format!("Invalid regex: {}", e))?;
    
    let mut results = Vec::new();
    const MAX_RESULTS: usize = 50;
    
    for entry in walkdir::WalkDir::new(workspace)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if results.len() >= MAX_RESULTS {
            break;
        }
        
        let path = entry.path();
        
        if utils::should_skip_file(path) {
            continue;
        }
        
        // Apply include filter if specified
        if let Some(ref glob) = include_glob {
            if let Some(ext) = utils::get_file_extension(path) {
                let filter_ext = glob.replace("*.", "").to_lowercase();
                if ext != filter_ext {
                    continue;
                }
            }
        }
        
        if !path.is_file() {
            continue;
        }
        
        // Skip large files
        if let Ok(metadata) = path.metadata() {
            if metadata.len() > 100_000 {
                continue;
            }
        }
        
        // Read and search file
        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    results.push(SearchResult {
                        file: path.to_string_lossy().to_string(),
                        line: line_num + 1,
                        content: line.trim().to_string(),
                    });
                    
                    if results.len() >= MAX_RESULTS {
                        break;
                    }
                }
            }
        }
    }
    
    Ok(results)
}

#[tauri::command]
pub async fn fuzzy_find_file(
    query: String,
    max_results: Option<usize>,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Vec<FuzzyResult>> {
    let app_state = state.read();
    let workspace = app_state.get_workspace()
        .ok_or("No workspace set")?;
    
    let max = max_results.unwrap_or(10);
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();
    
    for entry in walkdir::WalkDir::new(workspace)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if results.len() >= (max * 2) {
            break;
        }
        
        let path = entry.path();
        
        if utils::should_skip_file(path) {
            continue;
        }
        
        if !path.is_file() {
            continue;
        }
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let rel_path = path.to_string_lossy().to_lowercase();
        
        // Calculate match score
        let score = if file_name == query_lower {
            100
        } else if file_name.starts_with(&query_lower) {
            80
        } else if file_name.contains(&query_lower) {
            50
        } else if rel_path.contains(&query_lower) {
            30
        } else {
            0
        };
        
        if score > 0 {
            results.push(FuzzyResult {
                path: path.to_string_lossy().to_string(),
                score,
            });
        }
    }
    
    // Sort by score descending
    results.sort_by(|a, b| b.score.cmp(&a.score));
    
    Ok(results.into_iter().take(max).collect())
}
