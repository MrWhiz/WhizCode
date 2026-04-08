//! Data structures and types for the Skills system
//!
//! This module defines all core data structures used throughout the skills
//! management system, including skill metadata, selection results, and configuration.
//! All structures support JSON serialization/deserialization for IPC communication.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Represents a skill manifest with metadata about the skill.
///
/// A manifest is the primary metadata file for a skill, containing all information
/// needed to understand the skill's purpose, capabilities, and requirements.
///
/// # Fields
///
/// * `name` - Unique identifier for the skill (e.g., "code-analysis-skill")
/// * `version` - Semantic version of the skill (e.g., "1.0.0")
/// * `description` - Human-readable description of what the skill does
/// * `author` - Name or organization that created the skill
/// * `capabilities` - List of capabilities this skill provides (e.g., ["code-quality-analysis", "performance-analysis"])
/// * `requirements` - List of language/framework requirements (e.g., ["typescript", "javascript"])
/// * `dependencies` - External dependencies required by the skill
/// * `config_options` - Optional JSON configuration schema for skill customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique identifier for the skill
    pub name: String,
    /// Semantic version of the skill
    pub version: String,
    /// Human-readable description of the skill's purpose
    pub description: String,
    /// Author or organization that created the skill
    pub author: String,
    /// List of capabilities provided by this skill
    pub capabilities: Vec<String>,
    /// List of language/framework requirements
    pub requirements: Vec<String>,
    /// External dependencies required by the skill
    pub dependencies: Vec<Dependency>,
    /// Optional configuration schema for skill customization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_options: Option<serde_json::Value>,
}

impl SkillManifest {
    /// Validates that all required fields are present and non-empty.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the manifest is valid, `Err(String)` with a descriptive error message otherwise.
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Skill name cannot be empty".to_string());
        }
        if self.version.is_empty() {
            return Err("Skill version cannot be empty".to_string());
        }
        if self.description.is_empty() {
            return Err("Skill description cannot be empty".to_string());
        }
        if self.author.is_empty() {
            return Err("Skill author cannot be empty".to_string());
        }
        if self.capabilities.is_empty() {
            return Err("Skill must have at least one capability".to_string());
        }
        Ok(())
    }

    /// Checks if this manifest has a specific capability.
    #[allow(dead_code)]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Checks if this manifest requires a specific language/framework.
    #[allow(dead_code)]
    pub fn requires(&self, requirement: &str) -> bool {
        self.requirements.iter().any(|r| r == requirement)
    }

    /// Returns the number of dependencies.
    #[allow(dead_code)]
    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }
}

/// Represents a dependency required by a skill.
///
/// Dependencies can be external packages, system tools, or other skills.
///
/// # Fields
///
/// * `name` - Name of the dependency (e.g., "ast-parser", "typescript")
/// * `version` - Required version or version constraint (e.g., "^1.0.0")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Name of the dependency
    pub name: String,
    /// Required version or version constraint
    pub version: String,
}

impl Dependency {
    /// Creates a new dependency with the given name and version.
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Validates that the dependency has a non-empty name and version.
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Dependency name cannot be empty".to_string());
        }
        if self.version.is_empty() {
            return Err("Dependency version cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Status of a skill in the system.
///
/// Indicates whether a skill is available for use, unavailable due to unmet requirements,
/// or explicitly disabled by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum SkillStatus {
    /// Skill is available and can be used
    Available,
    /// Skill is unavailable with a reason (e.g., "Missing dependencies: typescript")
    Unavailable(String),
    /// Skill is explicitly disabled by the user
    Disabled,
}

impl SkillStatus {
    /// Returns true if the skill is available.
    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        matches!(self, SkillStatus::Available)
    }

    /// Returns true if the skill is disabled.
    #[allow(dead_code)]
    pub fn is_disabled(&self) -> bool {
        matches!(self, SkillStatus::Disabled)
    }

    /// Returns true if the skill is unavailable.
    #[allow(dead_code)]
    pub fn is_unavailable(&self) -> bool {
        matches!(self, SkillStatus::Unavailable(_))
    }

    /// Returns the reason if the skill is unavailable, otherwise None.
    #[allow(dead_code)]
    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            SkillStatus::Unavailable(reason) => Some(reason),
            _ => None,
        }
    }
}

