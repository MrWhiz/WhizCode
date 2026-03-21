use crate::error::Result;

#[tauri::command]
pub async fn cache_get_stats() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
