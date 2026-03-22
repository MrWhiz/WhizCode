use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::error::Result;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeItem {
    pub id: String,
    pub topic: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DistillationRequest {
    pub messages: Vec<serde_json::Value>,
    pub workspace_path: String,
}

#[tauri::command]
pub async fn distill_session(messages: Vec<serde_json::Value>, workspace_path: String) -> Result<Vec<KnowledgeItem>> {
    let _prompt = format!(
        "Analyze the following conversation and extract 1-3 critical 'Knowledge Items' (KIs).
        A KI is an architectural decision, a learned codebase rule, a resolved bug, or structural context.
        Skip temporary logs. Provide response as JSON array of {{topic, content}}."
    );
    
    // In a real implementation, we would call an LLM here.
    // For now, we simulate distillation or prepare the structure.
    // We'll use the workspace_path to store KIs.
    
    let ki_dir = Path::new(&workspace_path).join(".whizcode").join("knowledge");
    if !ki_dir.exists() {
        fs::create_dir_all(&ki_dir)?;
    }
    
    // We'll iterate the request and save them as KIs
    let mut extracted = Vec::new();
    for _msg in &messages {
        // Simple logic for now: only mock or manually provided ones in this version
    }
    
    // Just save a sample for now if some were provided as simulated input
    let sample = KnowledgeItem {
        id: uuid::Uuid::new_v4().to_string(),
        topic: "Project Discovery".to_string(),
        content: "This project uses Tauri 2.0 and React with a custom theme. The backend manages all storage in .whizcode/".to_string(),
        timestamp: Utc::now().timestamp(),
    };
    
    let path = ki_dir.join(format!("{}.json", sample.id));
    fs::write(path, serde_json::to_string(&sample)?)?;
    extracted.push(sample);
    
    Ok(extracted)
}

pub fn load_relevant_knowledge(workspace_path: &Path) -> Result<String> {
    let ki_dir = workspace_path.join(".whizcode").join("knowledge");
    if !ki_dir.exists() {
        return Ok(String::new());
    }
    
    let mut lore = String::from("\n<knowledge_items>\nRelevant discoveries from past sessions:\n");
    let mut found = false;
    
    if let Ok(entries) = fs::read_dir(ki_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(ki) = serde_json::from_str::<KnowledgeItem>(&content) {
                        lore.push_str(&format!("### {}\n{}\n---\n", ki.topic, ki.content));
                        found = true;
                    }
                }
            }
        }
    }
    
    if found {
        lore.push_str("</knowledge_items>\n");
        Ok(lore)
    } else {
        Ok(String::new())
    }
}