/// Represents a complete skill with its manifest and metadata.
///
/// A Skill combines the manifest metadata with runtime information like
/// the skill's location, current status, and cache state.
///
/// # Fields
///
/// * `manifest` - The skill's metadata
/// * `path` - File system path where the skill is located
/// * `status` - Current status (Available, Unavailable, Disabled)
/// * `enabled` - Whether the user has enabled this skill
/// * `cached` - Whether the skill is loaded from cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// The skill's metadata
    pub manifest: SkillManifest,
    /// File system path where the skill is located
    pub path: PathBuf,
    /// Current status of the skill
    pub status: SkillStatus,
    /// Whether the user has enabled this skill
    pub enabled: bool,
    /// Whether the skill is loaded from cache
    pub cached: bool,
}

impl Skill {
    /// Creates a new skill with the given manifest and path.
    #[allow(dead_code)]
    pub fn new(manifest: SkillManifest, path: PathBuf) -> Self {
        Self {
            manifest,
            path,
            status: SkillStatus::Available,
            enabled: true,
            cached: false,
        }
    }

    /// Creates a skill from a manifest (for discovered skills)
    pub fn from_manifest(manifest: SkillManifest) -> Self {
        Self {
            manifest,
            path: PathBuf::new(),
            status: SkillStatus::Available,
            enabled: true,
            cached: false,
        }
    }

    /// Returns the skill's name.
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// Returns the skill's version.
    #[allow(dead_code)]
    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    /// Returns true if the skill is usable (available and enabled).
    #[allow(dead_code)]
    pub fn is_usable(&self) -> bool {
        self.enabled && self.status.is_available()
    }

    /// Returns true if the skill has a specific capability.
    #[allow(dead_code)]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.manifest.has_capability(capability)
    }

    /// Returns the number of capabilities this skill provides.
    #[allow(dead_code)]
    pub fn capability_count(&self) -> usize {
        self.manifest.capabilities.len()
    }

    /// Marks the skill as unavailable with a reason.
    #[allow(dead_code)]
    pub fn mark_unavailable(&mut self, reason: impl Into<String>) {
        self.status = SkillStatus::Unavailable(reason.into());
    }

    /// Marks the skill as available.
    #[allow(dead_code)]
    pub fn mark_available(&mut self) {
        self.status = SkillStatus::Available;
    }

    /// Marks the skill as disabled.
    #[allow(dead_code)]
    pub fn mark_disabled(&mut self) {
        self.status = SkillStatus::Disabled;
    }

    /// Enables the skill.
    #[allow(dead_code)]
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disables the skill.
    #[allow(dead_code)]
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Marks the skill as cached.
    #[allow(dead_code)]
    pub fn mark_cached(&mut self) {
        self.cached = true;
    }

    /// Marks the skill as not cached.
    #[allow(dead_code)]
    pub fn mark_not_cached(&mut self) {
        self.cached = false;
    }
}

/// Result of skill selection for a query.
///
/// Contains the skills selected for a query and any conflicts that were resolved
/// during the selection process.
///
/// # Fields
///
/// * `selected_skills` - List of skills selected for the query
/// * `conflicts_resolved` - List of conflicts that were detected and resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSelectionResult {
    /// List of skills selected for the query
    pub selected_skills: Vec<SelectedSkill>,
    /// List of conflicts that were detected and resolved
    pub conflicts_resolved: Vec<ConflictResolution>,
}

impl SkillSelectionResult {
    /// Creates a new empty selection result.
    pub fn new() -> Self {
        Self {
            selected_skills: Vec::new(),
            conflicts_resolved: Vec::new(),
        }
    }

    /// Returns the number of selected skills.
    #[allow(dead_code)]
    pub fn skill_count(&self) -> usize {
        self.selected_skills.len()
    }

