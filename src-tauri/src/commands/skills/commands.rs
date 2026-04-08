//! Tauri IPC Commands for Skills Management
//!
//! Exposes skills functionality to the frontend via Tauri's command system.
//! Follows Claude Code's plugin architecture for skill invocation.

use super::manager::SkillsManager;
use super::models::{Skill, SkillContext, SkillSelectionResult};
use std::path::PathBuf;
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};

/// Simple skill response for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResponse {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
}

impl From<Skill> for SkillResponse {
    fn from(skill: Skill) -> Self {
        Self {
            name: skill.manifest.name,
            description: skill.manifest.description,
            version: skill.manifest.version,
            author: skill.manifest.author,
            enabled: skill.enabled,
        }
    }
}

/// Global SkillsManager instance
static SKILLS_MANAGER: OnceLock<SkillsManager> = OnceLock::new();

/// Initializes the global SkillsManager
///
/// Should be called once during application startup
pub async fn init_skills_manager() -> Result<(), String> {
    let manager = SkillsManager::new().await?;
    SKILLS_MANAGER.set(manager).map_err(|_| {
        "SkillsManager already initialized".to_string()
    })?;
    
    // Auto-discover skills on startup
    if let Ok(manager) = get_skills_manager() {
        match manager.discover_skills().await {
            Ok(skills) => {
                eprintln!("[Skills] Auto-discovered {} skills on startup", skills.len());
            }
            Err(e) => {
                eprintln!("[Skills] Failed to auto-discover skills: {}", e);
                // Don't fail initialization if discovery fails
            }
        }
    }
    
    Ok(())
}

/// Gets a reference to the global SkillsManager
pub fn get_skills_manager() -> Result<&'static SkillsManager, String> {
    SKILLS_MANAGER.get().ok_or_else(|| {
        "SkillsManager not initialized".to_string()
    })
}

/// Tauri command: Get all discovered skills
///
/// # Returns
///
/// `Vec<SkillResponse>` with all skills in the system
#[tauri::command]
pub fn get_skills() -> Result<Vec<SkillResponse>, String> {
    let manager = get_skills_manager()?;
    let skills = manager.get_all_skills();
    Ok(skills.into_iter().map(SkillResponse::from).collect())
}

/// Tauri command: Discover skills from repository
///
/// Triggers a fresh discovery of skills from the configured repository.
/// This may take up to 500ms for typical repositories.
///
/// # Returns
///
/// `Vec<SkillResponse>` with newly discovered skills
#[tauri::command]
pub async fn discover_skills() -> Result<Vec<SkillResponse>, String> {
    let manager = get_skills_manager()?;
    let skills = manager.discover_skills().await?;
    Ok(skills.into_iter().map(SkillResponse::from).collect())
}

/// Tauri command: Refresh skills from repository
///
/// Clears the in-memory cache and re-discovers skills.
///
/// # Returns
///
/// `Vec<SkillResponse>` with refreshed skills
#[tauri::command]
pub async fn refresh_skills() -> Result<Vec<SkillResponse>, String> {
    let manager = get_skills_manager()?;
    let skills = manager.refresh_skills().await?;
    Ok(skills.into_iter().map(SkillResponse::from).collect())
}

/// Tauri command: Select skills for a query
///
/// Uses intelligent relevance scoring to select the most appropriate skills
/// for the given query and context.
///
/// # Arguments
///
/// * `query` - User's query or request
/// * `workspace_path` - Path to the current workspace
/// * `project_type` - Type of project (e.g., "typescript", "python")
/// * `files` - List of relevant files
///
/// # Returns
///
/// `SkillSelectionResult` with selected skills and conflict resolutions
#[tauri::command]
pub async fn select_skills(
    query: String,
    workspace_path: String,
    project_type: String,
    files: Vec<String>,
) -> Result<SkillSelectionResult, String> {
    let manager = get_skills_manager()?;

    let context = SkillContext::new(
        PathBuf::from(workspace_path),
        query.clone(),
        project_type,
        files,
    );

    manager.select_skills(&query, &context).await
}

