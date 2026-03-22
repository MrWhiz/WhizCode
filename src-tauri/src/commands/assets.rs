use serde::{Deserialize, Serialize};
use crate::error::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageRequest {
    pub prompt: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationResult {
    pub url: String,
    pub asset_path: String,
}

#[tauri::command]
pub async fn generate_image(request: ImageRequest, workspace_path: String) -> Result<GenerationResult> {
    eprintln!("[AssetGen] Generating image for prompt: {}", request.prompt);
    
    // In a real implementation, call DALL-E/Midjourney here.
    // For now, we will generate a placeholder or a beautiful SVG based on the prompt.
    
    let assets_dir = Path::new(&workspace_path).join("assets").join("ai_generated");
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)?;
    }
    
    let file_id = uuid::Uuid::new_v4().to_string();
    let file_name = format!("{}.svg", file_id);
    let full_path = assets_dir.join(&file_name);
    
    // Create a high-quality SVG placeholder as a fallback.
    // Use r##" to avoid premature termination since SVG attributes often contain "#"
    let svg_content = format!(
        r##"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" style="stop-color:#313244;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#11111b;stop-opacity:1" />
                </linearGradient>
            </defs>
            <rect width="100%" height="100%" fill="url(#grad1)" rx="12" />
            <text x="50%" y="50%" font-family="sans-serif" font-size="24" fill="#cdd6f4" text-anchor="middle" dominant-baseline="middle">
                Asset: {}
            </text>
        </svg>"##, 
        request.width, request.height, request.prompt
    );
    
    fs::write(&full_path, svg_content)?;
    
    Ok(GenerationResult {
        url: format!("file:///{}", full_path.to_string_lossy()),
        asset_path: full_path.to_string_lossy().to_string(),
    })
}