    /// Returns the number of resolved conflicts.
    #[allow(dead_code)]
    pub fn conflict_count(&self) -> usize {
        self.conflicts_resolved.len()
    }

    /// Returns true if any skills were selected.
    #[allow(dead_code)]
    pub fn has_skills(&self) -> bool {
        !self.selected_skills.is_empty()
    }

    /// Returns true if any conflicts were resolved.
    #[allow(dead_code)]
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts_resolved.is_empty()
    }

    /// Adds a selected skill to the result.
    #[allow(dead_code)]
    pub fn add_skill(&mut self, skill: SelectedSkill) {
        self.selected_skills.push(skill);
    }

    /// Adds a conflict resolution to the result.
    #[allow(dead_code)]
    pub fn add_conflict(&mut self, conflict: ConflictResolution) {
        self.conflicts_resolved.push(conflict);
    }

    /// Returns the average confidence score of selected skills.
    #[allow(dead_code)]
    pub fn average_confidence(&self) -> f32 {
        if self.selected_skills.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.selected_skills.iter().map(|s| s.confidence).sum();
        sum / self.selected_skills.len() as f32
    }
}

/// A skill selected for a specific query with confidence score.
///
/// Represents a skill that has been selected for use in response to a user query,
/// along with a confidence score indicating how relevant the skill is.
///
/// # Fields
///
/// * `name` - Name of the selected skill
/// * `confidence` - Confidence score [0.0, 1.0] indicating relevance
/// * `capabilities` - Capabilities of this skill relevant to the query
/// * `context` - Context information for skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedSkill {
    /// Name of the selected skill
    pub name: String,
    /// Confidence score [0.0, 1.0] indicating relevance to the query
    pub confidence: f32,
    /// Capabilities of this skill relevant to the query
    pub capabilities: Vec<String>,
    /// Context information for skill execution
    pub context: SkillContext,
}

impl SelectedSkill {
    /// Creates a new selected skill.
    pub fn new(
        name: impl Into<String>,
        confidence: f32,
        capabilities: Vec<String>,
        context: SkillContext,
    ) -> Self {
        Self {
            name: name.into(),
            confidence: confidence.clamp(0.0, 1.0),
            capabilities,
            context,
        }
    }

    /// Returns true if the confidence score is above the given threshold.
    #[allow(dead_code)]
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }

    /// Returns the number of relevant capabilities.
    #[allow(dead_code)]
    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

/// Context information for skill selection and execution.
///
/// Provides information about the workspace and query that helps skills
/// understand the context in which they're being used.
///
/// # Fields
///
/// * `workspace_path` - Path to the current workspace
/// * `query` - The user's query or request
/// * `project_type` - Type of project (e.g., "typescript", "python", "rust")
/// * `files` - List of relevant files in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    /// Path to the current workspace
    pub workspace_path: PathBuf,
    /// The user's query or request
    pub query: String,
    /// Type of project (e.g., "typescript", "python", "rust")
    pub project_type: String,
    /// List of relevant files in the workspace
    pub files: Vec<String>,
}

impl SkillContext {
    /// Creates a new skill context.
    pub fn new(
        workspace_path: PathBuf,
        query: impl Into<String>,
        project_type: impl Into<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            workspace_path,
            query: query.into(),
            project_type: project_type.into(),
            files,
        }
    }

    /// Returns the number of files in the context.
    #[allow(dead_code)]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns true if the context has any files.
    #[allow(dead_code)]
    pub fn has_files(&self) -> bool {
        !self.files.is_empty()
    }

    /// Adds a file to the context.
    #[allow(dead_code)]
    pub fn add_file(&mut self, file: impl Into<String>) {
        self.files.push(file.into());
    }
}

/// Represents a conflict resolution between two skills.
///
/// When two skills have overlapping capabilities, a conflict is detected
/// and resolved by choosing one skill over the other based on confidence scores.
///
/// # Fields
///
/// * `skill_a` - Name of the first skill
/// * `skill_b` - Name of the second skill
/// * `resolution` - Description of how the conflict was resolved
/// * `winner` - Name of the skill that was chosen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Name of the first skill
    pub skill_a: String,
    /// Name of the second skill
    pub skill_b: String,
    /// Description of how the conflict was resolved
    pub resolution: String,
    /// Name of the skill that was chosen
    pub winner: String,
}

