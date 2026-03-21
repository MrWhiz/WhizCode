use crate::error::Result;

#[tauri::command]
pub async fn error_recovery_get_statistics() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
