use crate::error::Result;

#[tauri::command]
pub async fn mcp_get_marketplace() -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