impl ConflictResolution {
    /// Creates a new conflict resolution.
    pub fn new(
        skill_a: impl Into<String>,
        skill_b: impl Into<String>,
        resolution: impl Into<String>,
        winner: impl Into<String>,
    ) -> Self {
        Self {
            skill_a: skill_a.into(),
            skill_b: skill_b.into(),
            resolution: resolution.into(),
            winner: winner.into(),
        }
    }

    /// Returns the loser of the conflict (the skill that was not chosen).
    pub fn loser(&self) -> &str {
        if self.winner == self.skill_a {
            &self.skill_b
        } else {
            &self.skill_a
        }
    }
}

/// Configuration for the Skills Manager.
///
/// Contains all configuration options for the skills system, including
/// repository URL, performance thresholds, and user preferences.
///
/// # Fields
///
/// * `repository_url` - URL of the skills repository
/// * `max_skills` - Maximum number of skills to select per query
/// * `confidence_threshold` - Minimum confidence score for skill selection
/// * `cache_ttl` - Time-to-live for cached skills
/// * `enabled_skills` - List of skills enabled by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// URL of the skills repository
    pub repository_url: String,
    /// Maximum number of skills to select per query
    pub max_skills: usize,
    /// Minimum confidence score [0.0, 1.0] for skill selection
    pub confidence_threshold: f32,
    /// Time-to-live for cached skills
    pub cache_ttl: Duration,
    /// List of skills enabled by the user
    pub enabled_skills: Vec<String>,
}

impl SkillsConfig {
    /// Creates a new configuration with custom values.
    #[allow(dead_code)]
    pub fn new(
        repository_url: impl Into<String>,
        max_skills: usize,
        confidence_threshold: f32,
        cache_ttl: Duration,
    ) -> Self {
        Self {
            repository_url: repository_url.into(),
            max_skills,
            confidence_threshold: confidence_threshold.clamp(0.0, 1.0),
            cache_ttl,
            enabled_skills: Vec::new(),
        }
    }

    /// Enables a skill by adding it to the enabled list.
    #[allow(dead_code)]
    pub fn enable_skill(&mut self, skill_name: impl Into<String>) {
        let name = skill_name.into();
        if !self.enabled_skills.contains(&name) {
            self.enabled_skills.push(name);
        }
    }

    /// Disables a skill by removing it from the enabled list.
    #[allow(dead_code)]
    pub fn disable_skill(&mut self, skill_name: &str) {
        self.enabled_skills.retain(|s| s != skill_name);
    }

    /// Returns true if a skill is enabled.
    #[allow(dead_code)]
    pub fn is_skill_enabled(&self, skill_name: &str) -> bool {
        self.enabled_skills.contains(&skill_name.to_string())
    }

    /// Returns the number of enabled skills.
    #[allow(dead_code)]
    pub fn enabled_skill_count(&self) -> usize {
        self.enabled_skills.len()
    }
}

impl Default for SkillsConfig {
    /// Creates a default configuration with sensible defaults.
    ///
    /// - Repository: https://github.com/alirezarezvani/claude-skills.git
    /// - Max skills: 5
    /// - Confidence threshold: 0.5
    /// - Cache TTL: 24 hours
    fn default() -> Self {
        Self {
            repository_url: "https://github.com/alirezarezvani/claude-skills.git".to_string(),
            max_skills: 5,
            confidence_threshold: 0.5,
            cache_ttl: Duration::from_secs(86400), // 24 hours
            enabled_skills: Vec::new(),
        }
    }
}

impl Default for SkillContext {
    fn default() -> Self {
        Self {
            workspace_path: PathBuf::new(),
            query: String::new(),
            project_type: String::new(),
            files: Vec::new(),
        }
    }
}