/// Tauri command: Get a specific skill by name
///
/// # Arguments
///
/// * `name` - Skill name
///
/// # Returns
///
/// `Some(Skill)` if found, `None` otherwise
#[tauri::command]
pub fn get_skill(name: String) -> Result<Option<Skill>, String> {
    let manager = get_skills_manager()?;
    Ok(manager.get_skill(&name))
}

/// Tauri command: Enable a skill
///
/// # Arguments
///
/// * `name` - Skill name
///
/// # Returns
///
/// `Ok(())` if successful
#[tauri::command]
pub fn enable_skill(name: String) -> Result<(), String> {
    let manager = get_skills_manager()?;
    manager.enable_skill(&name)
}

/// Tauri command: Disable a skill
///
/// # Arguments
///
/// * `name` - Skill name
///
/// # Returns
///
/// `Ok(())` if successful
#[tauri::command]
pub fn disable_skill(name: String) -> Result<(), String> {
    let manager = get_skills_manager()?;
    manager.disable_skill(&name)
}

/// Tauri command: Set repository URL
///
/// Updates the skills repository URL and triggers a refresh.
///
/// # Arguments
///
/// * `url` - New repository URL
///
/// # Returns
///
/// `Ok(())` if successful
#[tauri::command]
pub async fn set_repository_url(url: String) -> Result<(), String> {
    let manager = get_skills_manager()?;
    manager.set_repository_url(url).await?;
    // Refresh skills after URL change
    manager.refresh_skills().await?;
    Ok(())
}

/// Tauri command: Get current configuration
///
/// # Returns
///
/// `SkillsConfig` with current settings
#[tauri::command]
pub async fn get_skills_config() -> Result<super::models::SkillsConfig, String> {
    let manager = get_skills_manager()?;
    Ok(manager.get_config().await)
}

/// Tauri command: Get skill count
///
/// # Returns
///
/// Number of skills in the system
#[tauri::command]
pub fn get_skill_count() -> Result<usize, String> {
    let manager = get_skills_manager()?;
    Ok(manager.skill_count())
}

/// Tauri command: Analyze workspace and select relevant skills
///
/// Analyzes the current workspace to determine which skills are most relevant
/// and returns them with confidence scores.
///
/// # Arguments
///
/// * `workspace_path` - Path to the current workspace
/// * `project_type` - Type of project (e.g., "typescript", "python", "rust")
/// * `files` - List of relevant files in the workspace
///
/// # Returns
///
/// `Vec<SkillResponse>` with selected skills ranked by relevance
#[tauri::command]
pub async fn analyze_workspace_skills(
    workspace_path: String,
    project_type: String,
    files: Vec<String>,
) -> Result<Vec<SkillResponse>, String> {
    let manager = get_skills_manager()?;

    // Create a generic query based on workspace context
    let query = format!(
        "Analyze and improve {} project with {} files",
        project_type,
        files.len()
    );

    let context = super::models::SkillContext::new(
        std::path::PathBuf::from(workspace_path),
        query,
        project_type,
        files,
    );

    // Select skills for this workspace
    let result = manager.select_skills(&context.query, &context).await?;

    // Convert to SkillResponse format
    let skill_responses: Vec<SkillResponse> = result
        .selected_skills
        .into_iter()
        .map(|selected| {
            // Find the full skill to get all details
            if let Some(skill) = manager.get_skill(&selected.name) {
                SkillResponse {
                    name: skill.manifest.name,
                    description: skill.manifest.description,
                    version: skill.manifest.version,
                    author: skill.manifest.author,
                    enabled: skill.enabled,
                }
            } else {
                SkillResponse {
                    name: selected.name.clone(),
                    description: format!("Confidence: {:.0}%", selected.confidence * 100.0),
                    version: "1.0.0".to_string(),
                    author: "Claude Skills".to_string(),
                    enabled: true,
                }
            }
        })
        .collect();

    Ok(skill_responses)
}

