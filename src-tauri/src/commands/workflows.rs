use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::error::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillSummary {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub async fn list_workflows(workspace_path: String) -> Result<Vec<WorkflowSummary>> {
    let workflows_dir = Path::new(&workspace_path).join(".whizcode").join("workflows");
    let mut summaries = Vec::new();
    
    if workflows_dir.exists() {
        if let Ok(entries) = fs::read_dir(workflows_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                    let name = entry.file_name().to_string_lossy().replace(".md", "");
                    // Basic description extraction (first line or so)
                    let description = if let Ok(content) = fs::read_to_string(entry.path()) {
                        content.lines().next().unwrap_or("").to_string()
                    } else {
                        "".to_string()
                    };
                    
                    summaries.push(WorkflowSummary {
                        name,
                        description,
                        path: entry.path().to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    
    Ok(summaries)
}

#[tauri::command]
pub async fn list_skills(workspace_path: String) -> Result<Vec<SkillSummary>> {
    let skills_dir = Path::new(&workspace_path).join(".whizcode").join("skills");
    let mut summaries = Vec::new();
    
    if skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let skill_md = entry.path().join("SKILL.md");
                    if skill_md.exists() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        summaries.push(SkillSummary {
                            name,
                            path: skill_md.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(summaries)
}

pub fn get_workflows_context(workspace_path: &Path) -> String {
    let mut context = String::new();
    let workflows_dir = workspace_path.join(".whizcode").join("workflows");
    let skills_dir = workspace_path.join(".whizcode").join("skills");
    
    if workflows_dir.exists() {
        if let Ok(entries) = fs::read_dir(workflows_dir) {
            let mut wfs = Vec::new();
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                    wfs.push(entry.file_name().to_string_lossy().replace(".md", ""));
                }
            }
            if !wfs.is_empty() {
                context.push_str(&format!("\nAvailable Workflows (use read_file to access): {}\n", wfs.join(", ")));
            }
        }
    }
    
    if skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(skills_dir) {
            let mut sks = Vec::new();
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.path().join("SKILL.md").exists() {
                    sks.push(entry.file_name().to_string_lossy().to_string());
                }
            }
            if !sks.is_empty() {
                context.push_str(&format!("\nAvailable Modular Skills (use read_file to access): {}\n", sks.join(", ")));
            }
        }
    }
    
    context
}
