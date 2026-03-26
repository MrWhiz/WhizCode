use crate::commands::planning::ExecutionPlan;
use crate::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecArtifact {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub plan: ExecutionPlan,
}

fn specs_dir(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join(".whizcode").join("specs")
}

fn spec_path(workspace_path: &str, slug: &str) -> PathBuf {
    specs_dir(workspace_path).join(format!("{}.json", slug))
}

pub fn save_spec_artifact(workspace_path: &str, plan: &ExecutionPlan, original_query: &str) -> Result<SpecArtifact> {
    let now = Utc::now().to_rfc3339();
    let artifact = SpecArtifact {
        id: plan.id.clone(),
        name: plan.objective.clone(),
        description: original_query.trim().to_string(),
        created_at: now.clone(),
        updated_at: now,
        status: "draft".to_string(),
        plan: plan.clone(),
    };

    let dir = specs_dir(workspace_path);
    fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(&artifact)?;
    fs::write(spec_path(workspace_path, &artifact.id), content)?;
    Ok(artifact)
}

#[tauri::command]
pub async fn specs_list(workspace_path: Option<String>) -> Result<Vec<serde_json::Value>> {
    let Some(workspace_path) = workspace_path else {
        return Ok(vec![]);
    };

    let dir = specs_dir(&workspace_path);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut specs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        if let Ok(spec) = serde_json::from_str::<SpecArtifact>(&content) {
            specs.push(serde_json::json!({
                "id": spec.id,
                "name": spec.name,
                "description": spec.description,
                "status": spec.status,
                "created_at": spec.created_at,
                "updated_at": spec.updated_at,
                "task_count": spec.plan.tasks.len(),
                "acceptance_criteria": spec.plan.acceptance_criteria,
            }));
        }
    }

    specs.sort_by(|a, b| {
        b.get("updated_at")
            .and_then(|v| v.as_str())
            .cmp(&a.get("updated_at").and_then(|v| v.as_str()))
    });
    Ok(specs)
}

#[tauri::command]
pub async fn specs_get(workspace_path: String, slug: String) -> Result<serde_json::Value> {
    let path = spec_path(&workspace_path, &slug);
    if !path.exists() {
        return Ok(serde_json::json!({
            "id": slug,
            "name": "",
            "description": "",
            "tasks": [],
            "status": "draft"
        }));
    }

    let content = fs::read_to_string(path)?;
    let spec: SpecArtifact = serde_json::from_str(&content)?;
    Ok(serde_json::to_value(spec)?)
}
