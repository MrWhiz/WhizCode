//! Agent Integration for Skills
//!
//! This module handles the integration of skills into the agent execution pipeline.
//! It provides functions to select skills for tasks, format skills for prompts,
//! and manage skill invocation during agent execution.

use super::models::{SkillSelectionResult, SkillContext, SelectedSkill};
use super::manager::SkillsManager;
use std::path::PathBuf;

/// Selects skills for a given task using the SkillsManager
///
/// This function analyzes the task and workspace context to determine
/// which skills are most relevant for execution.
///
/// # Arguments
///
/// * `task` - The task description
/// * `workspace_path` - Optional path to the workspace
/// * `project_type` - Type of project (e.g., "typescript", "python")
/// * `files` - List of relevant files
/// * `manager` - Reference to the SkillsManager
///
/// # Returns
///
/// A SkillSelectionResult containing selected skills and any conflicts resolved
pub async fn select_skills_for_task(
    task: &str,
    workspace_path: Option<String>,
    project_type: String,
    files: Vec<String>,
    manager: &SkillsManager,
) -> Result<SkillSelectionResult, String> {
    tracing::info!("Selecting skills for task: {}", task);

    // Get all available skills
    let available_skills = manager.get_all_skills();

    if available_skills.is_empty() {
        tracing::warn!("No skills available for selection");
        return Ok(SkillSelectionResult::new());
    }

    // Create skill context
    let context = SkillContext::new(
        workspace_path.map(PathBuf::from).unwrap_or_default(),
        task.to_string(),
        project_type,
        files,
    );

    // Select skills using the manager's selector
    let result = manager.select_skills(task, &context)
        .await
        .map_err(|e| format!("Failed to select skills: {}", e))?;

    tracing::info!(
        "Selected {} skills for task, resolved {} conflicts",
        result.selected_skills.len(),
        result.conflicts_resolved.len()
    );

    Ok(result)
}

/// Formats selected skills into a context string for the agent prompt
///
/// Creates a human-readable description of selected skills that can be
/// included in the agent's system prompt to inform it about available capabilities.
///
/// # Arguments
///
/// * `selected_skills` - List of selected skills
///
/// # Returns
///
/// A formatted string describing the selected skills
pub fn format_skills_for_prompt(selected_skills: &[SelectedSkill]) -> String {
    if selected_skills.is_empty() {
        return String::new();
    }

    let mut output = String::from("\n## Available Skills\n\n");
    output.push_str("The following specialized skills are available for this task:\n\n");

    for (idx, skill) in selected_skills.iter().enumerate() {
        output.push_str(&format!(
            "{}. **{}** (Confidence: {:.0}%)\n",
            idx + 1,
            skill.name,
            skill.confidence * 100.0
        ));

        if !skill.capabilities.is_empty() {
            output.push_str("   Capabilities: ");
            output.push_str(&skill.capabilities.join(", "));
            output.push('\n');
        }

        output.push('\n');
    }

    output
}

/// Creates a system prompt addition for skills
///
/// Generates a system prompt section that instructs the agent to use
/// the available skills when appropriate.
///
/// # Arguments
///
/// * `selected_skills` - List of selected skills
///
/// # Returns
///
/// A system prompt addition string
pub fn create_skills_system_prompt(selected_skills: &[SelectedSkill]) -> String {
    if selected_skills.is_empty() {
        return String::new();
    }

    let mut prompt = String::from(
        "\n## Skill Usage Instructions\n\n\
         You have access to specialized skills that can enhance your capabilities. \
         When appropriate for the task, consider using these skills:\n\n"
    );

    for skill in selected_skills {
        prompt.push_str(&format!(
            "- **{}**: {}\n",
            skill.name,
            skill.capabilities.join(", ")
        ));
    }

    prompt.push_str(
        "\nWhen using skills, clearly indicate which skill you're invoking and why it's relevant to the task.\n"
    );

    prompt
}

/// Formats skills for display in the chat UI
///
/// Creates a structured representation of selected skills for display
/// in the chat interface, showing confidence scores and capabilities.
///
/// # Arguments
///
/// * `selected_skills` - List of selected skills
///
/// # Returns
///
/// A JSON value representing the skills for UI display
pub fn format_skills_for_ui(selected_skills: &[SelectedSkill]) -> serde_json::Value {
    let skills_data: Vec<serde_json::Value> = selected_skills
        .iter()
        .map(|skill| {
            serde_json::json!({
                "name": skill.name,
                "confidence": skill.confidence,
                "capabilities": skill.capabilities,
            })
        })
        .collect();

    serde_json::json!({
        "count": skills_data.len(),
        "skills": skills_data,
    })
}