/// Tauri command: Select skills for a specific task
///
/// Analyzes a task and selects the most relevant skills for execution.
/// This is used by the agent to determine which skills to use for a query.
///
/// # Arguments
///
/// * `task` - The user's task or query
/// * `workspace_path` - Path to the current workspace (optional)
/// * `project_type` - Type of project (e.g., "typescript", "python")
/// * `files` - List of relevant files in the workspace
///
/// # Returns
///
/// `SkillSelectionResult` with selected skills and conflict resolutions
#[tauri::command]
pub async fn select_skills_for_task(
    task: String,
    workspace_path: Option<String>,
    project_type: String,
    files: Vec<String>,
) -> Result<super::models::SkillSelectionResult, String> {
    let manager = get_skills_manager()?;

    super::agent_integration::select_skills_for_task(
        &task,
        workspace_path,
        project_type,
        files,
        manager,
    )
    .await
}

/// Tauri command: Get skills system prompt addition
///
/// Creates a system prompt section that instructs the agent to use
/// the available skills when appropriate.
///
/// # Arguments
///
/// * `task` - The user's task or query
/// * `workspace_path` - Path to the current workspace (optional)
/// * `project_type` - Type of project
/// * `files` - List of relevant files
///
/// # Returns
///
/// A system prompt addition string
#[tauri::command]
pub async fn get_skills_system_prompt(
    task: String,
    workspace_path: Option<String>,
    project_type: String,
    files: Vec<String>,
) -> Result<String, String> {
    let manager = get_skills_manager()?;

    let result = super::agent_integration::select_skills_for_task(
        &task,
        workspace_path,
        project_type,
        files,
        manager,
    )
    .await?;

    Ok(super::agent_integration::create_skills_system_prompt(&result.selected_skills))
}

/// Tauri command: Get skills for UI display
///
/// Formats selected skills for display in the chat interface.
///
/// # Arguments
///
/// * `task` - The user's task or query
/// * `workspace_path` - Path to the current workspace (optional)
/// * `project_type` - Type of project
/// * `files` - List of relevant files
///
/// # Returns
///
/// A JSON value with formatted skills for UI display
#[tauri::command]
pub async fn get_skills_for_ui(
    task: String,
    workspace_path: Option<String>,
    project_type: String,
    files: Vec<String>,
) -> Result<serde_json::Value, String> {
    let manager = get_skills_manager()?;

    let result = super::agent_integration::select_skills_for_task(
        &task,
        workspace_path,
        project_type,
        files,
        manager,
    )
    .await?;

    Ok(super::agent_integration::format_skills_for_ui(&result.selected_skills))
}

/// Tauri command: Get formatted skills context for agent prompt
///
/// Formats selected skills into a context string suitable for inclusion
/// in the agent's system prompt.
///
/// # Arguments
///
/// * `task` - The user's task or query
/// * `workspace_path` - Path to the current workspace (optional)
/// * `project_type` - Type of project
/// * `files` - List of relevant files
///
/// # Returns
///
/// A formatted string describing the selected skills
#[tauri::command]
pub async fn get_skills_context_for_prompt(
    task: String,
    workspace_path: Option<String>,
    project_type: String,
    files: Vec<String>,
) -> Result<String, String> {
    let manager = get_skills_manager()?;

    let result = super::agent_integration::select_skills_for_task(
        &task,
        workspace_path,
        project_type,
        files,
        manager,
    )
    .await?;

    Ok(super::agent_integration::format_skills_for_prompt(&result.selected_skills))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_skills_manager() {
        let result = init_skills_manager().await;
        // May fail if already initialized, but should not panic
        let _ = result;
    }

    #[test]
    fn test_get_skills_manager_not_initialized() {
        // This test assumes manager might not be initialized
        // In practice, it should be initialized before tests run
        let _ = get_skills_manager();
    }
}
