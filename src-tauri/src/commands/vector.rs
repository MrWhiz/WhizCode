use crate::error::Result;

#[tauri::command]
pub async fn vector_get_index_stats() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
