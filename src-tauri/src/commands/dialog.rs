use crate::error::Result;

#[tauri::command]
pub async fn dialog_open_folder() -> Result<Option<String>> {
    let folder = rfd::AsyncFileDialog::new()
        .pick_folder()
        .await;
    
    Ok(folder.map(|f| f.path().to_string_lossy().to_string()))
}
