use crate::error::Result;

#[tauri::command]
pub async fn ai_get_learning_insights() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}

#[tauri::command]
pub async fn ai_get_learning_metrics() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}

#[tauri::command]
pub async fn ai_get_code_metrics(_workspace_path: String) -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}

#[tauri::command]
pub async fn ai_get_context_memory_stats() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
