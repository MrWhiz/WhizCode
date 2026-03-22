use crate::error::Result;

#[tauri::command]
pub async fn specs_list() -> Result<Vec<serde_json::Value>> {
    // Return empty list - specs are loaded from .whizcode/specs/ directory by frontend
    Ok(vec![])
}

#[tauri::command]
pub async fn specs_get(_slug: String) -> Result<serde_json::Value> {
    // Specs are loaded from filesystem by frontend
    Ok(serde_json::json!({
        "id": _slug,
        "name": "",
        "description": "",
        "tasks": [],
        "status": "draft"
    }))
}
