//! Skills Manager - Central orchestrator for the skills system
//!
//! The SkillsManager coordinates all skills operations including discovery,
//! caching, selection, and orchestration. It follows Claude Code's plugin
//! architecture pattern with automatic skill discovery and context-aware selection.

use super::models::{Skill, SkillsConfig, SkillSelectionResult, SkillContext};
use super::discovery::SkillsDiscoveryEngine;
use super::selector::SkillSelector;
use super::cache::CacheManager;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Central orchestrator for skills management
///
/// Manages the complete lifecycle of skills including:
/// - Discovery from repositories
/// - Caching for performance
/// - Selection based on queries
/// - Conflict resolution
/// - Multi-agent orchestration
#[allow(dead_code)]
pub struct SkillsManager {
    /// In-memory cache of discovered skills
    skills: Arc<DashMap<String, Skill>>,
    /// Cache manager for persistent storage
    cache_manager: Arc<CacheManager>,
    /// Discovery engine for finding skills
    discovery_engine: Arc<SkillsDiscoveryEngine>,
    /// Skill selector for intelligent selection
    selector: Arc<SkillSelector>,
    /// Configuration for the skills system
    config: Arc<RwLock<SkillsConfig>>,
    /// Cache directory path
    cache_dir: PathBuf,
}

impl SkillsManager {
    /// Creates a new SkillsManager with default configuration
    ///
    /// Uses default repository URL and cache directory (~/.whizcode/skills/cache/)
    pub async fn new() -> Result<Self, String> {
        let config = SkillsConfig::default();
        Self::with_config(config).await
    }

    /// Creates a new SkillsManager with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Custom SkillsConfig with repository URL and preferences
    ///
    /// # Returns
    ///
    /// `Ok(SkillsManager)` if initialization succeeds, `Err(String)` otherwise
    pub async fn with_config(config: SkillsConfig) -> Result<Self, String> {
        // Determine cache directory
        let cache_dir = Self::get_cache_dir()?;

        // Initialize cache manager
        let cache_manager = Arc::new(CacheManager::new(cache_dir.clone()));
        cache_manager.initialize_cache_dir()?;

        // Initialize discovery engine
        let discovery_engine = Arc::new(SkillsDiscoveryEngine::new(config.repository_url.clone()));

        // Initialize skill selector
        let selector = Arc::new(SkillSelector::new(
            config.max_skills,
            config.confidence_threshold,
        ));

        Ok(Self {
            skills: Arc::new(DashMap::new()),
            cache_manager,
            discovery_engine,
            selector,
            config: Arc::new(RwLock::new(config)),
            cache_dir,
        })
    }