/// Extracts project type from workspace context
///
/// Analyzes the workspace to determine the project type
/// (e.g., "typescript", "python", "rust")
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace
/// * `files` - List of files in the workspace
///
/// # Returns
///
/// A string representing the detected project type
pub fn detect_project_type(_workspace_path: Option<&str>, files: &[String]) -> String {
    // Check for common project files
    for file in files {
        if file.contains("package.json") {
            return "typescript".to_string();
        }
        if file.contains("Cargo.toml") {
            return "rust".to_string();
        }
        if file.contains("requirements.txt") || file.contains("setup.py") {
            return "python".to_string();
        }
        if file.contains("go.mod") {
            return "go".to_string();
        }
        if file.contains("pom.xml") || file.contains("build.gradle") {
            return "java".to_string();
        }
    }

    // Check file extensions
    for file in files {
        if file.ends_with(".ts") || file.ends_with(".tsx") {
            return "typescript".to_string();
        }
        if file.ends_with(".rs") {
            return "rust".to_string();
        }
        if file.ends_with(".py") {
            return "python".to_string();
        }
        if file.ends_with(".go") {
            return "go".to_string();
        }
        if file.ends_with(".java") {
            return "java".to_string();
        }
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_skills_for_prompt_empty() {
        let skills = vec![];
        let result = format_skills_for_prompt(&skills);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_skills_for_prompt_with_skills() {
        let skills = vec![
            SelectedSkill::new(
                "code-analyzer",
                0.95,
                vec!["code-analysis".to_string(), "performance-check".to_string()],
                SkillContext::default(),
            ),
            SelectedSkill::new(
                "test-runner",
                0.85,
                vec!["testing".to_string()],
                SkillContext::default(),
            ),
        ];

        let result = format_skills_for_prompt(&skills);
        assert!(result.contains("code-analyzer"));
        assert!(result.contains("test-runner"));
        assert!(result.contains("95%"));
        assert!(result.contains("85%"));
    }

    #[test]
    fn test_create_skills_system_prompt_empty() {
        let skills = vec![];
        let result = create_skills_system_prompt(&skills);
        assert_eq!(result, "");
    }

    #[test]
    fn test_create_skills_system_prompt_with_skills() {
        let skills = vec![
            SelectedSkill::new(
                "code-analyzer",
                0.95,
                vec!["code-analysis".to_string()],
                SkillContext::default(),
            ),
        ];

        let result = create_skills_system_prompt(&skills);
        assert!(result.contains("Skill Usage Instructions"));
        assert!(result.contains("code-analyzer"));
    }

    #[test]
    fn test_format_skills_for_ui() {
        let skills = vec![
            SelectedSkill::new(
                "code-analyzer",
                0.95,
                vec!["code-analysis".to_string()],
                SkillContext::default(),
            ),
        ];

        let result = format_skills_for_ui(&skills);
        assert_eq!(result["count"], 1);
        assert_eq!(result["skills"][0]["name"], "code-analyzer");
        assert_eq!(result["skills"][0]["confidence"], 0.95);
    }

    #[test]
    fn test_detect_project_type_typescript() {
        let files = vec!["package.json".to_string(), "src/main.ts".to_string()];
        let project_type = detect_project_type(None, &files);
        assert_eq!(project_type, "typescript");
    }

    #[test]
    fn test_detect_project_type_rust() {
        let files = vec!["Cargo.toml".to_string(), "src/main.rs".to_string()];
        let project_type = detect_project_type(None, &files);
        assert_eq!(project_type, "rust");
    }

    #[test]
    fn test_detect_project_type_python() {
        let files = vec!["requirements.txt".to_string(), "main.py".to_string()];
        let project_type = detect_project_type(None, &files);
        assert_eq!(project_type, "python");
    }

    #[test]
    fn test_detect_project_type_by_extension() {
        let files = vec!["main.rs".to_string(), "lib.rs".to_string()];
        let project_type = detect_project_type(None, &files);
        assert_eq!(project_type, "rust");
    }

    #[test]
    fn test_detect_project_type_unknown() {
        let files = vec!["README.md".to_string()];
        let project_type = detect_project_type(None, &files);
        assert_eq!(project_type, "unknown");
    }
}
