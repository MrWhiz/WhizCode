use crate::error::Result;

#[tauri::command]
pub async fn specs_list() -> Result<Vec<serde_json::Value>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn specs_get(_slug: String) -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}
