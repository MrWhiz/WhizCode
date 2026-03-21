use crate::error::Result;

#[tauri::command]
pub async fn diagnostics_check(
    _file_path: String,
    _workspace_path: String,
    _content: Option<String>,
) -> Result<Vec<serde_json::Value>> {
    Ok(vec![])
}
