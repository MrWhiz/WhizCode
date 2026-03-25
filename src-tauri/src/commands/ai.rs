use crate::commands::code_intelligence::CodeIntelligence;
use crate::commands::context_memory::ContextMemory;
use crate::error::Result;
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub async fn ai_get_learning_insights() -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "insights": [],
        "total_sessions": 0,
        "patterns_detected": 0
    }))
}

#[tauri::command]
pub async fn ai_get_learning_metrics() -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "accuracy": 0.0,
        "improvement_rate": 0.0,
        "total_interactions": 0
    }))
}

#[tauri::command]
pub async fn ai_get_code_metrics(
    workspace_path: String,
    intel_state: State<'_, Arc<Mutex<CodeIntelligence>>>,
) -> Result<serde_json::Value> {
    let intel = intel_state.lock().unwrap();
    let context = intel.analyze_workspace_if_stale(workspace_path)?;
    Ok(serde_json::json!({
        "files_analyzed": context.metrics.total_files,
        "complexity_score": context.metrics.average_complexity,
        "maintainability_index": context.metrics.maintainability_index,
        "symbols": context.metrics.total_symbols,
        "patterns_detected": context.patterns.len(),
        "last_analyzed": context.last_analyzed
    }))
}

#[tauri::command]
pub async fn ai_get_context_memory_stats(
    memory_state: State<'_, Arc<Mutex<ContextMemory>>>,
) -> Result<serde_json::Value> {
    let memory = memory_state.lock().unwrap();
    let stats = memory.get_statistics();
    Ok(serde_json::json!({
        "memory_used": stats.total_patterns + stats.total_preferences + stats.total_error_patterns + stats.total_strategies + stats.total_projects,
        "items_stored": {
            "patterns": stats.total_patterns,
            "preferences": stats.total_preferences,
            "error_patterns": stats.total_error_patterns,
            "strategies": stats.total_strategies,
            "projects": stats.total_projects
        },
        "retention_rate": 1.0,
        "most_used_language": stats.most_used_language,
        "most_common_error": stats.most_common_error,
        "best_strategy": stats.best_strategy
    }))
}
