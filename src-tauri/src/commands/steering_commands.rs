use crate::commands::steering_files::SteeringFileManager;
use serde_json::json;

#[tauri::command]
pub fn load_steering_files(workspace_path: String) -> Result<serde_json::Value, String> {
    let steering = SteeringFileManager::load_steering_files(&workspace_path)?;
    Ok(json!(steering))
}

#[tauri::command]
pub fn validate_steering_files(workspace_path: String) -> Result<serde_json::Value, String> {
    let steering = SteeringFileManager::load_steering_files(&workspace_path)?;
    let errors = SteeringFileManager::validate_steering_files(&steering);

    Ok(json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "steering": steering
    }))
}

#[tauri::command]
pub fn create_default_steering_files(workspace_path: String) -> Result<String, String> {
    SteeringFileManager::create_default_steering_files(&workspace_path)?;
    Ok("Default steering files created successfully".to_string())
}

#[tauri::command]
pub fn get_steering_context(workspace_path: String) -> Result<String, String> {
    let steering = SteeringFileManager::load_steering_files(&workspace_path)?;
    Ok(SteeringFileManager::get_steering_context(&steering))
}