    /// Gets the cache directory path (~/.whizcode/skills/cache/)
    fn get_cache_dir() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
        Ok(home.join(".whizcode").join("skills").join("cache"))
    }

    /// Discovers skills from the configured repository
    ///
    /// Orchestrates the complete discovery process:
    /// 1. Fetches repository contents
    /// 2. Parses and validates manifests
    /// 3. Checks dependencies
    /// 4. Caches results
    /// 5. Returns discovered skills
    ///
    /// # Performance
    ///
    /// Completes in < 500ms for typical repositories
    ///
    /// # Returns
    ///
    /// `Ok(Vec<Skill>)` with all discovered skills
    /// `Err(String)` if discovery fails
    pub async fn discover_skills(&self) -> Result<Vec<Skill>, String> {
        tracing::info!("Starting skill discovery");

        // Discover skills from repository
        let discovered_skills = self.discovery_engine.discover_skills().await?;

        // Store in memory cache
        for skill in &discovered_skills {
            self.skills.insert(skill.name().to_string(), skill.clone());
        }

        tracing::info!("Discovered {} skills", discovered_skills.len());
        Ok(discovered_skills)
    }

    /// Selects the most relevant skills for a given query
    ///
    /// Uses intelligent relevance scoring to select skills that match the query.
    /// Implements Claude Code's context-aware skill selection pattern.
    ///
    /// # Arguments
    ///
    /// * `query` - User's query or request
    /// * `context` - SkillContext with workspace information
    ///
    /// # Returns
    ///
    /// `Ok(SkillSelectionResult)` with selected skills and conflict resolutions
    /// `Err(String)` if selection fails
    pub async fn select_skills(
        &self,
        query: &str,
        context: &SkillContext,
    ) -> Result<SkillSelectionResult, String> {
        tracing::info!("Selecting skills for query: {}", query);

        // Get all available skills from memory cache
        let available_skills: Vec<Skill> = self
            .skills
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        if available_skills.is_empty() {
            tracing::warn!("No skills available for selection");
            return Ok(SkillSelectionResult::new());
        }

        // Use selector to find relevant skills
        let result = self
            .selector
            .select_skills(query, context, &available_skills)
            .await?;

        tracing::info!(
            "Selected {} skills for query",
            result.selected_skills.len()
        );

        Ok(result)
    }

    /// Gets all discovered skills
    ///
    /// # Returns
    ///
    /// `Vec<Skill>` with all skills in the system
    pub fn get_all_skills(&self) -> Vec<Skill> {
        self.skills
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Gets a specific skill by name
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    ///
    /// # Returns
    ///
    /// `Some(Skill)` if found, `None` otherwise
    pub fn get_skill(&self, name: &str) -> Option<Skill> {
        self.skills.get(name).map(|entry| entry.value().clone())
    }

    /// Enables a skill
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(String)` if skill not found
    pub fn enable_skill(&self, name: &str) -> Result<(), String> {
        if let Some(mut skill) = self.skills.get_mut(name) {
            skill.enable();
            Ok(())
        } else {
            Err(format!("Skill '{}' not found", name))
        }
    }

    /// Disables a skill
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(String)` if skill not found
    pub fn disable_skill(&self, name: &str) -> Result<(), String> {
        if let Some(mut skill) = self.skills.get_mut(name) {
            skill.disable();
            Ok(())
        } else {
            Err(format!("Skill '{}' not found", name))
        }
    }

    /// Refreshes skills from the repository
    ///
    /// Re-discovers skills and updates the cache
    ///
    /// # Returns
    ///
    /// `Ok(Vec<Skill>)` with refreshed skills
    /// `Err(String)` if refresh fails
    pub async fn refresh_skills(&self) -> Result<Vec<Skill>, String> {
        tracing::info!("Refreshing skills from repository");
        self.skills.clear();
        self.discover_skills().await
    }

    /// Updates the repository URL
    ///
    /// # Arguments
    ///
    /// * `url` - New repository URL
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful
    pub async fn set_repository_url(&self, url: String) -> Result<(), String> {
        let mut config = self.config.write().await;
        config.repository_url = url;
        Ok(())
    }

    /// Gets the current configuration
    ///
    /// # Returns
    ///
    /// `SkillsConfig` with current settings
    pub async fn get_config(&self) -> SkillsConfig {
        self.config.read().await.clone()
    }

    /// Gets the cache manager
    #[allow(dead_code)]
    pub fn cache_manager(&self) -> Arc<CacheManager> {
        self.cache_manager.clone()
    }

    /// Gets the number of cached skills
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skills_manager_creation() {
        let manager = SkillsManager::new().await;
        assert!(manager.is_ok(), "Should create SkillsManager successfully");
    }

    #[tokio::test]
    async fn test_skills_manager_with_custom_config() {
        let config = SkillsConfig::default();
        let manager = SkillsManager::with_config(config).await;
        assert!(manager.is_ok(), "Should create SkillsManager with custom config");
    }

    #[tokio::test]
    async fn test_get_all_skills_empty() {
        let manager = SkillsManager::new().await.unwrap();
        let skills = manager.get_all_skills();
        assert!(skills.is_empty(), "Should return empty list initially");
    }

    #[tokio::test]
    async fn test_skill_count() {
        let manager = SkillsManager::new().await.unwrap();
        assert_eq!(manager.skill_count(), 0, "Should have 0 skills initially");
    }
}
